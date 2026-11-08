use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/hoisted_between_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
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
    !code.contains("arr0 = [")
}

#[test]
fn each_rows_decode_decision_matches_its_label_in_both_lanes() {
    for (label, source) in rows() {
        assert!(
            label.starts_with("dec-") || label.starts_with("keep-"),
            "{label}"
        );
        for lane in [false, true] {
            let code = transformed(lane, &source);
            assert_eq!(
                decoded(&code),
                label.starts_with("dec-"),
                "{label} (unsafe={lane}):\n{code}"
            );
        }
    }
}

#[test]
fn every_row_prints_like_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
