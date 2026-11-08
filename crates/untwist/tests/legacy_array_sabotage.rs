use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    drop_legacy_array_guards_for_tests, trust_legacy_rotation_patches_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_array_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_refused_row_diverges_from_node_once_the_guards_are_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    assert!(sabotage.len() >= 4, "found {} rows", sabotage.len());
    let run = |source: &str| transformed(source);
    drop_legacy_array_guards_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    drop_legacy_array_guards_for_tests(false);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    let missing: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !names.contains(label))
        .collect();
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}

#[test]
fn a_rotation_behind_a_patched_built_in_diverges_from_node_once_patches_are_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let patched = [
        "ref-premise-shift-patched-before-a-rotated-function",
        "ref-premise-shift-defined-before-a-rotated-function",
        "ref-premise-push-patched-by-a-called-function",
    ];
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| patched.contains(&label.as_str()))
        .collect();
    assert_eq!(sabotage.len(), patched.len());
    let run = |source: &str| transformed(source);
    trust_legacy_rotation_patches_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_legacy_rotation_patches_for_tests(false);
    assert_eq!(diverged.len(), patched.len(), "{diverged:?}");
}
