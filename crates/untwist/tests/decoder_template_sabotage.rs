use untwist::{Options, Pipeline};
use untwist_eval::mutants::{
    leave_memo_miss_results_unchecked_for_tests, recognize_gates_in_value_position_for_tests,
    trust_rotator_wrappers_by_declaration_for_tests, trust_state_left_by_gates_for_tests,
    trust_template_arity_for_tests, trust_template_parameters_for_tests,
    trust_template_prefix_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/decoder_template_rows.txt");

type Switch = fn(bool);

const MUTANTS: [(&str, &[Switch]); 7] = [
    ("ref-param-", &[trust_template_parameters_for_tests]),
    ("ref-arity-", &[trust_template_arity_for_tests]),
    ("ref-prefix-", &[trust_template_prefix_for_tests]),
    (
        "ref-value-gate-",
        &[recognize_gates_in_value_position_for_tests],
    ),
    (
        "ref-miss-result-",
        &[leave_memo_miss_results_unchecked_for_tests],
    ),
    ("ref-gate-state-", &[trust_state_left_by_gates_for_tests]),
    (
        "dec-wrapper-",
        &[trust_rotator_wrappers_by_declaration_for_tests],
    ),
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
fn decoder_template_rows_diverge_from_node_once_their_checks_are_dropped() {
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
