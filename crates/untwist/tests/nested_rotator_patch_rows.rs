use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/nested_rotator_patch_rows.txt");

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
    !code.contains("arr0")
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
fn the_default_lane_decodes_a_nested_rotator_only_when_nothing_replaces_its_built_ins() {
    for (label, source) in rows() {
        let code = transformed(false, &source);
        assert_eq!(
            decoded(&code),
            label.starts_with("dec-"),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn the_unsafe_lane_keeps_its_pristine_built_ins_premise() {
    for (label, source) in rows() {
        let code = transformed(true, &source);
        assert!(decoded(&code), "{label}:\n{code}");
    }
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run = |source: &str| transformed(false, source);
    let modes: [Mode<'_>; 1] = [("default", &run)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
