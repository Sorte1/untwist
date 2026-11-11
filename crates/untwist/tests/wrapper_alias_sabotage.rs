use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::drop_wrapper_guards_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/wrapper_alias_rows.txt");
const SAFE_WITHOUT_GUARDS: [&str; 1] = ["guard-wrapper-in-sloppy-block"];

fn guard_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("guard-") && !SAFE_WITHOUT_GUARDS.contains(label))
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
fn every_guard_row_diverges_from_node_once_the_wrapper_guards_are_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = guard_rows();
    assert!(sabotage.len() >= 3, "found {} guard rows", sabotage.len());
    drop_wrapper_guards_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    drop_wrapper_guards_for_tests(false);
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
