mod common;

use common::{bundle_from, webpack4_without_module_value};
use oxc_allocator::Allocator;
use untwist_unpack::mappings::{apply_mappings, Mapping, MappingMatcher};

fn mapping(path: &str, matcher: MappingMatcher) -> Mapping {
    Mapping {
        path: path.to_string(),
        matcher,
    }
}

#[test]
fn mappings_move_modules_and_unused_ones_are_reported() {
    let source = webpack4_without_module_value();
    let alloc = Allocator::default();
    let mut bundle = bundle_from(&alloc, &source).expect("webpack bundle");
    let mappings = vec![
        mapping(
            "./utils/color.js",
            MappingMatcher::StringLiteral("#FBC02D".to_string()),
        ),
        mapping("package", MappingMatcher::NumericLiteral(4.0)),
        mapping(
            "./never.js",
            MappingMatcher::Identifier("neverSeen".to_string()),
        ),
    ];
    let notes = apply_mappings(&mut bundle, &mappings);
    assert_eq!(bundle.modules["2"].path.as_str(), "utils/color.js");
    assert_eq!(bundle.modules["3"].path.as_str(), "node_modules/package");
    assert_eq!(bundle.modules["1"].path.as_str(), "1.js");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("no module matched"), "{notes:?}");
}

#[test]
fn a_member_matcher_selects_by_object_and_property() {
    let alloc = Allocator::default();
    let mut bundle = common::bundle_of(
        &alloc,
        &[
            ("1", "lodash.merge(a, b);\n", false),
            ("2", "x();\n", false),
        ],
        None,
    );
    let mappings = vec![mapping(
        "lodash/index.js",
        MappingMatcher::Member {
            object: "lodash".to_string(),
            property: "merge".to_string(),
        },
    )];
    assert!(apply_mappings(&mut bundle, &mappings).is_empty());
    assert_eq!(
        bundle.modules["1"].path.as_str(),
        "node_modules/lodash/index.js"
    );
    assert_eq!(bundle.modules["2"].path.as_str(), "2.js");
}

#[test]
fn a_mapping_is_used_at_most_once() {
    let alloc = Allocator::default();
    let mut bundle = common::bundle_of(
        &alloc,
        &[("1", "x(1);\n", false), ("2", "y(1);\n", false)],
        None,
    );
    let mappings = vec![mapping("./one.js", MappingMatcher::NumericLiteral(1.0))];
    assert!(apply_mappings(&mut bundle, &mappings).is_empty());
    assert_eq!(bundle.modules["1"].path.as_str(), "one.js");
    assert_eq!(bundle.modules["2"].path.as_str(), "2.js");
}

#[test]
fn an_unsafe_mapping_path_is_refused() {
    let alloc = Allocator::default();
    let mut bundle = common::bundle_of(&alloc, &[("1", "x(1);\n", false)], None);
    let mappings = vec![mapping("../escape.js", MappingMatcher::NumericLiteral(1.0))];
    let notes = apply_mappings(&mut bundle, &mappings);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("unsafe path"), "{notes:?}");
    assert_eq!(bundle.modules["1"].path.as_str(), "1.js");
}

#[test]
fn applying_a_mapping_clears_a_stale_unsafe_path_source() {
    let alloc = Allocator::default();
    let mut bundle = common::bundle_of(&alloc, &[("1", "x(1);\n", false)], None);
    bundle
        .modules
        .get_mut("1")
        .expect("module")
        .unsafe_path_source = Some("../escape.js".to_string());
    let mappings = vec![mapping("./one.js", MappingMatcher::NumericLiteral(1.0))];
    assert!(apply_mappings(&mut bundle, &mappings).is_empty());
    assert_eq!(bundle.modules["1"].path.as_str(), "one.js");
    assert_eq!(bundle.modules["1"].unsafe_path_source, None);
}

#[test]
fn no_mappings_means_no_work() {
    let alloc = Allocator::default();
    let mut bundle = common::bundle_of(&alloc, &[("1", "x();\n", false)], None);
    assert!(apply_mappings(&mut bundle, &[]).is_empty());
    assert_eq!(bundle.modules["1"].path.as_str(), "1.js");
}
