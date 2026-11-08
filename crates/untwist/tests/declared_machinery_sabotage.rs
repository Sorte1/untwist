use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    drop_early_read_guards_for_tests, trust_declared_machinery_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/declared_machinery_rows.txt");

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

fn diverged(node: &std::path::Path, rows: &[(String, String)]) -> Vec<String> {
    let run = |source: &str| transformed(source);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    node_rows::mismatches(node, rows, &modes)
        .iter()
        .filter_map(|row| row.split(" (mutant):").next().map(String::from))
        .collect()
}

#[test]
fn every_refused_row_diverges_from_node_once_declared_machinery_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = refused_rows();
    assert!(sabotage.len() >= 2, "found {} rows", sabotage.len());
    drop_early_read_guards_for_tests(true);
    let held = diverged(&node, &sabotage);
    trust_declared_machinery_for_tests(true);
    let broken = diverged(&node, &sabotage);
    trust_declared_machinery_for_tests(false);
    drop_early_read_guards_for_tests(false);
    assert!(
        held.is_empty(),
        "diverged with only the early-read guards dropped: {held:?}"
    );
    let missing: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !broken.iter().any(|name| name == label))
        .collect();
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}
