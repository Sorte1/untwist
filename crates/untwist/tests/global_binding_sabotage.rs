use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_global_binding_writes_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/global_binding_rows.txt");

fn sabotage_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-") || label.starts_with("dec-observed-"))
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
fn every_sabotage_row_diverges_from_node_once_global_writes_to_top_level_bindings_are_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = sabotage_rows();
    assert!(
        sabotage.len() >= 20,
        "found {} sabotage rows",
        sabotage.len()
    );
    trust_global_binding_writes_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_global_binding_writes_for_tests(false);
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
