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

#[test]
fn exhausting_the_suffix_range_is_reported_distinctly() {
    let alloc = Allocator::default();
    let sources: Vec<(String, &str, bool)> = (0..1001)
        .map(|i| (format!("m{i}"), "x();\n", false))
        .collect();
    let refs: Vec<(&str, &str, bool)> = sources
        .iter()
        .map(|(id, code, is_esm)| (id.as_str(), *code, *is_esm))
        .collect();
    let mut bundle = bundle_of(&alloc, &refs, None);
    for module in bundle.modules.values_mut() {
        module.path = PosixPath::parse("shared.js").expect("safe");
    }
    let notes = assign_paths(&mut bundle);
    assert!(
        notes
            .iter()
            .any(|note| note.message.contains("could not uniquify")
                && note.message.contains("shared.js")),
        "{notes:?}"
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
