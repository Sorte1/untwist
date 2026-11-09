use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_deferred_legacy_reach_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_reach_rows.txt");

fn refused_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-"))
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
fn every_reached_row_diverges_from_node_once_the_deferred_check_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = refused_rows();
    assert!(sabotage.len() >= 4, "found {} rows", sabotage.len());
    trust_deferred_legacy_reach_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_deferred_legacy_reach_for_tests(false);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    let kept: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !names.contains(label))
        .collect();
    assert!(
        kept.is_empty(),
        "these rows still print like node under the mutant: {kept:?}"
    );
}
