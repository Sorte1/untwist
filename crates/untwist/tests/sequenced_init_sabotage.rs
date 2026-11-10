use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    ignore_declarator_writes_for_tests, trust_annex_b_writes_for_tests,
    trust_nested_literal_writes_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/sequenced_external_rows.txt");

fn rows_with(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
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

fn assert_rows_diverge(prefix: &str, switch: fn(bool)) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = rows_with(prefix);
    assert!(!sabotage.is_empty(), "found no {prefix} rows");
    switch(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    switch(false);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    for (label, _) in &sabotage {
        assert!(
            names.contains(&label.as_str()),
            "{label} did not diverge from node under the mutant"
        );
    }
}

#[test]
fn initialiser_and_nested_write_rows_diverge_from_node_once_their_check_is_dropped() {
    assert_rows_diverge("ref-init-", |on| {
        ignore_declarator_writes_for_tests(on);
        trust_annex_b_writes_for_tests(on);
    });
    assert_rows_diverge("ref-nested-", trust_nested_literal_writes_for_tests);
}
