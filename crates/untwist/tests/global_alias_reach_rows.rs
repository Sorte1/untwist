use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const WRITTEN_IN_BOTH_LANES: &[&str] = &[
    "ka03", "ka04", "ka05", "ka07", "ka11", "ka12", "ka15", "ka16", "ka17", "kl03", "ko01", "kw01",
    "kw02", "kw03", "kw04", "al6", "al7", "iife", "wf0", "wf1", "wf2", "wf4",
];

const DEFAULT_LANE_ONLY: &[&str] = &[
    "ka06", "ka08", "ka09", "ka10", "kw05", "al3", "al4", "wf3", "wf5", "wf6", "ka21", "ka23",
    "kd01", "kd02", "kd08", "kd16", "kd17", "kd18", "ke01", "ke03", "ke04", "ke06", "kf01", "kf02",
    "kf03", "kf06", "kf07", "kf08", "kf09", "kf10", "kf12", "kw06", "kw07", "ko02", "mp01", "mp04",
    "mp05",
];

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/global_alias_reach")
}

fn source(label: &str) -> String {
    std::fs::read_to_string(data().join(format!("{label}.js"))).unwrap()
}

fn programs(labels: &[&str]) -> Vec<(String, String)> {
    labels
        .iter()
        .map(|label| (label.to_string(), source(label)))
        .collect()
}

fn run(unsafe_transforms: bool, rename: RenameMode, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        rename,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_row_prints_what_node_prints_in_the_safe_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let labels: Vec<&str> = WRITTEN_IN_BOTH_LANES
        .iter()
        .chain(DEFAULT_LANE_ONLY)
        .copied()
        .collect();
    let default = |source: &str| run(false, RenameMode::None, source);
    let smart = |source: &str| run(false, RenameMode::Smart, source);
    let lanes: [Mode<'_>; 2] = [("default", &default), ("rename smart", &smart)];
    let wrong = node_rows::mismatches(&node, &programs(&labels), &lanes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_spelled_global_write_of_an_alias_or_wrapper_is_judged_under_unsafe_too() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let unsafe_lane = |source: &str| run(true, RenameMode::None, source);
    let lanes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs(WRITTEN_IN_BOTH_LANES), &lanes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn aliases_and_wrappers_nothing_reaches_are_still_inlined_and_decoded() {
    for unsafe_transforms in [false, true] {
        let wrapped = run(unsafe_transforms, RenameMode::None, &source("wf0"));
        assert!(wrapped.contains("\"hello, \""), "{wrapped}");
        assert!(!wrapped.contains("_0x463d34"), "{wrapped}");
        let unread = run(unsafe_transforms, RenameMode::None, &source("iife"));
        assert!(!unread.contains("_0x5258"), "{unread}");
        let refused = run(unsafe_transforms, RenameMode::None, &source("al7"));
        assert!(refused.contains("_0x44e3e4"), "{refused}");
    }
}

fn partly_inlined_layered_program() -> String {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/obfuscator-relayer/preset-default-n2--greeting-1.js");
    let source = std::fs::read_to_string(fixture).unwrap();
    let big = format!("'{}'", "q".repeat(1_000_000));
    let mut program = source.replacen("'hello,\\x20'", &big, 1);
    program.push_str("\nconsole.log(_0x4006(0x15e));\n");
    for _ in 0..20 {
        program.push_str("console.log(_0x4006(0x15d).length);\n");
    }
    program
}

#[test]
fn a_speculated_unit_whose_values_are_only_partly_inlined_keeps_its_obligation() {
    let program = partly_inlined_layered_program();
    assert!(program.contains(&"q".repeat(1_000_000)));
    let output = run(false, RenameMode::None, &program);
    let inlined = output.contains("console.log(\"qqqq");
    let kept = output.contains("_0x4006(349)") || output.contains("_0x4006(0x15d)");
    assert!(!(inlined && kept), "the unit was decoded only in part");
}
