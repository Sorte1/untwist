use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/single_use_decoder_rows.txt");
const PREFIXES: [&str; 4] = ["dec-", "ref-", "guard-", "premise-"];

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
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            PREFIXES.iter().any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn a_single_use_decoder_decodes_only_when_its_index_is_its_own() {
    for lane in [false, true] {
        for (label, source) in rows() {
            if lane && label.starts_with("premise-") {
                continue;
            }
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
fn both_lanes_run_like_node_outside_the_unsafe_premises() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 6, "found {} rows", programs.len());
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &programs, &modes);
    let covered: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !label.starts_with("premise-"))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &covered, &modes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
