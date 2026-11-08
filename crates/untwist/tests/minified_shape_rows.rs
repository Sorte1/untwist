use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/minified_shape_rows.txt");
const PREFIXES: [&str; 3] = ["dec-", "guard-", "ref-"];

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
    !code.contains("arr0 = ") && !code.contains("function arr0") && !code.contains("dec0")
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
            let should_decode = label.starts_with("dec-");
            let code = transformed(lane, &source);
            assert_eq!(
                decoded(&code),
                should_decode,
                "{label} (unsafe={lane}): expected decode={should_decode}:\n{code}"
            );
        }
    }
}

#[test]
fn both_lanes_run_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
