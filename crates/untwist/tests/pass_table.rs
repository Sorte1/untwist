use std::collections::BTreeSet;

use untwist::{all_pass_ctors, pass_named, PassInfo, PASSES};
use untwist_deobfuscate::unstamp::UNSTAMP_REMAINING_PASS;
use untwist_unminify::untranspile_ctors;

const RENAME_PASSES: [&str; 6] = [
    "name-commonjs-wrappers",
    "rename-simple",
    "rename-smart",
    "rename-unique",
    "mangle",
    "rename-shorthand",
];

fn engine_names() -> BTreeSet<&'static str> {
    let mut names: BTreeSet<&'static str> = all_pass_ctors()
        .iter()
        .chain(untranspile_ctors().iter())
        .map(|ctor| ctor().name())
        .collect();
    names.extend(RENAME_PASSES);
    names.extend([
        UNSTAMP_REMAINING_PASS,
        untwist_deobfuscate::layers::LAYER_PASS,
        untwist_deobfuscate::layers::INLINE_LAYER_PASS,
    ]);
    names
}

#[test]
fn the_table_names_every_pass_the_engine_runs_and_nothing_else() {
    let table: BTreeSet<&str> = PASSES.iter().map(|pass| pass.name).collect();
    assert_eq!(table.len(), PASSES.len(), "a pass is listed twice");
    let engine = engine_names();
    assert_eq!(table, engine, "engine: {engine:#?}");
}

#[test]
fn the_table_marks_exactly_the_unsafe_passes() {
    let unsafe_names: BTreeSet<&str> = all_pass_ctors()
        .iter()
        .chain(untranspile_ctors().iter())
        .map(|ctor| ctor())
        .filter(|pass| pass.safety() == untwist_core::pass::Safety::Unsafe)
        .map(|pass| pass.name())
        .collect();
    let marked: BTreeSet<&str> = PASSES
        .iter()
        .filter(|pass| pass.unsafe_only)
        .map(|pass| pass.name)
        .collect();
    assert_eq!(marked, unsafe_names);
}

#[test]
fn a_known_name_resolves_to_its_static_name_and_an_unknown_one_does_not() {
    assert_eq!(pass_named("dead-code"), Some("dead-code"));
    assert_eq!(pass_named("layers"), Some("layers"));
    assert_eq!(pass_named("no-such-pass"), None);
    let stages: BTreeSet<&str> = PASSES.iter().map(|pass: &PassInfo| pass.stage).collect();
    assert!(stages.iter().all(|stage| !stage.is_empty()));
}
