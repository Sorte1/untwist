use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/instance_receiver_rows.txt");

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
            vec![
                "instance-receivers",
                untwist_deobfuscate::decoded::BUILTIN_ALIAS_CHECKS,
            ]
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
    let code = transformed(&options(false, false), source);
    !code.contains("console.log(bQ(") && !code.contains("function aQ(")
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

fn patches_before_the_decode(source: &str) -> bool {
    source.contains("'patched'")
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 10, "found {} rows", programs.len());
    let safe = options(false, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_unsafe_lane_decodes_the_patching_rows_with_pristine_built_ins() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(_, source)| patches_before_the_decode(source))
        .collect();
    let safe = options(false, false);
    let aggressive = options(true, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let lanes = node_rows::pristine_under_unsafe(&node, &programs, (&run_safe, &run_unsafe));
    assert!(lanes.wrong.is_empty(), "{}", lanes.wrong.join("\n"));
    let expected: Vec<String> = programs.iter().map(|(label, _)| label.clone()).collect();
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
fn the_sabotage_rows_diverge_from_node_under_the_instance_receivers_mutant() {
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
