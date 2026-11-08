use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::drop_any_settled_right_of_if_tests_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/dead_member_rows.txt");

fn guard_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("guard-"))
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
fn guard_rows_diverge_once_any_settled_right_of_a_test_is_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let guards = guard_rows();
    assert!(!guards.is_empty(), "found no guard rows");
    drop_any_settled_right_of_if_tests_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &guards, &modes);
    drop_any_settled_right_of_if_tests_for_tests(false);
    assert_eq!(diverged.len(), guards.len(), "{}", diverged.join("\n"));
}
