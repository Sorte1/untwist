use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_single_use_index_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/single_use_decoder_rows.txt");

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_refused_row_diverges_from_node_once_the_index_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-"))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert!(sabotage.len() >= 3, "found {} rows", sabotage.len());
    let run = |source: &str| transformed(source);
    trust_single_use_index_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_single_use_index_for_tests(false);
    assert_eq!(diverged.len(), sabotage.len(), "{diverged:?}");
}
