use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/blocked_hoist_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
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
fn a_blocked_hoist_prints_what_node_prints_in_every_lane_its_premises_cover() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 30, "found {} rows", programs.len());
    let default = |source: &str| run(false, RenameMode::None, source);
    let smart = |source: &str| run(false, RenameMode::Smart, source);
    let mangle = |source: &str| run(false, RenameMode::Mangle, source);
    let simple = |source: &str| run(false, RenameMode::Simple, source);
    let safe_lanes: [Mode<'_>; 4] = [
        ("default", &default),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
    ];
    let mut wrong = node_rows::mismatches(&node, &programs, &safe_lanes);
    let (direct_eval, static_code): (Vec<_>, Vec<_>) = programs
        .into_iter()
        .partition(|(_, program)| node_rows::runs_dynamic_code(program));
    assert_eq!(direct_eval.len(), 4, "{direct_eval:?}");
    let unsafe_lane = |source: &str| run(true, RenameMode::None, source);
    let unsafe_simple = |source: &str| run(true, RenameMode::Simple, source);
    let unsafe_lanes: [Mode<'_>; 2] = [
        ("unsafe", &unsafe_lane),
        ("unsafe rename simple", &unsafe_simple),
    ];
    wrong.extend(node_rows::mismatches(&node, &static_code, &unsafe_lanes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
