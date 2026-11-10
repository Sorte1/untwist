use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{
    finish_with, holds_a_bundle_with, unpack_program_taking_with, unpack_program_with, Bundle,
    BundleKind, UnpackOptions,
};

type Files = Vec<(String, String)>;

const FIXTURES: &str = "../../fixtures/unpack-bundlers/rollup";

const ON: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    conservative: false,
    trust_free_loader: false,
};

fn files(bundle: Option<Bundle<'_>>, alloc: &Allocator) -> Option<Files> {
    let mut bundle = bundle?;
    assert_eq!(bundle.kind, BundleKind::Rollup);
    finish_with(alloc, &mut bundle, &[], ON);
    Some(
        bundle
            .modules
            .values()
            .map(|module| {
                (
                    module.path.as_str().to_string(),
                    pretty_codegen(module.program()),
                )
            })
            .collect(),
    )
}

fn split(source: &str) -> Option<Files> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    files(unpack_program_with(&alloc, &parsed.program, ON), &alloc)
}

fn split_taken(source: &str) -> (Option<Files>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = files(
        unpack_program_taking_with(&alloc, &mut parsed.program, ON),
        &alloc,
    );
    (found, pretty_codegen(&parsed.program))
}

fn code<'f>(files: &'f Files, path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, code)| code.as_str())
        .unwrap_or_else(|| panic!("no {path} in {:?}", paths(files)))
}

fn paths(files: &Files) -> Vec<&str> {
    files.iter().map(|(path, _)| path.as_str()).collect()
}

fn refused(source: &str) {
    assert_eq!(split(source), None, "split:\n{source}");
    let alloc = Allocator::default();
    let before = pretty_codegen(&parse_program(&alloc, source).program);
    let (taken, after) = split_taken(source);
    assert_eq!(taken, None);
    assert_eq!(after, before, "the refused input is left intact");
}

fn iife(body: &str) -> String {
    format!("(function () {{\n  \"use strict\";\n{body}\n}})();\n")
}

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURES)
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

const TWO_MODULES: &str = "function add(a, b) { return a + b; }
const base = 10;
function addBase(n) { return add(n, base); }
function shout(s) { return s.toUpperCase(); }
const greeting = \"hi\";
function greet() { return shout(greeting); }
function mul(a, b) { return a * b; }
const two = 2;
function double(n) { return mul(n, two); }
console.log(\"entry\", addBase(1), greet(), double(3));";

#[test]
fn two_clusters_become_two_chunks_and_an_entry() {
    let files = split(&iife(TWO_MODULES)).expect("a split");
    assert_eq!(
        paths(&files),
        ["chunk_add.js", "chunk_shout.js", "chunk_mul.js", "index.js"]
    );
    let add = code(&files, "chunk_add.js");
    assert!(add.contains("export { addBase };"), "{add}");
    let index = code(&files, "index.js");
    assert!(
        index.contains("import { addBase } from \"./chunk_add.js\";")
            && index.contains("import { greet } from \"./chunk_shout.js\";"),
        "{index}"
    );
    assert!(index.contains("\"use strict\";"), "{index}");
}

#[test]
fn the_fixtures_split_alike_taken_and_cloned() {
    for name in ["min-rollup-hoisted.js", "rollup-iife.js"] {
        let source = fixture(name);
        let cloned = split(&source).unwrap_or_else(|| panic!("{name} splits"));
        let (taken, rest) = split_taken(&source);
        assert_eq!(taken, Some(cloned), "{name}");
        assert!(
            rest.contains("(function()") || rest.contains("(function ()"),
            "{rest}"
        );
        assert!(
            !rest.contains("console.log"),
            "{name}: the wrapper body was taken"
        );
    }
}

#[test]
fn a_write_across_clusters_keeps_writer_and_owner_in_one_module() {
    let source = fixture("neg-rollup-cross-write.js");
    let files = split(&source).expect("a split");
    let holder = files
        .iter()
        .find(|(_, code)| code.contains("count += 1"))
        .map(|(_, code)| code.as_str())
        .expect("a module writes count");
    assert!(holder.contains("let count = 0"), "{files:?}");
    assert!(
        files
            .iter()
            .all(|(_, code)| !code.contains("import { count")),
        "{files:?}"
    );
    assert_eq!(split_taken(&source).0, Some(files));
}

#[test]
fn a_forward_reference_across_clusters_joins_them() {
    let source = iife(
        "function a1() { return b2(); }\nfunction a2() { return a1(); }\n\
         function f1() { return 3; }\nfunction f2() { return f1(); }\nconsole.log(f2());\n\
         function b1() { return 2; }\nfunction b2() { return b1(); }\nconsole.log(\"entry\", a2());",
    );
    let files = split(&source).expect("a split");
    let holder = files
        .iter()
        .find(|(_, code)| code.contains("function a1"))
        .map(|(_, code)| code.as_str())
        .expect("a1 is declared");
    assert!(holder.contains("function b2"), "{files:?}");
}

#[test]
fn wrapper_arguments_keep_their_reader_in_the_entry() {
    let body = TWO_MODULES.replace("const two = 2;", "const two = arguments.length + 2;");
    let files = split(&iife(&body)).expect("a split");
    assert!(
        code(&files, "index.js").contains("arguments.length"),
        "{files:?}"
    );
}

#[test]
fn a_forward_reference_joins_its_items_into_one_module() {
    let files = split(&fixture("min-rollup-forward.js")).expect("a split");
    let wrap = code(&files, "chunk_log.js");
    assert!(
        wrap.contains("function wrap") && wrap.contains("function later"),
        "{wrap}"
    );
}

#[test]
fn top_level_this_stays_in_the_entry() {
    let files = split(&fixture("min-rollup-forward.js")).expect("a split");
    let index = code(&files, "index.js");
    assert!(index.contains("kind(this)"), "{index}");
}

#[test]
fn a_host_require_gets_a_create_require_prologue() {
    let files = split(&fixture("min-rollup-forward.js")).expect("a split");
    let holder = files
        .iter()
        .find(|(_, code)| code.contains("require(\"path\")"))
        .map(|(_, code)| code.as_str())
        .expect("a module requires path");
    assert!(
        holder.contains("import { createRequire } from \"node:module\";")
            && holder.contains("const require = createRequire(import.meta.url);"),
        "{holder}"
    );
}

#[test]
fn the_entry_imports_every_chunk_in_source_order() {
    let source = iife(
        "function a1() { return 1; }\nfunction a2() { return a1(); }\nconsole.log(\"a\", a2());\n\
         function b1() { return 2; }\nfunction b2() { return b1(); }\nconsole.log(\"b\", b2());\n\
         function c1() { return 3; }\nfunction c2() { return c1(); }\nconsole.log(\"c\", c2());\n\
         console.log(\"entry\");",
    );
    let files = split(&source).expect("a split");
    let index = code(&files, "index.js");
    let a = index.find("import \"./chunk_a1.js\";").expect(index);
    let b = index.find("import \"./chunk_b1.js\";").expect(index);
    let c = index.find("import \"./chunk_c1.js\";").expect(index);
    assert!(a < b && b < c, "{index}");
}

#[test]
fn a_return_at_wrapper_level_keeps_every_later_item_in_the_entry() {
    let source = iife(
        "function a1() { return 1; }\nfunction a2() { return a1(); }\n\
         function b1() { return 2; }\nfunction b2() { return b1(); }\n\
         if (globalThis.stop) { return; }\n\
         function c1() { return 3; }\nfunction c2() { return c1(); }\nconsole.log(c2());",
    );
    let files = split(&source).expect("a split");
    let index = code(&files, "index.js");
    assert!(
        index.contains("return;") && index.contains("function c2"),
        "{index}"
    );
}

#[test]
fn items_after_a_wrapper_level_return_stay_behind_it() {
    let source = iife(
        "function a1() { return 1; }\nfunction a2() { return a1(); }\n\
         if (globalThis.stop) { return; }\n\
         var c1 = console.log(\"c\");\nfunction c2() { return c1; }\nconsole.log(a2());",
    );
    let files = split(&source).expect("a split");
    let index = code(&files, "index.js");
    assert!(index.contains("console.log(\"c\")"), "{index}");
}

#[test]
fn a_top_level_declaration_outside_the_wrapper_refuses() {
    let body = iife(TWO_MODULES);
    for outer in [
        "var shell;",
        "var a, b;",
        "function helper() {}",
        "let late;",
        "class Outer {}",
    ] {
        refused(&format!("{outer}\n{body}"));
        refused(&format!("{body}\n{outer}"));
    }
    assert!(split(&format!(";\n{body}")).is_some());
}

#[test]
fn a_document_current_script_read_refuses() {
    for read in [
        "const script = document.currentScript;",
        "const script = document[\"currentScript\"];",
        "const script = typeof document != \"undefined\" ? document.currentScript : null;",
    ] {
        refused(&iife(&format!("{read}\n{TWO_MODULES}")));
        refused(&iife(&format!("{TWO_MODULES}\n{read}")));
    }
}

#[test]
fn a_var_declared_in_two_items_stays_in_one_module() {
    let source = iife(
        "var v = 1;\nfunction a1() { return v; }\nfunction a2() { return a1(); }\n\
         function b1() { return 2; }\nfunction b2() { return b1(); }\nvar v;\n\
         function c1() { return 3; }\nfunction c2() { return c1(); }\nconsole.log(a2(), b2(), c2());",
    );
    let files = split(&source).expect("a split");
    let holder = files
        .iter()
        .find(|(_, code)| code.contains("var v = 1"))
        .map(|(_, code)| code.as_str())
        .expect("v is declared");
    assert!(holder.contains("var v;"), "{files:?}");
}

#[test]
fn chunk_ids_are_unique_ignoring_case() {
    let source = iife(
        "function a() { return 1; }\nfunction a1() { return a(); }\nconsole.log(a1());\n\
         function A() { return 2; }\nfunction A1() { return A(); }\nconsole.log(A1());\n\
         console.log(\"entry\");",
    );
    let files = split(&source).expect("a split");
    assert_eq!(paths(&files), ["chunk_a.js", "chunk_A-2.js", "index.js"]);
}

#[test]
fn the_flag_off_finds_nothing() {
    let source = iife(TWO_MODULES);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let off = UnpackOptions::default();
    assert!(unpack_program_with(&alloc, &parsed.program, off).is_none());
    assert!(!holds_a_bundle_with(&parsed.program, off));
    assert!(holds_a_bundle_with(&parsed.program, ON));
}

#[test]
fn a_sloppy_wrapper_refuses() {
    refused(&fixture("neg-rollup-sloppy.js"));
}

#[test]
fn a_free_host_module_name_refuses() {
    refused(&fixture("neg-rollup-host-module.js"));
    for name in ["exports", "__filename", "__dirname"] {
        refused(&iife(&format!(
            "{TWO_MODULES}\nconsole.log(typeof {name});"
        )));
    }
}

#[test]
fn one_module_refuses() {
    refused(&fixture("neg-rollup-one-module.js"));
}

#[test]
fn require_other_than_a_bare_call_refuses() {
    for tail in [
        "console.log(typeof require);",
        "console.log(require(\"./local\"));",
        "console.log(require(\"/abs\"));",
        "console.log(require(\"a\", 1));",
        "console.log(require?.(\"a\"));",
    ] {
        refused(&iife(&format!("{TWO_MODULES}\n{tail}")));
    }
}

#[test]
fn direct_eval_and_import_meta_refuse() {
    refused(&iife(&format!("{TWO_MODULES}\neval(\"1\");")));
    refused(&iife(&format!(
        "{TWO_MODULES}\nconsole.log(import.meta.url);"
    )));
}

#[test]
fn an_arrow_wrapper_reading_this_or_arguments_refuses() {
    let arrow = |body: &str| format!("(() => {{\n  \"use strict\";\n{body}\n}})();\n");
    assert!(split(&arrow(TWO_MODULES)).is_some());
    refused(&arrow(&format!("{TWO_MODULES}\nconsole.log(this);")));
    refused(&arrow(&format!(
        "{TWO_MODULES}\nconsole.log(arguments.length);"
    )));
}

#[test]
fn the_wrapper_shape_is_strict() {
    let body = TWO_MODULES;
    for source in [
        format!("(function (x) {{ \"use strict\";\n{body} }})();"),
        format!("(function () {{ \"use strict\";\n{body} }})(1);"),
        format!("(async function () {{ \"use strict\";\n{body} }})();"),
        format!("(function* () {{ \"use strict\";\n{body} }})().next();"),
        format!("var lib = (function () {{ \"use strict\";\n{body} }})();"),
        format!("var a = 1;\n(function () {{ \"use strict\";\n{body} }})();"),
        format!("console.log(1);\n(function () {{ \"use strict\";\n{body} }})();"),
        format!("(function () {{ \"use strict\";\n{body} }}).call(this);"),
        format!("import \"x\";\n(function () {{ \"use strict\";\n{body} }})();"),
    ] {
        refused(&source);
    }
    assert!(split(&format!("\"use strict\";\n(function () {{\n{body} }})();")).is_some());
    assert!(split(&format!("!function () {{ \"use strict\";\n{body} }}();")).is_some());
}

#[test]
fn with_the_flag_every_other_kind_keeps_its_own_detector() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-bundlers");
    let manifest = std::fs::read_to_string(root.join("MANIFEST")).expect("MANIFEST");
    let mut wrong = Vec::new();
    for line in manifest
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let path = line.split('\t').next().unwrap_or_default();
        let source = std::fs::read_to_string(root.join(path)).expect("fixture");
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &source);
        let on = unpack_program_with(&alloc, &parsed.program, ON).map(|b| b.kind.as_str());
        let off = unpack_program_with(&alloc, &parsed.program, UnpackOptions::default())
            .map(|b| b.kind.as_str());
        let expected = match (off, path) {
            (
                None,
                "rollup/rollup-iife.js"
                | "rollup/min-rollup-hoisted.js"
                | "rollup/min-rollup-forward.js"
                | "rollup/neg-rollup-cross-write.js",
            ) => Some("rollup"),
            (other, _) => other,
        };
        if on != expected {
            wrong.push(format!("{path}: off {off:?}, on {on:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_await_identifier_refuses() {
    refused(&iife(&format!("var await = 1;\n{TWO_MODULES}")));
    refused(&iife(&format!("{TWO_MODULES}\nconsole.log(typeof await);")));
    refused(&iife(&format!(
        "{TWO_MODULES}\nawait: for (;;) {{ break await; }}"
    )));
}

#[test]
fn a_segment_with_one_declaration_folds_into_its_neighbour() {
    let source = iife(&format!(
        "console.log(f1());\nfunction f1() {{ return 1; }}\n{TWO_MODULES}"
    ));
    let files = split(&source).expect("a split");
    assert_eq!(
        paths(&files),
        ["chunk_f1.js", "chunk_shout.js", "chunk_mul.js", "index.js"]
    );
    assert!(
        code(&files, "chunk_f1.js").contains("function add"),
        "{files:?}"
    );
}

#[test]
fn this_inside_a_field_and_return_inside_an_arrow_do_not_pin() {
    let body = TWO_MODULES.replace(
        "const two = 2;",
        "const two = 2;\nclass Box { self = this; }\nconst one = () => { return 1; };",
    );
    let files = split(&iife(&body)).expect("a split");
    let index = code(&files, "index.js");
    assert!(
        !index.contains("class Box") && !index.contains("return 1"),
        "{index}"
    );
}

const SWEEP: &[&str] = &[
    "fixtures/behaviour",
    "fixtures/eval",
    "fixtures/unminify",
    "fixtures/deobfuscate-gaps",
    "fixtures/jsx",
    "fixtures/corpus-npm",
];

const SWEEP_SPLIT: &[&str] = &[];

fn js_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read sweep dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            js_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "js") {
            out.push(path);
        }
    }
}

#[test]
fn with_the_flag_the_sweep_splits_only_the_recorded_inputs() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    for dir in SWEEP {
        js_files(&repo.join(dir), &mut files);
    }
    files.sort();
    let mut found = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).expect("read sweep input");
        if split(&source).is_some() {
            let rel = file.strip_prefix(&repo).unwrap_or(file);
            found.push(rel.display().to_string());
        }
    }
    assert!(files.len() > 270, "sweep shrank to {}", files.len());
    assert_eq!(found, SWEEP_SPLIT);
}

const DECLARED_TWICE: &str = "function f() { return 1; }\nfunction f() { return 2; }\n\
     var a = f();\nfunction a2() { return a + f(); }\nfunction a3() { return a2() + a; }\n\
     var b = 10;\nfunction b2() { return b * 2; }\nfunction b3() { return b2() + b; }\n\
     var c = 5;\nfunction c2() { return c * 3; }\nfunction c3() { return c2() + c; }\n\
     console.log(a3(), b3(), c3());";

#[test]
fn a_function_declared_twice_keeps_only_its_last_declaration() {
    let source = iife(DECLARED_TWICE);

    let files = split(&source).expect("a split");

    let all: Vec<&str> = files.iter().map(|(_, code)| code.as_str()).collect();
    let all = all.join("\n");
    assert_eq!(all.matches("function f()").count(), 1, "{files:?}");
    assert!(
        all.contains("return 2;") && !all.contains("return 1;"),
        "{files:?}"
    );
    assert_eq!(split_taken(&source).0, Some(files));
}

#[test]
fn a_function_that_shares_its_name_with_another_kind_of_declaration_refuses_the_split() {
    let with_var = DECLARED_TWICE.replace("function f() { return 1; }", "var f;");
    let var_last = DECLARED_TWICE.replace(
        "function f() { return 2; }\n",
        "function f() { return 2; }\nvar f;\n",
    );
    assert_ne!(with_var, DECLARED_TWICE);
    assert_ne!(var_last, DECLARED_TWICE);
    refused(&iife(&with_var));
    refused(&iife(&var_last));
}
