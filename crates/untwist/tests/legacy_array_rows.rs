use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::drop_legacy_array_guards_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_array_rows.txt");
const PREFIXES: [&str; 3] = ["dec-", "ref-", "guard-"];
const UNSAFE_PREMISE: &str = "ref-premise-";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains("arr0")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            PREFIXES.iter().any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label_in_both_lanes() {
    for lane in [false, true] {
        for (label, source) in rows() {
            if lane && label.starts_with(UNSAFE_PREMISE) {
                continue;
            }
            let should_decode = label.starts_with("dec-");
            let code = transformed(&options(lane), &source);
            assert_eq!(
                decoded(&code),
                should_decode,
                "{label} (unsafe={lane}): expected decode={should_decode}:\n{code}"
            );
        }
    }
}

#[test]
fn the_unsafe_lane_decodes_the_rows_its_premises_cover() {
    let covered = [
        "ref-premise-global-key-read",
        "ref-premise-push-patched-first",
    ];
    for (label, source) in rows() {
        if !covered.contains(&label.as_str()) {
            continue;
        }
        let code = transformed(&options(true), &source);
        assert!(decoded(&code), "{label}:\n{code}");
    }
}

#[test]
fn a_refused_array_is_left_as_written() {
    for (label, source) in rows() {
        if !label.starts_with("ref-") {
            continue;
        }
        let code = transformed(&options(false), &source);
        assert!(code.contains("var arr0 = ["), "{label}:\n{code}");
    }
}

#[test]
fn both_lanes_run_like_node_outside_the_unsafe_premises() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 15, "found {} rows", programs.len());
    let safe = options(false);
    let aggressive = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &programs, &modes);
    let covered: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !label.starts_with(UNSAFE_PREMISE))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &covered, &modes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_refused_row_diverges_from_node_once_the_guards_are_dropped() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    assert!(sabotage.len() >= 4, "found {} rows", sabotage.len());
    let safe = options(false);
    let run = |source: &str| transformed(&safe, source);
    drop_legacy_array_guards_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    drop_legacy_array_guards_for_tests(false);
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
