mod common;

use common::cpu_or_wall;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_testing::{generated_bundle_present, js_files_in};
use untwist_unpack::{holds_a_bundle, unpack, unpack_taking, BundleKind, UnpackResult};

const HELPERS: &str = "var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
";

const DIRECT: &str = "var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
";

const SHIM: &str =
    "var __require = ((x) => typeof require !== \"undefined\" ? require : x)(function(x) {
  throw Error('Dynamic require of \"' + x + '\" is not supported');
});
";

const TO_ESM: &str = "var __create = Object.create;
var __defProp = Object.defineProperty;
var __getProtoOf = Object.getPrototypeOf;
var __toESM = (mod, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __defProp(target, \"default\", { value: mod, enumerable: true }));
";

fn files(result: Option<UnpackResult<'_>>) -> Option<Vec<(String, String)>> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Esbuild);
    Some(
        result
            .bundle
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

fn split(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    files(unpack(&alloc, &parsed.program, &[]))
}

fn split_taken(source: &str) -> (Option<Vec<(String, String)>>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = files(unpack_taking(&alloc, &mut parsed.program, &[]));
    (found, pretty_codegen(&parsed.program))
}

fn code<'f>(files: &'f [(String, String)], path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, code)| code.as_str())
        .unwrap_or_else(|| {
            panic!(
                "no {path} in {:?}",
                files.iter().map(|(p, _)| p).collect::<Vec<_>>()
            )
        })
}

fn paths(files: &[(String, String)]) -> Vec<&str> {
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

fn factory(key: &str, params: &str, body: &str) -> String {
    format!(
        "var require_{} = __commonJS({{\n  \"{key}\"({params}) {{\n{body}\n  }}\n}});\n",
        stem(key)
    )
}

fn stem(key: &str) -> String {
    key.rsplit('/')
        .next()
        .unwrap_or(key)
        .trim_end_matches(".js")
        .replace(['-', '.'], "_")
}

#[test]
fn a_commonjs_factory_becomes_a_module_and_its_calls_require_it() {
    let source = format!(
        "{HELPERS}{}console.log(require_a().value);\n",
        factory(
            "src/a.js",
            "exports, module",
            "module.exports = { value: 1 };"
        )
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(paths(&files), ["index.js", "src/a.js"]);
    assert_eq!(code(&files, "src/a.js"), "module.exports = { value: 1 };");
    assert!(code(&files, "index.js").contains("console.log(require(\"./src/a.js\").value);"));
    assert!(!code(&files, "index.js").contains("require_a"));
}

#[test]
fn a_keyless_factory_takes_its_path_from_the_comment_else_its_name() {
    let source = format!(
        "{DIRECT}// lib/b.js\nvar require_b = __commonJS((exports2) => {{ exports2.b = 2; }});\nvar require_c = __commonJS(function(e) {{ e.c = require_b().b; }});\nconsole.log(require_c().c);\n"
    );
    let files = split(&source).expect("a bun-style bundle");
    assert_eq!(paths(&files), ["index.js", "lib/b.js", "require_c.js"]);
    assert_eq!(code(&files, "lib/b.js"), "exports.b = 2;");
    assert_eq!(
        code(&files, "require_c.js"),
        "exports.c = require(\"./lib/b.js\").b;"
    );
}

#[test]
fn a_parameter_is_renamed_without_capturing_a_local_binding() {
    let source = format!(
        "{HELPERS}{}require_a();\n",
        factory("a.js", "e, m", "var module = 1;\nm.exports = module;")
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(
        code(&files, "a.js"),
        "var _module = 1;\nmodule.exports = _module;"
    );
    let half = source.replace("\"a.js\"(e, m)", "\"a.js\"(exports, m)");
    let files = split(&half).expect("an esbuild bundle");
    assert_eq!(
        code(&files, "a.js"),
        "var _module = 1;\nmodule.exports = _module;"
    );
}

#[test]
fn the_dynamic_require_shim_becomes_the_host_require_inside_a_module() {
    let source = format!(
        "{HELPERS}{SHIM}{}require_a();\n__require(\"fs\");\n",
        factory(
            "a.js",
            "exports",
            "exports.join = __require(\"path\").join;"
        )
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(
        code(&files, "a.js"),
        "exports.join = require(\"path\").join;"
    );
    assert!(code(&files, "index.js").contains("__require(\"fs\");"));
}

#[test]
fn helpers_a_factory_uses_move_to_the_runtime_module() {
    let source = format!(
        "{HELPERS}{TO_ESM}{}{}var b = __toESM(require_b());\nrequire_a();\n",
        factory("b.js", "exports", "exports.b = 1;"),
        factory(
            "a.js",
            "exports",
            "exports.load = () => __toESM(require_b());"
        )
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(
        paths(&files),
        ["index.js", "_runtime/esbuild.js", "b.js", "a.js"]
    );
    let runtime = code(&files, "_runtime/esbuild.js");
    assert!(runtime.contains("var __toESM ="), "{runtime}");
    assert!(
        runtime.contains(
            "module.exports = {\n  __create,\n  __defProp,\n  __getProtoOf,\n  __toESM\n};"
        ),
        "{runtime}"
    );
    assert!(
        code(&files, "a.js").starts_with("var { __toESM } = require(\"./_runtime/esbuild.js\");\n")
    );
    let entry = code(&files, "index.js");
    assert!(entry.starts_with("var { __create, __defProp, __getProtoOf, __toESM } = require(\"./_runtime/esbuild.js\");\nvar __getOwnPropNames"), "{entry}");
    assert!(!entry.contains("var __toESM ="), "{entry}");
}

#[test]
fn an_entry_write_to_a_moved_helper_refuses() {
    let source = format!(
        "{HELPERS}{TO_ESM}{}require_a();\n__toESM = null;\n",
        factory("a.js", "exports", "exports.load = () => __toESM({});")
    );
    refused(&source);
}

#[test]
fn a_moved_helper_that_reads_the_entry_refuses() {
    let source = format!(
        "{HELPERS}var __helper = () => late;\n{}var late = 1;\nrequire_a();\n",
        factory("a.js", "exports", "exports.v = __helper();")
    );
    refused(&source);
}

#[test]
fn an_esm_entry_gets_a_created_require() {
    let source = format!(
        "import fs from \"fs\";\n{HELPERS}{}console.log(require_a(), fs);\n",
        factory("a.js", "exports", "exports.a = 1;")
    );
    let files = split(&source).expect("an esbuild bundle");
    let entry = code(&files, "index.js");
    assert!(entry.starts_with("import fs from \"fs\";\nimport { createRequire } from \"module\";\nconst require = createRequire(import.meta.url);\n"), "{entry}");
    assert!(
        entry.contains("console.log(require(\"./a.js\"), fs);"),
        "{entry}"
    );
    assert_eq!(code(&files, "a.js"), "\"use strict\";\nexports.a = 1;");
}

#[test]
fn an_esm_entry_that_reads_require_gets_a_fresh_name() {
    let source = format!(
        "{HELPERS}{}console.log(require_a());\nexport const has = typeof require;\n",
        factory("a.js", "exports", "exports.a = 1;")
    );
    let files = split(&source).expect("an esbuild bundle");
    let entry = code(&files, "index.js");
    assert!(
        entry.contains("const require2 = createRequire(import.meta.url);"),
        "{entry}"
    );
    assert!(
        entry.contains("console.log(require2(\"./a.js\"));"),
        "{entry}"
    );
    assert!(entry.contains("typeof require;"), "{entry}");
}

#[test]
fn an_arrow_iife_is_unwrapped_into_the_entry() {
    let source = format!(
        "(() => {{\n\"use strict\";\n{HELPERS}{}console.log(require_a().a);\n}})();\n",
        factory("a.js", "exports", "exports.a = 1;")
    );
    let files = split(&source).expect("an esbuild bundle");
    let entry = code(&files, "index.js");
    assert!(
        entry.starts_with("\"use strict\";\nvar __getOwnPropNames"),
        "{entry}"
    );
    assert_eq!(code(&files, "a.js"), "\"use strict\";\nexports.a = 1;");
    let function = source
        .replace("(() => {", "(function () {")
        .replace("})();", "}).call(this);");
    refused(&function);
}

#[test]
fn names_that_differ_only_in_case_get_distinct_paths() {
    let source = format!(
        "{DIRECT}var b = __commonJS((e) => {{ e.v = 1; }});\nvar B = __commonJS((e) => {{ e.v = 2; }});\nconsole.log(b().v, B().v);\n"
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(paths(&files), ["index.js", "b.js", "B-2.js"]);
    assert!(code(&files, "index.js")
        .contains("console.log(require(\"./b.js\").v, require(\"./B-2.js\").v);"));
}

#[test]
fn a_local_binding_with_a_factory_name_keeps_its_own_calls() {
    let source = format!(
        "{HELPERS}{}{}require_a();\n",
        factory("b.js", "exports", "exports.b = 1;"),
        factory(
            "a.js",
            "exports",
            "exports.f = function (require_b) { return require_b(); };\nexports.b = require_b();"
        )
    );
    let files = split(&source).expect("an esbuild bundle");
    assert_eq!(
        code(&files, "a.js"),
        "exports.f = function(require_b) {\n  return require_b();\n};\nexports.b = require(\"./b.js\");"
    );
}

#[test]
fn a_factory_that_reads_a_bundle_binding_refuses() {
    let source = format!(
        "{HELPERS}var shared = 1;\n{}require_a();\n",
        factory("a.js", "exports", "exports.v = shared;")
    );
    refused(&source);
    let global = source.replace("exports.v = shared;", "exports.v = Math.max(1, 2);");
    assert!(split(&global).is_some());
}

#[test]
fn a_require_binding_used_as_a_value_refuses() {
    let base = format!(
        "{HELPERS}{}{}",
        factory("b.js", "exports", "exports.b = 1;"),
        factory("a.js", "exports", "exports.get = require_b;")
    );
    refused(&format!("{base}require_a();\n"));
    refused(&format!(
        "{}require_a();\n",
        base.replace("exports.get = require_b;", "exports.get = require_b(1);")
    ));
    let in_entry = base.replace("exports.get = require_b;", "exports.a = 1;");
    refused(&format!("{in_entry}setTimeout(require_b);\n"));
    refused(&format!("{in_entry}require_b(1);\n"));
}

#[test]
fn an_entry_call_before_the_declaration_counts_only_when_deferred() {
    let tail = format!(
        "{}require_b();\n",
        factory("b.js", "exports", "exports.b = 1;")
    );
    for early in [
        "var later = () => require_b();\n",
        "function later() { return require_b(); }\n",
    ] {
        assert!(
            split(&format!("{HELPERS}{early}{tail}")).is_some(),
            "{early}"
        );
    }
    refused(&format!("{HELPERS}require_b();\n{tail}"));
}

#[test]
fn top_level_this_in_a_factory_refuses() {
    let base = format!(
        "{HELPERS}{}require_a();\n",
        factory("a.js", "exports", "exports.self = this;")
    );
    refused(&base);
    let nested = base.replace(
        "exports.self = this;",
        "exports.f = function () { return this; };",
    );
    assert!(split(&nested).is_some());
}

#[test]
fn top_level_this_in_a_class_key_or_heritage_refuses() {
    for body in ["class A extends f(this) {}", "class A { [this.k]() {} }"] {
        refused(&format!(
            "{HELPERS}{}require_a();\n",
            factory("a.js", "exports", body)
        ));
    }
    let members = format!(
        "{HELPERS}{}require_a();\n",
        factory(
            "a.js",
            "exports",
            "exports.C = class { x = this; static { f(this); } };"
        )
    );
    assert!(split(&members).is_some());
}

#[test]
fn the_factory_arguments_refuse() {
    let source = format!(
        "{HELPERS}{}require_a();\n",
        factory("a.js", "exports", "exports.n = arguments.length;")
    );
    refused(&source);
    let arrow = source.replace(
        "exports.n = arguments.length;",
        "exports.f = () => arguments.length;",
    );
    refused(&arrow);
}

#[test]
fn the_module_parameter_outside_exports_and_typeof_refuses() {
    let base = format!(
        "{HELPERS}{}require_a();\n",
        factory("a.js", "exports, module", "module.exports = typeof module;")
    );
    assert!(split(&base).is_some());
    refused(&base.replace(
        "module.exports = typeof module;",
        "module.exports = module.id;",
    ));
    refused(&base.replace("module.exports = typeof module;", "keep(module);"));
}

#[test]
fn a_host_name_read_free_in_a_factory_refuses() {
    for read in ["__dirname", "__filename", "module.exports", "exports"] {
        let source = format!(
            "{HELPERS}{}require_a();\n",
            factory("a.js", "", &format!("console.log({read});"))
        );
        refused(&source);
    }
}

#[test]
fn a_host_name_read_in_the_entry_refuses() {
    let body = factory("a.js", "exports", "exports.a = 1;");
    let entry = |read: &str| format!("{HELPERS}{SHIM}{body}console.log(require_a(), {read});\n");
    for read in [
        "__dirname",
        "__filename",
        "module.filename",
        "module",
        "exports.a",
        "require.resolve(\"path\")",
        "require(\"./x.js\")",
        "require(name)",
        "__require(\"./x.js\")",
        "__require.cache",
    ] {
        refused(&entry(read));
    }
    for read in [
        "require(\"path\")",
        "__require(\"path\")",
        "typeof module",
        "typeof __filename",
        "module.exports = 1",
    ] {
        assert!(split(&entry(read)).is_some(), "{read}");
    }
    let bun = format!(
        "// @bun @bun-cjs\n(function(exports, require, module, __filename, __dirname) {{{DIRECT}var require_a = __commonJS((e, m) => {{ m.exports = 1; }});\nconsole.log(require_a(), READ);\n}})\n"
    );
    assert!(split(&bun.replace("READ", "require(\"path\")")).is_some());
    for read in ["__filename", "module.id", "require(\"./x.js\")"] {
        refused(&bun.replace("READ", read));
    }
}

#[test]
fn a_host_require_needs_one_bare_literal() {
    let base = format!(
        "{HELPERS}{SHIM}{}require_a();\n",
        factory("a.js", "exports", "exports.p = require(\"path\");")
    );
    assert!(split(&base).is_some());
    for call in [
        "require(\"./x.js\")",
        "require(name)",
        "require.resolve(\"path\")",
        "__require(\"/abs\")",
        "__require(name)",
    ] {
        refused(&base.replace("require(\"path\")", call));
    }
}

#[test]
fn an_entry_call_before_the_factory_is_declared_refuses() {
    let body = factory("a.js", "exports", "exports.a = 1;");
    refused(&format!("{HELPERS}console.log(require_a());\n{body}"));
    let deferred = format!("{HELPERS}function later() {{ return require_a(); }}\n{body}later();\n");
    assert!(split(&deferred).is_some());
}

#[test]
fn a_bun_factory_that_reads_the_wrapper_module_refuses_and_restores() {
    let ok = format!(
        "// @bun @bun-cjs\n(function(exports, require, module, __filename, __dirname) {{{DIRECT}var require_a = __commonJS((e, m) => {{ m.exports = require(\"path\").sep; }});\nconsole.log(require_a());\n}})\n"
    );
    let files = split(&ok).expect("a bun bundle");
    refused(&ok.replacen("// @bun @bun-cjs\n", "", 1));
    refused(&ok.replacen("// @bun @bun-cjs\n", "// @bun\n", 1));
    assert_eq!(
        code(&files, "require_a.js"),
        "module.exports = require(\"path\").sep;"
    );
    refused(&ok.replace(
        "m.exports = require(\"path\").sep;",
        "m.exports = module.id;",
    ));
}

#[test]
fn an_unproven_lazy_helper_never_splits() {
    let source = "var __commonJS = (cb, mod) => () => (cb((mod = { exports: {} }).exports, mod), mod.exports);\nvar require_a = __commonJS((e) => { e.a = 1; });\nconsole.log(require_a());\n";
    refused(source);
    let alloc = Allocator::default();
    assert!(!holds_a_bundle(&parse_program(&alloc, source).program));
}

#[test]
fn a_shadowed_require_at_a_rewritten_call_refuses() {
    let source = format!(
        "{HELPERS}{}{}require_a();\n",
        factory("b.js", "exports", "exports.b = 1;"),
        factory(
            "a.js",
            "exports",
            "var require = null;\nexports.b = require_b();"
        )
    );
    refused(&source);
}

#[test]
fn direct_eval_refuses() {
    let body = factory("a.js", "exports", "exports.a = eval(\"1\");");
    refused(&format!("{HELPERS}{body}require_a();\n"));
    let entry = factory("a.js", "exports", "exports.a = 1;");
    refused(&format!("{HELPERS}{entry}eval(\"require_a()\");\n"));
}

#[test]
fn the_factory_shape_must_match_the_helper_call() {
    let keyed =
        format!("{HELPERS}var require_a = __commonJS((e) => {{ e.a = 1; }});\nrequire_a();\n");
    refused(&keyed);
    let direct = format!(
        "{DIRECT}{}require_a();\n",
        factory("a.js", "exports", "exports.a = 1;")
    );
    refused(&direct);
}

#[test]
fn the_first_key_getter_must_be_object_get_own_property_names() {
    let source = format!(
        "{HELPERS}{}require_a();\n",
        factory("a.js", "exports", "exports.a = 1;")
    );
    assert!(split(&source).is_some());
    refused(&source.replace("Object.getOwnPropertyNames", "Object.keys"));
}

#[test]
fn a_redeclared_factory_or_moved_helper_refuses() {
    let body = factory("a.js", "exports", "exports.a = 1;");
    assert!(split(&format!("{HELPERS}{body}require_a();\n")).is_some());
    refused(&format!(
        "{HELPERS}function require_a() {{ return 0; }}\n{body}require_a();\n"
    ));
    let helper = format!(
        "{HELPERS}{TO_ESM}{}require_a();\n",
        factory("a.js", "exports", "exports.load = () => __toESM({});")
    );
    assert!(split(&helper).is_some());
    refused(&format!("{helper}function __toESM() {{}}\n"));
}

fn bundler_fixtures() -> Vec<(String, String)> {
    [
        "fixtures/unpack-bundlers/esbuild",
        "fixtures/unpack-bundlers/bun",
    ]
    .into_iter()
    .flat_map(js_files_in)
    .filter(|(label, _)| !label.contains("neg-"))
    .collect()
}

#[test]
fn the_fixtures_extract_alike_taken_and_cloned_and_lose_their_factory_bodies() {
    let fixtures = bundler_fixtures();
    assert!(fixtures.len() >= 12, "{}", fixtures.len());
    for (label, source) in fixtures {
        let cloned = split(&source);
        let (taken, after) = split_taken(&source);
        assert!(cloned.is_some(), "{label}");
        assert_eq!(taken, cloned, "{label}");
        if source.contains("eval cycle-a start") {
            assert!(
                !after.contains("eval cycle-a start"),
                "{label}: a factory body was cloned"
            );
        }
    }
}

#[test]
fn the_generated_bundle_takes_every_factory_body() {
    if !generated_bundle_present() {
        return;
    }
    let (label, source) = js_files_in("fixtures/generated/kinds")
        .into_iter()
        .find(|(label, _)| label.ends_with("esbuild-5mb.js"))
        .expect("the generated esbuild bundle");
    let (taken, after) = split_taken(&source);
    let files = taken.unwrap_or_else(|| panic!("{label}"));
    assert!(files.len() > 1000, "{}", files.len());
    assert!(!after.contains("exports.k0 ="), "a factory body was cloned");
}

fn hostile_bundle(modules: usize) -> String {
    let separate = modules / 4;
    let mut source = String::new();
    for index in 0..modules {
        source.push_str(&format!("// head {index}\n"));
    }
    source.push_str(DIRECT);
    for index in 0..modules {
        source.push_str(&format!("var pure{index} = Object.keys;\n"));
    }
    for index in 0..separate {
        source.push_str(&format!(
            "// m{index}.js\nvar require_m{index} = __commonJS((exports, module) => {{ module.exports = pure{index} && {index}; }});\n"
        ));
    }
    let shared: Vec<String> = (separate..modules)
        .map(|index| {
            format!("require_m{index} = __commonJS((exports, module) => {{ module.exports = pure{index} && {index}; }})")
        })
        .collect();
    source.push_str(&format!("var {}, extra = 1;\n", shared.join(",\n  ")));
    source.push_str(&format!(
        "console.log(require_m0(), require_m{}(), extra);\n",
        modules - 1
    ));
    for index in 0..modules {
        source.push_str(&format!("// tail {index}\n"));
    }
    source
}

#[test]
fn a_hostile_hundred_thousand_module_bundle_splits_in_linear_time() {
    const MODULES: usize = 100_000;
    const LINEAR_FACTOR: u32 = 5;
    let source = hostile_bundle(MODULES);
    let alloc = Allocator::default();
    let (parsed, parse) = cpu_or_wall(|| parse_program(&alloc, &source));
    let (_, scope) = cpu_or_wall(|| untwist_core::semantic::build_scoping(&parsed.program));
    let (modules, spent) = cpu_or_wall(|| {
        unpack(&alloc, &parsed.program, &[]).map(|result| result.bundle.modules.len())
    });
    assert_eq!(modules, Some(MODULES + 2));
    let bound = (parse + scope) * LINEAR_FACTOR;
    assert!(
        spent < bound,
        "{MODULES} modules took {spent:?}, over {LINEAR_FACTOR} x parse {parse:?} + scoping {scope:?}"
    );
}

const ESM: &str = "var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);\n";

#[test]
fn a_runtime_binding_written_after_its_declaration_is_read_live() {
    let source = format!(
        "{DIRECT}{ESM}var value;\nvar init_value = __esm(() => {{ value = 42; }});\nvar require_main = __commonJS((exports, module) => {{ init_value(); module.exports = value; }});\nconsole.log(require_main(), value);\n"
    );

    let files = split(&source).expect("an esbuild bundle");

    let runtime = code(&files, "_runtime/esbuild.js");
    assert!(runtime.contains("get value() {"), "{runtime}");
    assert!(runtime.contains("init_value"), "{runtime}");
    let main = code(&files, "require_main.js");
    assert!(
        main.starts_with("var runtime = require(\"./_runtime/esbuild.js\");\nvar { init_value } = require(\"./_runtime/esbuild.js\");\n"),
        "{main}"
    );
    assert!(main.contains("module.exports = runtime.value;"), "{main}");
    let entry = code(&files, "index.js");
    assert!(
        entry.contains("console.log(require(\"./require_main.js\"), runtime.value);"),
        "{entry}"
    );
    assert!(!entry.contains("{ value"), "{entry}");
    let (taken, _) = split_taken(&source);
    assert_eq!(taken.as_ref(), Some(&files));
}

#[test]
fn a_live_runtime_binding_keeps_its_call_receiver_and_its_shorthand_key() {
    let source = format!(
        "{DIRECT}var greet;\nvar set = () => {{ greet = function () {{ return this; }}; }};\nvar require_main = __commonJS((exports, module) => {{ set(); var local = (greet) => greet; module.exports = {{ greet, called: greet(), tagged: greet`x`, kind: typeof greet, local }}; }});\nconsole.log(require_main());\n"
    );

    let files = split(&source).expect("an esbuild bundle");

    let main = code(&files, "require_main.js");
    assert!(main.contains("greet: runtime.greet,"), "{main}");
    assert!(main.contains("called: (0, runtime.greet)(),"), "{main}");
    assert!(main.contains("tagged: (0, runtime.greet)`x`,"), "{main}");
    assert!(main.contains("kind: typeof runtime.greet,"), "{main}");
    assert!(main.contains("var local = (greet) => greet;"), "{main}");
}

#[test]
fn a_live_runtime_binding_read_where_a_member_cannot_stand_is_refused() {
    for read in ["delete value;", "for (value in exports) {}", "value++;"] {
        refused(&format!(
            "{DIRECT}var value;\nvar set = () => {{ value = 1; }};\nvar require_main = __commonJS((exports, module) => {{ set(); {read} module.exports = value; }});\nconsole.log(require_main());\n"
        ));
    }
}

#[test]
fn a_lazy_helper_declared_twice_or_assigned_again_is_not_trusted() {
    let tail = "var require_a = __commonJS((exports, module) => { module.exports = \"a\"; });\nconsole.log(require_a());\n";
    for second in [
        "var __commonJS = (cb) => () => 42;\n",
        "var __commonJS = function (cb) { return 42; };\n",
        "__commonJS = (cb) => () => 42;\n",
        "function swap() { __commonJS = (cb) => () => 42; }\nswap();\n",
    ] {
        refused(&format!("{DIRECT}{second}{tail}"));
    }
}

#[test]
fn a_first_key_helper_whose_names_getter_changes_is_not_trusted() {
    for second in [
        "__getOwnPropNames = () => [\"other\"];\n",
        "var __getOwnPropNames = () => [\"other\"];\n",
    ] {
        refused(&format!(
            "{HELPERS}{second}{}console.log(require_a());\n",
            factory("src/a.js", "exports, module", "module.exports = 1;")
        ));
    }
}

#[test]
fn a_with_statement_in_a_factory_or_in_the_entry_is_refused() {
    refused(&format!(
        "{DIRECT}var require_a = __commonJS((e, m) => {{ var o = {{ e: {{ tag: \"inner\" }} }}; with (o) {{ e.x = 1; }} m.exports = Object.keys(e); }});\nconsole.log(require_a());\n"
    ));
    refused(&format!(
        "{DIRECT}var require_a = __commonJS((exports, module) => {{ module.exports = 1; }});\nwith ({{ require_a: () => 2 }}) {{ console.log(require_a()); }}\n"
    ));
}

#[test]
fn a_helper_that_reads_a_host_name_is_never_moved_into_the_runtime() {
    for read in [
        "require(\"./config.json\").name",
        "typeof module.exports.__esm",
        "exports.a",
        "__filename",
        "typeof __dirname",
    ] {
        refused(&format!(
            "{DIRECT}{ESM}var init_x = __esm(() => {{ console.log({read}); }});\nvar require_a = __commonJS((exports, module) => {{ init_x(); module.exports = 1; }});\nconsole.log(require_a());\n"
        ));
    }
}

fn refused_with_note(source: &str) {
    refused(source);
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert_eq!(
        untwist_unpack::detect::refused_kind(&parsed.program, Default::default()),
        Some(BundleKind::Esbuild),
        "no refusal note for:\n{source}"
    );
}

const GUARDED: &str = "var __commonJS = (cb, mod) => function __require() { try { return mod || cb((mod = { exports: {} }).exports, mod), mod.exports; } catch (e) { throw mod = 0, e; } };\n";

#[test]
fn a_helper_binding_that_shadows_a_name_the_helper_reads_refuses() {
    let direct = "var require_a = __commonJS((exports, module) => { module.exports = \"a\"; });\nconsole.log(require_a());\n";
    let keyed = format!(
        "{}console.log(require_a());\n",
        factory("a.js", "exports, module", "module.exports = \"a\";")
    );
    let named = DIRECT
        .replace("() => (", "function __require() { return ")
        .replace("mod.exports);", "mod.exports; };");
    assert!(split(&format!("{named}{direct}")).is_some(), "{named}");
    assert!(split(&format!("{GUARDED}{direct}")).is_some());
    assert!(split(&format!("{HELPERS}{keyed}")).is_some());
    for shadow in ["function mod()", "function cb()"] {
        let helper = named.replace("function __require()", shadow);
        refused_with_note(&format!("{helper}{direct}"));
    }
    refused_with_note(&format!(
        "{}{keyed}",
        HELPERS.replace("function __require()", "function __getOwnPropNames()")
    ));
    refused_with_note(&format!(
        "{}{direct}",
        GUARDED.replace(
            "catch (e) { throw mod = 0, e; }",
            "catch (mod) { throw mod = 0, mod; }"
        )
    ));
    for param in ["eval", "arguments"] {
        let helper = named.replace("cb", param);
        refused_with_note(&format!("{helper}{direct}"));
    }
    let getter_param = HELPERS
        .replace("(cb, mod)", "(__getOwnPropNames, mod)")
        .replace(
            "cb[__getOwnPropNames(cb)[0]]",
            "__getOwnPropNames[__getOwnPropNames(__getOwnPropNames)[0]]",
        );
    refused_with_note(&format!("{getter_param}{keyed}"));
}

#[test]
fn a_bound_object_in_the_keyed_helper_refuses() {
    let keyed = factory("a.js", "exports, module", "module.exports = \"a\";");
    let source = format!("{HELPERS}{keyed}console.log(require_a());\n");
    assert!(split(&source).is_some());
    refused_with_note(&format!(
        "var Object = {{ getOwnPropertyNames: () => [\"toString\"] }};\n{source}"
    ));
    refused_with_note(&format!("{source}function Object() {{}}\n"));
    let wrapped = format!("(() => {{\nvar Object = globalThis.Object;\n{source}}})();\n");
    refused_with_note(&wrapped);
}

#[test]
fn a_keyed_factory_named_proto_refuses() {
    let source = format!(
        "{HELPERS}var require_a = __commonJS({{ __proto__: function (exports, module) {{ module.exports = 1; }} }});\nconsole.log(require_a());\n"
    );
    refused_with_note(&source);
    refused_with_note(&source.replace("__proto__:", "\"__proto__\":"));
    let plain = source.replace("__proto__:", "\"a.js\":");
    assert!(split(&plain).is_some());
}

#[test]
fn super_in_a_method_form_factory_refuses() {
    refused_with_note(&format!(
        "{HELPERS}{}console.log(require_a());\n",
        factory(
            "a.js",
            "exports, module",
            "module.exports = typeof super.hasOwnProperty;"
        )
    ));
    refused_with_note(&format!(
        "{HELPERS}{}console.log(require_a());\n",
        factory(
            "a.js",
            "exports, module",
            "var f = () => super.toString;\nmodule.exports = f();"
        )
    ));
    for nested in [
        "class A { m() { return super.toString; } }\nmodule.exports = new A().m();",
        "var o = { m() { return super.toString; } };\nmodule.exports = o.m();",
        "class A { f = super.toString; }\nmodule.exports = new A().f;",
    ] {
        let source = format!(
            "{HELPERS}{}console.log(require_a());\n",
            factory("a.js", "exports, module", nested)
        );
        assert!(split(&source).is_some(), "{nested}");
    }
}

#[test]
fn entry_code_between_factory_declarations_refuses() {
    let a = "var require_a = __commonJS((exports, module) => { exports.before = true; module.exports.b = require_b(); });\n";
    let b = "var require_b = __commonJS((exports, module) => { module.exports = \"b\"; });\n";
    let tail = "console.log(JSON.stringify(require_a()));\n";
    assert!(split(&format!("{DIRECT}{a}{b}{tail}")).is_some());
    refused_with_note(&format!(
        "{DIRECT}{a}try {{ require_a(); }} catch (e) {{ console.log(\"caught\"); }}\n{b}{tail}"
    ));
    refused_with_note(&format!(
        "{DIRECT}function early() {{ return require_b(); }}\ntry {{ console.log(early()); }} catch (e) {{ console.log(\"not yet\"); }}\n{b}console.log(require_b());\n"
    ));
    refused_with_note(&format!(
        "{DIRECT}{}, early = require_a();\n{b}{tail}",
        a.trim_end().trim_end_matches(';')
    ));
    assert!(split(&format!(
        "{DIRECT}function later() {{ return require_b(); }}\nvar alias = () => require_a();\n{a}{b}console.log(later(), alias());\n"
    ))
    .is_some());
}

#[test]
fn a_lazy_helper_whose_argument_has_an_effect_is_not_moved_out_of_order() {
    let tail = "var require_lib = __commonJS((exports, module) => { init_b(); module.exports = globalThis.b; });\nconsole.log(\"entry start\");\ninit_a();\nconsole.log(require_lib());\n";
    for (a, b) in [
        (
            "(console.log(\"effect 1\"), () => {})",
            "(console.log(\"effect 2\"), () => { globalThis.b = \"b ready\"; })",
        ),
        ("...[() => {}]", "...[() => { globalThis.b = 1; }]"),
        ("make()", "make()"),
        ("{ [key()]() {} }", "{ [key()]() { globalThis.b = 1; } }"),
    ] {
        refused(&format!(
            "{DIRECT}{ESM}var init_a = __esm({a});\nvar init_b = __esm({b});\n{tail}"
        ));
    }
    for (a, b) in [
        ("() => {}", "() => { globalThis.b = 1; }"),
        ("function () {}", "function () { globalThis.b = 1; }"),
        ("{ \"a.js\"() {} }", "{ \"b.js\"() { globalThis.b = 1; } }"),
    ] {
        let files = split(&format!(
            "{DIRECT}{ESM}var init_a = __esm({a});\nvar init_b = __esm({b});\n{tail}"
        ))
        .unwrap_or_else(|| panic!("a lazy helper of a function literal moves: {b}"));
        assert!(code(&files, "_runtime/esbuild.js").contains("init_b"));
    }
}

#[test]
fn a_moved_helper_that_reads_the_bundle_context_refuses() {
    let tail = "var require_lib = __commonJS((exports, module) => { module.exports = top(); });\nconsole.log(require_lib());\n";
    for helper in [
        "var top = () => this;",
        "var top = () => this.tag;",
        "var top = () => class { [this]() {} };",
        "var top = () => typeof import.meta.url;",
        "var top = () => import(\"./chunk.js\");",
        "var top = () => arguments.length;",
    ] {
        refused(&format!("{DIRECT}{helper}\n{tail}"));
        refused(&format!(
            "import {{ sep }} from \"node:path\";\n{DIRECT}{helper}\n{tail}console.log(sep);\n"
        ));
    }
    for helper in [
        "var top = function () { return this; };",
        "function top() { return new.target; }",
        "var top = () => ({ m() { return super.toString; } });",
        "var top = () => class { x = this; };",
        "var top = async () => await 1;",
        "function* top() { yield 1; }",
    ] {
        assert!(
            split(&format!("{DIRECT}{helper}\n{tail}")).is_some(),
            "{helper}"
        );
    }
}
