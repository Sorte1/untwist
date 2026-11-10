mod common;

use common::bundle_of;
use oxc_allocator::Allocator;
use untwist_unpack::assign::assign_paths;
use untwist_unpack::path::PosixPath;

fn paths(bundle: &untwist_unpack::model::Bundle<'_>) -> Vec<String> {
    bundle
        .modules
        .values()
        .map(|module| module.path.as_str().to_string())
        .collect()
}

#[test]
fn safe_distinct_paths_are_left_alone() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("1", "x();\n", false), ("2", "y();\n", false)],
        Some("2"),
    );
    assert!(assign_paths(&mut bundle).is_empty());
    assert_eq!(
        paths(&bundle),
        vec!["1.js".to_string(), "index.js".to_string()]
    );
}

#[test]
fn colliding_paths_are_uniquified_from_the_original_stem() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            ("1", "x();\n", false),
            ("2", "y();\n", false),
            ("3", "z();\n", false),
        ],
        None,
    );
    for id in ["1", "2", "3"] {
        bundle.modules[id].path = PosixPath::parse("utils/color.js").expect("safe");
    }
    let notes = assign_paths(&mut bundle);
    assert_eq!(notes.len(), 2);
    assert_eq!(
        paths(&bundle),
        vec![
            "utils/color.js".to_string(),
            "utils/color-2.js".to_string(),
            "utils/color-3.js".to_string()
        ]
    );
}

fn bundle_of_ids<'a>(alloc: &'a Allocator, ids: &[String]) -> untwist_unpack::model::Bundle<'a> {
    let sources: Vec<(&str, &str, bool)> = ids
        .iter()
        .map(|id| (id.as_str(), "x();\n", false))
        .collect();
    bundle_of(alloc, &sources, None)
}

fn assert_distinct_ignoring_case(paths: &[String]) {
    let folded: std::collections::HashSet<String> =
        paths.iter().map(|path| path.to_lowercase()).collect();
    assert_eq!(folded.len(), paths.len(), "{paths:?}");
}

const MANY_COLLIDING_IDS: usize = 20_000;
const LINEAR_TIME_BUDGET: std::time::Duration = std::time::Duration::from_secs(5);

#[test]
fn thousands_of_ids_that_normalise_to_one_path_all_get_distinct_names_quickly() {
    let alloc = Allocator::default();
    let ids: Vec<String> = (0..MANY_COLLIDING_IDS)
        .map(|index| format!("x{index}/../a"))
        .collect();
    let mut bundle = bundle_of_ids(&alloc, &ids);

    let started = std::time::Instant::now();
    assign_paths(&mut bundle);
    let elapsed = started.elapsed();

    let assigned = paths(&bundle);
    assert_eq!(assigned.len(), MANY_COLLIDING_IDS);
    assert_distinct_ignoring_case(&assigned);
    assert_eq!(assigned[0], "a.js");
    assert_eq!(assigned[1], "a-2.js");
    assert_eq!(
        assigned[MANY_COLLIDING_IDS - 1],
        format!("a-{MANY_COLLIDING_IDS}.js")
    );
    assert!(elapsed < LINEAR_TIME_BUDGET, "{elapsed:?}");
}

#[test]
fn a_very_deep_module_id_gets_a_flat_name() {
    let alloc = Allocator::default();
    let ids = [format!("{}a", "a/".repeat(20_000)), "b".to_string()];
    let mut bundle = bundle_of_ids(&alloc, &ids);

    assign_paths(&mut bundle);

    let assigned = paths(&bundle);
    assert!(
        assigned[0].starts_with("_unsafe_a_a_"),
        "{}",
        &assigned[0][..40]
    );
    assert!(!assigned[0].contains('/'));
    assert_eq!(assigned[1], "b.js");
}

#[test]
fn a_numbered_name_another_module_already_holds_is_skipped() {
    let alloc = Allocator::default();
    let ids = ["a-2".to_string(), "a".to_string(), "x/../a".to_string()];
    let mut bundle = bundle_of_ids(&alloc, &ids);

    assign_paths(&mut bundle);

    assert_eq!(
        paths(&bundle),
        vec![
            "a-2.js".to_string(),
            "a.js".to_string(),
            "a-3.js".to_string()
        ]
    );
}

#[test]
fn ids_that_differ_only_in_case_get_names_that_differ_in_more_than_case() {
    let alloc = Allocator::default();
    let ids = [
        "A".to_string(),
        "a".to_string(),
        "lib/Util".to_string(),
        "LIB/util".to_string(),
    ];
    let mut bundle = bundle_of_ids(&alloc, &ids);

    let notes = assign_paths(&mut bundle);

    let assigned = paths(&bundle);
    assert_distinct_ignoring_case(&assigned);
    assert_eq!(assigned[0], "A.js");
    assert_eq!(assigned[1], "a-2.js");
    assert_eq!(notes.len(), 2, "{notes:?}");
}

#[test]
fn a_module_under_a_path_another_module_holds_as_a_file_gets_a_flat_name() {
    let alloc = Allocator::default();
    let ids = [
        "a.js".to_string(),
        "a.js/b".to_string(),
        "A.JS/c".to_string(),
    ];
    let mut bundle = bundle_of_ids(&alloc, &ids);

    let notes = assign_paths(&mut bundle);

    let assigned = paths(&bundle);
    assert_eq!(assigned[0], "a.js");
    assert!(!assigned[1].contains('/'), "{assigned:?}");
    assert!(!assigned[2].contains('/'), "{assigned:?}");
    assert_distinct_ignoring_case(&assigned);
    assert_eq!(notes.len(), 2, "{notes:?}");
}

#[test]
fn a_module_named_like_a_directory_other_modules_live_in_is_renamed() {
    let alloc = Allocator::default();
    let ids = ["a.js/b".to_string(), "a.js".to_string(), "A".to_string()];
    let mut bundle = bundle_of_ids(&alloc, &ids);

    let notes = assign_paths(&mut bundle);

    assert_eq!(
        paths(&bundle),
        vec![
            "a.js/b.js".to_string(),
            "a-2.js".to_string(),
            "A-3.js".to_string()
        ]
    );
    assert_eq!(notes.len(), 2, "{notes:?}");
}

#[test]
fn the_names_the_tool_writes_itself_are_never_given_to_a_module() {
    let alloc = Allocator::default();
    let ids = [
        "deobfuscated".to_string(),
        "Deobfuscated".to_string(),
        "listing".to_string(),
        "binary".to_string(),
        "wasm/inside".to_string(),
    ];
    let mut bundle = bundle_of_ids(&alloc, &ids);
    bundle.modules["listing"].path = PosixPath::parse("BUNDLE.json").expect("safe");
    bundle.modules["binary"].path = PosixPath::parse("wasm").expect("safe");

    assign_paths(&mut bundle);

    assert_eq!(
        paths(&bundle),
        vec![
            "deobfuscated-2.js".to_string(),
            "Deobfuscated-3.js".to_string(),
            "BUNDLE-2.json".to_string(),
            "wasm-2".to_string(),
            "wasm/inside.js".to_string(),
        ]
    );
}

#[test]
fn an_unsafe_path_is_replaced_and_reported() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(&alloc, &[("../tmp.js", "x();\n", false)], None);
    bundle.modules["../tmp.js"].path = PosixPath::default();
    bundle.modules["../tmp.js"].unsafe_path_source = None;
    let notes = assign_paths(&mut bundle);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("unsafe output path"), "{notes:?}");
    assert_eq!(paths(&bundle), vec!["_unsafe____tmp_js.js".to_string()]);
}

#[test]
fn an_unsafe_module_id_is_reported_once() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(&alloc, &[("../evil", "x();\n", false)], None);
    let notes = assign_paths(&mut bundle);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("unsafe path"), "{notes:?}");
    assert!(notes[0].message.contains("../evil.js"), "{notes:?}");
    assert!(
        bundle.modules["../evil"]
            .path
            .as_str()
            .starts_with("_unsafe_"),
        "{}",
        bundle.modules["../evil"].path.as_str()
    );
}

#[test]
fn a_non_entry_module_named_index_leaves_index_js_to_the_entry() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            ("index", "x();\n", false),
            ("main", "y();\n", false),
            ("index-2", "z();\n", false),
        ],
        Some("main"),
    );
    let notes = assign_paths(&mut bundle);
    assert_eq!(
        paths(&bundle),
        vec![
            "index-2.js".to_string(),
            "index.js".to_string(),
            "index-2-2.js".to_string()
        ]
    );
    assert_eq!(notes.len(), 2, "{notes:?}");
}

#[test]
fn without_an_entry_a_module_named_index_keeps_index_js() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("index", "x();\n", false), ("main", "y();\n", false)],
        None,
    );
    assert!(assign_paths(&mut bundle).is_empty());
    assert_eq!(
        paths(&bundle),
        vec!["index.js".to_string(), "main.js".to_string()]
    );
}
