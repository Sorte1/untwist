use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/declared_machinery_rows.txt");

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

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("dec-") || label.starts_with("ref-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn machinery_declared_before_the_rotator_decodes_and_the_rest_is_left_in_both_lanes() {
    for lane in [false, true] {
        for (label, source) in rows() {
            let code = transformed(lane, &source);
            let decoded = !code.contains("arr0");
            assert_eq!(
                decoded,
                label.starts_with("dec-"),
                "{label} (unsafe={lane}):\n{code}"
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
    assert!(programs.len() >= 5, "found {} rows", programs.len());
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
