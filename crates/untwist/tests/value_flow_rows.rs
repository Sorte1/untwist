use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/value_flow_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool, mutant: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: if mutant {
            vec!["value-flow-sinks"]
        } else {
            vec![]
        },
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn decoded(source: &str) -> bool {
    !transformed(&options(false, false), source).contains("bQ(")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
}

fn patches_before_the_decode(label: &str) -> bool {
    label.starts_with("ref-") || label.starts_with("guard-closed-eval-")
}

#[test]
fn the_default_lane_runs_like_node_and_unsafe_decodes_with_pristine_built_ins() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 20, "found {} rows", programs.len());
    let safe = options(false, false);
    let aggressive = options(true, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let lanes = node_rows::pristine_under_unsafe(&node, &programs, (&run_safe, &run_unsafe));
    assert!(lanes.wrong.is_empty(), "{}", lanes.wrong.join("\n"));
    let expected: Vec<String> = programs
        .iter()
        .map(|(label, _)| label.clone())
        .filter(|label| patches_before_the_decode(label))
        .collect();
    assert_eq!(lanes.pristine, expected);
}

#[test]
fn each_rows_decode_decision_matches_its_label() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        assert_eq!(
            decoded(&source),
            should_decode,
            "{label}: decoded={} but expected decode={should_decode}",
            decoded(&source)
        );
    }
}

#[test]
fn the_sabotage_rows_diverge_from_node_under_the_value_flow_sinks_mutant() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    let mutant = options(false, true);
    let run_mutant = |source: &str| transformed(&mutant, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_mutant)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
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
