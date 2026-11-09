use untwist::{Options, Pipeline};
use untwist_eval::mutants::{ignore_memo_misses_for_tests, trust_memo_key_reads_for_tests};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/memo_key_rows.txt");

type Switch = fn(bool);

const MUTANTS: [(&str, &[Switch]); 2] = [
    (
        "ref-key-",
        &[trust_memo_key_reads_for_tests, ignore_memo_misses_for_tests],
    ),
    ("ref-miss-", &[ignore_memo_misses_for_tests]),
];

fn rows_with(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
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

fn kept_under(node: &std::path::Path, prefix: &str, switches: &[Switch]) -> Vec<String> {
    let sabotage = rows_with(prefix);
    assert!(!sabotage.is_empty(), "no {prefix} rows");
    switches.iter().for_each(|switch| switch(true));
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(node, &sabotage, &modes);
    switches.iter().for_each(|switch| switch(false));
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    sabotage
        .into_iter()
        .map(|(label, _)| label)
        .filter(|label| !names.contains(&label.as_str()))
        .collect()
}

#[test]
fn memo_key_rows_diverge_from_node_once_their_checks_are_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let kept: Vec<String> = MUTANTS
        .iter()
        .flat_map(|(prefix, switches)| kept_under(&node, prefix, switches))
        .collect();
    assert!(
        kept.is_empty(),
        "these rows still print like node under their mutant: {kept:?}"
    );
}
