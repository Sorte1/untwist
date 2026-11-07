use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/unsafe_name_reach_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn decoded(code: &str) -> bool {
    !code.contains("function a()")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            ["dec-", "ref-", "premise-"]
                .iter()
                .any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn the_unsafe_lane_decodes_exactly_the_rows_that_reach_no_machinery_name() {
    for (label, source) in rows() {
        let code = transformed(true, &source);
        assert_eq!(
            decoded(&code),
            !label.starts_with("ref-"),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn both_lanes_print_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 12, "found {} rows", programs.len());
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let default_lane: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &programs, &default_lane);
    let covered: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !label.starts_with("premise-"))
        .collect();
    let unsafe_lane: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &covered, &unsafe_lane));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
