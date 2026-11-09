use untwist::{Options, Pipeline};
use untwist_eval::mutants::{
    ignore_memo_misses_for_tests, ignore_static_uses_of_captured_memos_for_tests,
    trust_memo_key_reads_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/captured_memo_rows.txt");

type Switch = fn(bool);

const SWITCHES: [Switch; 3] = [
    ignore_static_uses_of_captured_memos_for_tests,
    trust_memo_key_reads_for_tests,
    ignore_memo_misses_for_tests,
];

fn sabotage_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-static-a-literal-key-"))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn literal_key_reads_of_a_shadowed_memo_diverge_from_node_once_their_checks_are_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = sabotage_rows();
    assert!(!sabotage.is_empty(), "no ref-static-a-literal-key- rows");
    SWITCHES.iter().for_each(|switch| switch(true));
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    SWITCHES.iter().for_each(|switch| switch(false));
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    let kept: Vec<&String> = sabotage
        .iter()
        .map(|(label, _)| label)
        .filter(|label| !names.contains(&label.as_str()))
        .collect();
    assert!(
        kept.is_empty(),
        "these rows still print like node under the mutants: {kept:?}"
    );
}
