use std::path::PathBuf;

use untwist_core::pass::PassOptions;
use untwist_testing::Allocator;

fn spellings() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/builtin_patches");
    let mut rows: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("patch spellings")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .map(|path| {
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read_to_string(&path).expect("source"))
        })
        .collect();
    rows.sort();
    rows
}

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[
            untwist_unminify::json_parse::boxed,
            untwist_unminify::remove_double_not::boxed,
        ],
        PassOptions::default(),
    )
}

#[test]
fn every_patch_spelling_keeps_the_program_output_by_default() {
    let rows = spellings();
    assert_eq!(rows.len(), 67);
    let changed: Vec<String> = rows
        .into_iter()
        .filter(|(_, source)| {
            let output = default_lane(source);
            untwist_testing::node::printed_by_node(&output)
                != untwist_testing::node::printed_by_node(source)
        })
        .map(|(name, _)| name)
        .collect();
    assert!(changed.is_empty(), "{changed:?}");
}

#[test]
fn a_strict_function_or_class_this_leaves_the_rewrites_on() {
    let rows = [
        "(function () { \"use strict\"; var self = this; g(self); })();\nvar x = JSON.parse(\"[1]\");",
        "class C { m() { return this; } }\nvar x = JSON.parse(\"[1]\");",
    ];
    for source in rows {
        let output = default_lane(source);
        assert!(output.contains("var x = [1];"), "{output}");
    }
}
