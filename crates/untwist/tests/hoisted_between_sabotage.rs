use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    move_inert_statements_into_rotators_for_tests, trust_alias_root_names_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/hoisted_between_rows.txt");

fn row(label: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(name, _)| *name == label)
        .map(|(name, program)| (name.to_string(), program.to_string()))
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

fn diverges(node: &std::path::Path, label: &str, switch: fn(bool)) -> bool {
    let rows = row(label);
    assert_eq!(rows.len(), 1, "{label}");
    switch(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(node, &rows, &modes);
    switch(false);
    diverged.len() == 1
}

#[test]
fn the_rows_diverge_from_node_once_their_check_is_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    assert!(diverges(
        &node,
        "dec-checksum-loop-after-a-hoisted-decoder-and-its-alias",
        move_inert_statements_into_rotators_for_tests
    ));
    assert!(diverges(
        &node,
        "keep-alias-read-where-the-array-name-is-shadowed",
        trust_alias_root_names_for_tests
    ));
}
