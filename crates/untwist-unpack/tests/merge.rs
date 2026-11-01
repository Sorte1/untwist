mod common;

use common::bundle_of;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::merge::merge_bundles;
use untwist_unpack::model::BundleKind;

#[test]
fn modules_and_chunk_ids_are_unioned() {
    let alloc = Allocator::default();
    let mut first = bundle_of(&alloc, &[("2wwy", "x();\n", false)], None);
    first.chunk_ids = vec!["15".to_string()];
    let mut second = bundle_of(&alloc, &[("w6GO", "y();\n", false)], None);
    second.chunk_ids = vec!["16".to_string()];
    let (merged, notes) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert!(notes.is_empty());
    assert_eq!(
        merged.modules.keys().cloned().collect::<Vec<_>>(),
        vec!["2wwy", "w6GO"]
    );
    assert_eq!(merged.chunk_ids, vec!["15".to_string(), "16".to_string()]);
    assert_eq!(merged.entry_id, None);
}

#[test]
fn the_entry_from_any_chunk_wins_and_is_retagged() {
    let alloc = Allocator::default();
    let first = bundle_of(&alloc, &[("3", "x();\n", false)], None);
    let second = bundle_of(&alloc, &[("2", "y();\n", false)], Some("2"));
    let (merged, _) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert_eq!(merged.entry_id.as_deref(), Some("2"));
    assert!(merged.modules["2"].is_entry);
    assert_eq!(merged.modules["2"].path.as_str(), "index.js");
    assert_eq!(merged.modules["3"].path.as_str(), "3.js");
}

#[test]
fn a_second_entry_from_a_later_chunk_is_ignored_and_reported() {
    let alloc = Allocator::default();
    let first = bundle_of(&alloc, &[("2", "x();\n", false)], Some("2"));
    let second = bundle_of(&alloc, &[("3", "y();\n", false)], Some("3"));
    let (merged, notes) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert_eq!(merged.entry_id.as_deref(), Some("2"));
    assert_eq!(notes.len(), 1);
    assert!(
        notes[0]
            .message
            .contains("entry 3 from a later chunk ignored; keeping 2"),
        "{notes:?}"
    );
}

#[test]
fn duplicate_module_ids_keep_the_first_and_are_reported() {
    let alloc = Allocator::default();
    let first = bundle_of(&alloc, &[("3", "first();\n", false)], None);
    let second = bundle_of(&alloc, &[("3", "second();\n", false)], None);
    let (merged, notes) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert_eq!(merged.modules["3"].code().trim_end(), "first();");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("duplicate module 3"), "{notes:?}");
}

#[test]
fn a_bundle_of_another_kind_is_skipped_with_a_note() {
    let alloc = Allocator::default();
    let first = bundle_of(&alloc, &[("3", "x();\n", false)], None);
    let mut second = bundle_of(&alloc, &[("4", "y();\n", false)], None);
    second.kind = BundleKind::Browserify;
    let (merged, notes) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert_eq!(merged.modules.len(), 1);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].message.contains("browserify"), "{notes:?}");
}

#[test]
fn merging_nothing_yields_nothing() {
    assert!(merge_bundles(&Allocator::default(), Vec::new()).is_none());
}

#[test]
fn a_second_bundle_of_a_kind_without_chunks_is_skipped_with_a_note() {
    let alloc = Allocator::default();
    let mut first = bundle_of(&alloc, &[("index", "x();\n", true)], Some("index"));
    first.kind = BundleKind::Esbuild;
    let mut second = bundle_of(
        &alloc,
        &[("index", "y();\n", true), ("a", "z();\n", false)],
        Some("index"),
    );
    second.kind = BundleKind::Esbuild;
    let (merged, notes) = merge_bundles(&alloc, vec![first, second]).expect("merged bundle");
    assert_eq!(
        merged.modules.keys().cloned().collect::<Vec<_>>(),
        vec!["index"]
    );
    assert_eq!(merged.modules["index"].code().trim_end(), "x();");
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert!(
        notes[0]
            .message
            .contains("skipped a second self-contained esbuild bundle"),
        "{notes:?}"
    );
}
