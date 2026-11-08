use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_array_rows.txt");
const PREFIXES: [&str; 5] = ["dec-", "unsafe-dec-", "ref-", "guard-", "premise-"];
const UNSAFE_PREMISE: &str = "ref-premise-";
const SELF_DEFENDING: &str = "self-defending";

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
    !code.contains("arr0 = ") && !code.contains("function arr0")
}

fn unsafe_premise(label: &str) -> bool {
    label.starts_with(UNSAFE_PREMISE) || label.starts_with("premise-")
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
            if lane && unsafe_premise(&label) {
                continue;
            }
            let should_decode =
                label.starts_with("dec-") || (lane && label.starts_with("unsafe-dec-"));
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
        "ref-premise-shift-patched-before-a-rotated-function",
        "ref-premise-shift-defined-before-a-rotated-function",
        "ref-premise-push-patched-by-a-called-function",
        "ref-premise-rotated-after-a-patch-in-its-function",
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
    let printed_as_written: Vec<(String, String)> = programs
        .iter()
        .filter(|(label, _)| !label.contains(SELF_DEFENDING))
        .cloned()
        .collect();
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let mut wrong = node_rows::mismatches(&node, &printed_as_written, &modes);
    let covered: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| !unsafe_premise(label))
        .collect();
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    wrong.extend(node_rows::mismatches(&node, &covered, &modes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
