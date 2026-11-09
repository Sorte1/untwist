use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_reach_rows.txt");
const PREFIXES: [&str; 3] = ["dec-", "ref-", "guard-"];

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
fn the_default_lane_decodes_exactly_the_dec_rows() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        let code = transformed(&options(false), &source);
        assert_eq!(
            !code.contains("arr0"),
            should_decode,
            "{label}: expected decode={should_decode}:\n{code}"
        );
    }
}

#[test]
fn a_reached_array_is_left_as_written() {
    for (label, source) in rows() {
        if label.starts_with("dec-") {
            continue;
        }
        let code = transformed(&options(false), &source);
        assert!(code.contains("var arr0 = ["), "{label}:\n{code}");
    }
}

#[test]
fn every_row_prints_like_node_in_the_default_lane_and_dec_rows_in_both() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let decoded: Vec<(String, String)> = programs
        .iter()
        .filter(|(label, _)| label.starts_with("dec-"))
        .cloned()
        .collect();
    let safe = options(false);
    let aggressive = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let default_lane: [Mode<'_>; 1] = [("safe", &run_safe)];
    let unsafe_lane: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    let mut wrong = node_rows::mismatches(&node, &programs, &default_lane);
    wrong.extend(node_rows::mismatches(&node, &decoded, &unsafe_lane));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
