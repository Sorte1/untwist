use indexmap::IndexMap;
use rustc_hash::FxHashSet;
use untwist_unpack::browserify::tree::{resolve_dependency_tree, DependencyTree};

fn tree(entries: &[(&str, &[(&str, &str)])]) -> DependencyTree {
    entries
        .iter()
        .map(|(id, dependencies)| {
            (
                (*id).to_string(),
                dependencies
                    .iter()
                    .map(|(dep, request)| ((*dep).to_string(), (*request).to_string()))
                    .collect(),
            )
        })
        .collect()
}

fn resolved(tree: &DependencyTree, entry: &str) -> IndexMap<String, String> {
    resolve_dependency_tree(tree, entry, &FxHashSet::default())
}

fn assert_paths(actual: &IndexMap<String, String>, expected: &[(&str, &str)]) {
    let mut actual: Vec<(String, String)> = actual
        .iter()
        .map(|(id, path)| (id.clone(), path.clone()))
        .collect();
    actual.sort();
    let mut expected: Vec<(String, String)> = expected
        .iter()
        .map(|(id, path)| ((*id).to_string(), (*path).to_string()))
        .collect();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn a_deep_tree_gets_a_synthesized_prefix() {
    let tree = tree(&[
        ("0", &[("1", "./a.js"), ("4", "lib")]),
        ("1", &[("2", "../bar/b.js")]),
        ("2", &[("3", "../../c.js")]),
        ("3", &[]),
        ("4", &[]),
    ]);
    assert_paths(
        &resolved(&tree, "0"),
        &[
            ("0", "tmp0/tmp1/index.js"),
            ("1", "tmp0/tmp1/a.js"),
            ("2", "tmp0/bar/b.js"),
            ("3", "c.js"),
            ("4", "node_modules/lib/index.js"),
        ],
    );
}

#[test]
fn package_internals_resolve_against_the_package_directory() {
    let tree = tree(&[
        ("1", &[]),
        ("2", &[("5", "./v1"), ("6", "./v4")]),
        ("3", &[]),
        ("4", &[]),
        ("5", &[("3", "./lib/bytesToUuid"), ("4", "./lib/rng")]),
        ("6", &[("3", "./lib/bytesToUuid"), ("4", "./lib/rng")]),
        ("7", &[("1", "number"), ("2", "uuid")]),
    ]);
    assert_paths(
        &resolved(&tree, "7"),
        &[
            ("1", "node_modules/number/index.js"),
            ("2", "node_modules/uuid/index.js"),
            ("3", "node_modules/uuid/lib/bytesToUuid.js"),
            ("4", "node_modules/uuid/lib/rng.js"),
            ("5", "node_modules/uuid/v1.js"),
            ("6", "node_modules/uuid/v4.js"),
            ("7", "index.js"),
        ],
    );
}

#[test]
fn circular_dependencies_terminate() {
    let tree = tree(&[
        ("1", &[("2", "./utils")]),
        ("2", &[("1", "./base64")]),
        ("3", &[("1", "./base64")]),
    ]);
    assert_paths(
        &resolved(&tree, "3"),
        &[("1", "base64.js"), ("2", "utils.js"), ("3", "index.js")],
    );
}

#[test]
fn an_unknown_entry_resolves_to_index_only() {
    let tree = tree(&[("1", &[])]);
    assert_paths(&resolved(&tree, "9"), &[("9", "index.js")]);
}

use untwist_unpack::browserify::tree::infer_index_directories;

fn repaired(tree: &DependencyTree, entry: &str) -> IndexMap<String, String> {
    resolve_dependency_tree(tree, entry, &infer_index_directories(tree, entry))
}

#[test]
fn an_index_directory_is_inferred_from_an_escaping_request() {
    let tree = tree(&[
        ("1", &[("2", "./utils"), ("4", "./test")]),
        ("2", &[("3", "./lib"), ("4", "../test")]),
        ("3", &[]),
        ("4", &[]),
    ]);
    assert_paths(
        &repaired(&tree, "1"),
        &[
            ("1", "index.js"),
            ("2", "utils/index.js"),
            ("3", "utils/lib.js"),
            ("4", "test.js"),
        ],
    );
}

#[test]
fn inference_leaves_the_passing_cases_untouched() {
    let uuid = tree(&[
        ("1", &[]),
        ("2", &[("5", "./v1"), ("6", "./v4")]),
        ("3", &[]),
        ("4", &[]),
        ("5", &[("3", "./lib/bytesToUuid"), ("4", "./lib/rng")]),
        ("6", &[("3", "./lib/bytesToUuid"), ("4", "./lib/rng")]),
        ("7", &[("1", "number"), ("2", "uuid")]),
    ]);
    assert!(infer_index_directories(&uuid, "7").is_empty());
    let deep = tree(&[
        ("0", &[("1", "./a.js"), ("4", "lib")]),
        ("1", &[("2", "../bar/b.js")]),
        ("2", &[("3", "../../c.js")]),
        ("3", &[]),
        ("4", &[]),
    ]);
    assert!(infer_index_directories(&deep, "0").is_empty());
    let circular = tree(&[
        ("1", &[("2", "./utils")]),
        ("2", &[("1", "./base64")]),
        ("3", &[("1", "./base64")]),
    ]);
    assert!(infer_index_directories(&circular, "3").is_empty());
}
