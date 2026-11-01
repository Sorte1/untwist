use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_core::pass::Note;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_inputs, BundleKind, UnpackOptions};

const BUNDLERS: &str = "../../fixtures/unpack-bundlers";
const SHAPE: &str = "bundle shape found but not proven";
const GROUP: &str = "linked inputs refused as a whole";

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(BUNDLERS)
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

struct Run {
    kind: Option<BundleKind>,
    files: Vec<(String, String)>,
    notes: Vec<String>,
    entry: Option<String>,
}

impl Run {
    fn code(&self, path: &str) -> &str {
        self.files
            .iter()
            .find(|(name, _)| name == path)
            .map(|(_, code)| code.as_str())
            .unwrap_or_else(|| panic!("no {path} in {:?}", self.paths()))
    }

    fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|(name, _)| name.as_str()).collect()
    }

    fn noted(&self, text: &str) -> bool {
        self.notes.iter().any(|note| note.contains(text))
    }
}

fn messages(notes: Vec<Note>) -> Vec<String> {
    notes.into_iter().map(|note| note.message).collect()
}

fn run(sources: &[String]) -> Run {
    let alloc = Allocator::default();
    let mut programs: Vec<Program<'_>> = sources
        .iter()
        .map(|source| {
            let parsed = parse_program(&alloc, source);
            assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
            parsed.program
        })
        .collect();
    let (bundle, notes) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    let notes = messages(notes);
    let Some(bundle) = bundle else {
        return Run {
            kind: None,
            files: Vec::new(),
            notes,
            entry: None,
        };
    };
    let mut files: Vec<(String, String)> = bundle
        .modules
        .values()
        .map(|module| {
            (
                module.path.as_str().to_string(),
                pretty_codegen(module.program()),
            )
        })
        .collect();
    files.sort();
    Run {
        kind: Some(bundle.kind),
        files,
        notes,
        entry: bundle.entry_id.clone(),
    }
}

fn dir(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| fixture(name)).collect()
}

const HOIST: [&str; 2] = ["parcel/hoist/index.js", "parcel/hoist/lazy.58e54a87.js"];
const NOHOIST: [&str; 2] = ["parcel/nohoist/index.js", "parcel/nohoist/lazy.99d34c29.js"];
const SERVER: [&str; 4] = [
    "turbopack/min-server/index.js",
    "turbopack/min-server/server/chunks/ssr/[turbopack]_runtime.js",
    "turbopack/min-server/server/chunks/ssr/app.js",
    "turbopack/min-server/server/chunks/ssr/lib.js",
];

#[test]
fn a_hoisted_parcel_build_and_its_lazy_chunk_unpack_as_one_bundle() {
    let run = run(&dir(&HOIST));
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    assert_eq!(run.entry.as_deref(), Some("index"));
    let loader = run.code("kFPqQ.js");
    assert!(
        loader.contains("Promise.resolve().then(() => require(\"./ibiaQ.js\"))"),
        "{loader}"
    );
    assert!(run.code("ibiaQ.js").contains("require(\"./66COJ.js\")"));
    assert!(!run.paths().contains(&"_runtime/parcel-2.js"));
}

#[test]
fn a_registry_parcel_build_and_its_lazy_chunk_unpack_as_one_bundle() {
    let run = run(&dir(&NOHOIST));
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    let loader = run.code("jjwCr.js");
    assert!(
        loader.contains(
            "Promise.resolve(require(\"./giiHr.js\")).then(() => require(\"./giiHr.js\"))"
        ),
        "{loader}"
    );
    assert!(run.code("giiHr.js").contains("require(\"./JQa2b.js\")"));
}

#[test]
fn the_input_order_does_not_matter() {
    let forward = run(&dir(&HOIST));
    let backward = run(&dir(&[HOIST[1], HOIST[0]]));
    assert_eq!(forward.files, backward.files);
    let forward = run(&dir(&NOHOIST));
    let backward = run(&dir(&[NOHOIST[1], NOHOIST[0]]));
    assert_eq!(forward.files, backward.files);
}

#[test]
fn each_part_alone_writes_no_bundle_and_says_why() {
    for name in HOIST.iter().chain(&NOHOIST) {
        let run = run(&dir(&[name]));
        assert_eq!(run.kind, None, "{name}");
        assert!(
            run.noted(&format!("parcel {SHAPE}")),
            "{name}: {:?}",
            run.notes
        );
    }
}

fn hoist_with(index: impl Fn(String) -> String, chunk: impl Fn(String) -> String) -> Run {
    let [root, lazy] = HOIST.map(fixture);
    run(&[index(root), chunk(lazy)])
}

#[test]
fn a_chunk_with_an_effect_is_not_linked() {
    let run = hoist_with(
        |root| root,
        |lazy| format!("{lazy}console.log(\"loaded\");\n"),
    );
    assert_eq!(run.kind, None, "{:?}", run.files);
}

#[test]
fn a_loader_naming_an_id_no_chunk_registers_is_not_linked() {
    let run = hoist_with(
        |root| root.replace("parcelRequire('ibiaQ')", "parcelRequire('zzzzz')"),
        |lazy| lazy,
    );
    assert_eq!(run.kind, None);
}

#[test]
fn an_id_defined_twice_across_the_inputs_is_not_linked() {
    let run = hoist_with(
        |root| root,
        |lazy| {
            lazy.replace(
                "parcelRegister(\"ibiaQ\"",
                "parcelRegister(\"66COJ\", function(module, exports) {});\nparcelRegister(\"ibiaQ\"",
            )
        },
    );
    assert_eq!(run.kind, None);
    assert!(!run.noted(GROUP), "{:?}", run.notes);
}

#[test]
fn a_refused_chunk_refuses_the_whole_group() {
    let run = hoist_with(
        |root| root,
        |lazy| {
            lazy.replace(
                "(0, $66COJ.say)(\"eval lazy\");",
                "(0, $66COJ.say)(parcelRegister);",
            )
        },
    );
    assert_eq!(run.kind, None);
    assert!(
        run.noted(GROUP) && run.noted("written for parcel:parcelRequire6b20"),
        "{:?}",
        run.notes
    );
}

#[test]
fn a_chunk_helper_that_differs_from_the_root_refuses_the_whole_group() {
    let run = hoist_with(
        |root| root,
        |lazy| {
            lazy.replace(
                "{get: v, set: s, enumerable: true",
                "{get: v, set: s, enumerable: !0",
            )
        },
    );
    assert_eq!(run.kind, None);
    assert!(run.noted(GROUP), "{:?}", run.notes);
}

#[test]
fn a_registry_chunk_with_two_entries_is_not_linked() {
    let [root, lazy] = NOHOIST.map(fixture);
    let lazy = lazy.replace(
        "[\"giiHr\"], \"giiHr\"",
        "[\"giiHr\", \"giiHr\"], \"giiHr\"",
    );
    assert_eq!(run(&[root, lazy]).kind, None);
}

#[test]
fn a_relative_require_beside_the_loader_shape_is_still_refused() {
    let run = hoist_with(
        |root| {
            root.replace(
                "Promise.resolve(require(\"./lazy.58e54a87.js\")).then(()=>parcelRequire('ibiaQ'))",
                "Promise.resolve(require(\"./lazy.58e54a87.js\")).then(()=>parcelRequire('ibiaQ')), require(\"./other.js\")",
            )
        },
        |lazy| lazy,
    );
    assert_eq!(run.kind, None);
}

#[test]
fn a_second_root_under_the_same_global_is_not_linked() {
    let [root, lazy] = HOIST.map(fixture);
    let other = fixture("parcel/min-parcel-hoist.js");
    let global = other
        .split('"')
        .find(|text| text.starts_with("parcelRequire") && text.len() > "parcelRequire".len())
        .expect("a registry global")
        .to_string();
    let other = other.replace(&global, "parcelRequire6b20");
    let run = run(&[root, lazy, other]);
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    assert!(!run.noted(GROUP), "{:?}", run.notes);
    assert!(!run.paths().contains(&"kFPqQ.js"), "{:?}", run.paths());
}

#[test]
fn turbopack_server_chunks_and_the_next_loader_unpack_as_one_bundle() {
    let run = run(&dir(&SERVER));
    assert_eq!(run.kind, Some(BundleKind::Turbopack), "{:?}", run.notes);
    let entry = run.code("index.js");
    assert!(
        entry.contains(
            "require(\"./server/chunks/ssr/[turbopack]_runtime.js\")(\"server/pages/index.js\")"
        ),
        "{entry}"
    );
    assert!(entry.contains("require(\"./301.js\")"), "{entry}");
    let app = run.code("301.js");
    assert!(app.contains("require(\"./201.js\")"), "{app}");
    assert!(app.starts_with("\"use strict\""), "{app}");
    assert!(run.code("302.js").contains("module.exports = function"));
    assert!(run
        .code("server/chunks/ssr/[turbopack]_runtime.js")
        .contains("getOrInstantiateRuntimeModule"));
}

#[test]
fn a_refused_server_chunk_refuses_the_whole_group() {
    let mut sources = dir(&SERVER);
    sources[2] = fixture("turbopack/neg-server/server/chunks/ssr/app.js");
    let run = run(&sources);
    assert_eq!(run.kind, None);
    assert!(
        run.noted(GROUP) && run.noted("written for turbopack:server"),
        "{:?}",
        run.notes
    );
}

#[test]
fn no_loader_or_no_runtime_links_nothing() {
    let sources = dir(&SERVER);
    assert_eq!(run(&sources[1..]).kind, None);
    let without_runtime = vec![sources[0].clone(), sources[2].clone(), sources[3].clone()];
    assert_eq!(run(&without_runtime).kind, None);
}

#[test]
fn a_second_loader_or_a_duplicate_id_links_nothing() {
    let mut sources = dir(&SERVER);
    sources.push(sources[0].clone());
    assert_eq!(run(&sources).kind, None);
    let mut sources = dir(&SERVER);
    sources[3] = sources[3].replace("202,", "301,");
    let run = run(&sources);
    assert_eq!(run.kind, None);
    assert!(!run.noted(GROUP), "{:?}", run.notes);
}

#[test]
fn a_loader_entry_that_no_chunk_defines_links_nothing() {
    let mut sources = dir(&SERVER);
    sources[0] = sources[0].replace("R.m(301)", "R.m(399)");
    assert_eq!(run(&sources).kind, None);
}

#[test]
fn a_runtime_path_outside_the_entry_directory_links_nothing() {
    let mut sources = dir(&SERVER);
    sources[0] = sources[0].replace("require(\"./server/", "require(\"../server/");
    assert_eq!(run(&sources).kind, None);
}

#[test]
fn a_runtime_path_under_node_modules_links_nothing() {
    let mut sources = dir(&SERVER);
    sources[0] = sources[0].replace("require(\"./server/", "require(\"./node_modules/server/");
    assert_eq!(run(&sources).kind, None);
}

#[test]
fn a_server_chunk_alone_writes_no_bundle_and_says_why() {
    let run = run(&dir(&[SERVER[3]]));
    assert_eq!(run.kind, None);
    assert!(run.noted(&format!("turbopack {SHAPE}")), "{:?}", run.notes);
}

#[test]
fn an_input_no_probe_matches_carries_no_note() {
    let run = run(&["console.log(1);\n".to_string()]);
    assert_eq!(run.kind, None);
    assert!(run.notes.is_empty(), "{:?}", run.notes);
}

#[test]
fn a_refused_new_kind_names_itself() {
    for (name, kind) in [
        ("systemjs/neg-system-cycle.js", "systemjs"),
        ("amd/neg-amd-cycle.js", "amd"),
        ("metro/neg-metro-five-params.js", "metro"),
        ("parcel/neg-parcel-foreign-id.js", "parcel"),
    ] {
        let run = run(&dir(&[name]));
        assert_eq!(run.kind, None, "{name}");
        assert!(
            run.noted(&format!("{kind} {SHAPE}")),
            "{name}: {:?}",
            run.notes
        );
    }
}

#[test]
fn a_written_bundle_carries_no_refusal_note() {
    let run = run(&dir(&["parcel/single-hoist.js"]));
    assert_eq!(run.kind, Some(BundleKind::Parcel));
    assert!(!run.noted(SHAPE), "{:?}", run.notes);
}

const RUNTIME: &str = "_runtime/parcel.js";
const LAZY_ANCHOR: &str = "(0, $66COJ.say)(\"eval lazy\");";
const ROOT_ANCHOR: &str = "$472169e8c320a18a$export$91d88f71a134afea(\"eval say\");";
const HELPER_HEAD: &str = "function $parcel$export(e, n, v, s) {\n";
const BODY: &str = "Object.defineProperty(e, n, {get: v";
const VAR_HEAD: &str = "var $parcel$export = function self(e, n, v, s) {\n";
const ARROW_HEAD: &str = "var $parcel$export = (e, n, v, s) => {\n";

fn in_lazy(code: &str) -> Run {
    hoist_with(
        |root| root,
        |lazy| lazy.replace(LAZY_ANCHOR, &format!("{LAZY_ANCHOR}\n{code}")),
    )
}

fn in_root(code: &str) -> Run {
    hoist_with(
        |root| root.replace(ROOT_ANCHOR, &format!("{ROOT_ANCHOR}\n{code}")),
        |lazy| lazy,
    )
}

fn in_both(edit: impl Fn(String) -> String) -> Run {
    hoist_with(&edit, &edit)
}

fn shares(run: &Run) -> bool {
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    !run.code(RUNTIME).contains("$parcel$export$1")
}

fn kept_apart(run: &Run, copy: &str) {
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    let runtime = run.code(RUNTIME);
    assert!(runtime.contains(HELPER_HEAD), "{runtime}");
    assert!(
        runtime.contains(&format!("function {copy}(e, n, v, s) {{")),
        "{runtime}"
    );
    let chunk = run.code("ibiaQ.js");
    assert!(
        chunk
            .lines()
            .next()
            .is_some_and(|import| import.contains(copy) && import.contains("require"))
            && chunk.contains(&format!("{copy}(module.exports, \"default\"")),
        "{chunk}"
    );
    assert!(!run.code("66COJ.js").contains(copy));
    assert!(!run.code("index.js").contains(&format!("{copy}(")));
}

#[test]
fn a_helper_only_ever_called_directly_is_shared() {
    assert!(shares(&run(&dir(&HOIST))));
}

#[test]
fn a_helper_read_as_a_value_in_the_chunk_is_kept_apart() {
    kept_apart(&in_lazy("var kept = $parcel$export;"), "$parcel$export$1");
}

#[test]
fn a_helper_property_written_in_the_root_is_kept_apart() {
    kept_apart(&in_root("$parcel$export.calls = 1;"), "$parcel$export$1");
}

#[test]
fn a_helper_compared_in_the_chunk_is_kept_apart() {
    kept_apart(
        &in_lazy("console.log($parcel$export === undefined);"),
        "$parcel$export$1",
    );
}

#[test]
fn a_helper_under_typeof_in_the_root_is_kept_apart() {
    kept_apart(
        &in_root("console.log(typeof $parcel$export);"),
        "$parcel$export$1",
    );
}

#[test]
fn an_optional_call_of_the_helper_is_kept_apart() {
    let run = hoist_with(
        |root| root,
        |lazy| {
            lazy.replace(
                "$parcel$export(module.exports, \"value\"",
                "$parcel$export?.(module.exports, \"value\"",
            )
        },
    );
    kept_apart(&run, "$parcel$export$1");
}

#[test]
fn a_helper_that_writes_a_global_is_kept_apart() {
    let run = in_both(|text| text.replace(BODY, &format!("seen = 1, {BODY}")));
    kept_apart(&run, "$parcel$export$1");
}

#[test]
fn a_helper_that_reads_its_arguments_object_is_kept_apart() {
    let run = in_both(|text| text.replace(BODY, &format!("arguments.length, {BODY}")));
    kept_apart(&run, "$parcel$export$1");
}

#[test]
fn a_helper_that_reads_another_helper_is_kept_apart_and_the_other_shared() {
    let run = in_both(|text| {
        text.replace(
            HELPER_HEAD,
            &format!("function $parcel$id(a) {{\n  return a;\n}}\n{HELPER_HEAD}"),
        )
        .replace("{get: v,", "{get: $parcel$id(v),")
    });
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert!(!runtime.contains("$parcel$id$1"), "{runtime}");
    assert_eq!(
        runtime.matches("function $parcel$id(").count(),
        1,
        "{runtime}"
    );
    assert_eq!(
        runtime.matches("get: $parcel$id(v)").count(),
        2,
        "{runtime}"
    );
}

#[test]
fn a_recursive_helper_is_kept_apart_and_its_copy_calls_itself() {
    let run =
        in_both(|text| text.replace(BODY, &format!("e || $parcel$export({{}}, n, v, s), {BODY}")));
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert!(
        runtime.contains("e || $parcel$export$1({}, n, v, s)"),
        "{runtime}"
    );
}

#[test]
fn the_copy_takes_a_name_the_runtime_does_not_use() {
    let run = hoist_with(
        |root| {
            root.replace(
                HELPER_HEAD,
                &format!("{HELPER_HEAD}  var $parcel$export$1 = 0;\n"),
            )
        },
        |lazy| {
            lazy.replace(
                HELPER_HEAD,
                &format!("{HELPER_HEAD}  var $parcel$export$1 = 0;\n"),
            )
            .replace(
                LAZY_ANCHOR,
                &format!("{LAZY_ANCHOR}\nvar kept = $parcel$export;"),
            )
        },
    );
    kept_apart(&run, "$parcel$export$2");
}

#[test]
fn a_helper_closed_in_the_chunk_but_not_in_the_root_is_kept_apart() {
    let run = hoist_with(
        |root| {
            root.replace(
                HELPER_HEAD,
                &format!("function $parcel$id(a) {{\n  return a;\n}}\n{HELPER_HEAD}"),
            )
            .replace("{get: v,", "{get: $parcel$id(v),")
        },
        |lazy| lazy.replace("{get: v,", "{get: $parcel$id(v),"),
    );
    kept_apart(&run, "$parcel$export$1");
}

#[test]
fn a_named_function_expression_that_reads_its_own_name_is_kept_apart() {
    let run = in_both(|text| {
        text.replace(HELPER_HEAD, VAR_HEAD)
            .replace(BODY, &format!("self.length, {BODY}"))
    });
    assert_eq!(run.kind, Some(BundleKind::Parcel), "{:?}", run.notes);
    let runtime = run.code(RUNTIME);
    assert!(
        runtime.contains("var $parcel$export$1 = function self(e, n, v, s)"),
        "{runtime}"
    );
    assert!(run
        .code("ibiaQ.js")
        .contains("$parcel$export$1(module.exports, \"value\""));
}

#[test]
fn an_arrow_helper_only_ever_called_directly_is_shared() {
    let run = in_both(|text| text.replace(HELPER_HEAD, ARROW_HEAD));
    assert!(shares(&run));
    assert!(run
        .code(RUNTIME)
        .contains("var $parcel$export = (e, n, v, s) =>"));
}

#[test]
fn a_helper_another_helper_reads_as_a_value_is_kept_apart() {
    let reader = format!("function $parcel$id(a) {{\n  return $parcel$export;\n}}\n{HELPER_HEAD}");
    let run = hoist_with(
        |root| {
            root.replace(HELPER_HEAD, &reader)
                .replace(ROOT_ANCHOR, &format!("{ROOT_ANCHOR}\n$parcel$id(0);"))
        },
        |lazy| {
            lazy.replace(HELPER_HEAD, &reader)
                .replace(LAZY_ANCHOR, &format!("{LAZY_ANCHOR}\n$parcel$id(0);"))
        },
    );
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert!(runtime.contains("return $parcel$export$1;"), "{runtime}");
}

fn kept_apart_with(local: &str) -> Run {
    hoist_with(
        |root| root.replace(BODY, &format!("{local} {BODY}")),
        |lazy| {
            lazy.replace(BODY, &format!("{local} {BODY}")).replace(
                LAZY_ANCHOR,
                &format!("{LAZY_ANCHOR}\nvar kept = $parcel$export;"),
            )
        },
    )
}

#[test]
fn a_copy_naming_itself_in_a_shorthand_keeps_the_property_key() {
    let run = in_both(|text| text.replace(BODY, &format!("({{ $parcel$export }}), {BODY}")));
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert!(
        runtime.contains("$parcel$export: $parcel$export$1"),
        "{runtime}"
    );
}

#[test]
fn a_copy_leaves_a_local_binding_of_its_name_alone() {
    let run = kept_apart_with("{ let { $parcel$export } = {}; }");
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert_eq!(
        runtime.matches("let { $parcel$export } = {};").count(),
        2,
        "{runtime}"
    );
}

#[test]
fn a_copy_leaves_a_local_assigned_through_a_shorthand_alone() {
    let run = kept_apart_with("{ let $parcel$export; ({ $parcel$export } = e); }");
    kept_apart(&run, "$parcel$export$1");
    let runtime = run.code(RUNTIME);
    assert_eq!(
        runtime.matches("({$parcel$export} = e);").count(),
        2,
        "{runtime}"
    );
}

#[test]
fn the_copy_takes_a_name_no_module_uses() {
    let run = in_lazy("var kept = $parcel$export;\nvar $parcel$export$1 = 0;");
    kept_apart(&run, "$parcel$export$2");
}
