use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/unsafe_reach_rows.txt");

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
fn code_text_and_with_reach_prints_what_node_prints_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 13, "found {} rows", programs.len());
    let default = |source: &str| run(false, RenameMode::None, source);
    let unsafe_lane = |source: &str| run(true, RenameMode::None, source);
    let unsafe_simple = |source: &str| run(true, RenameMode::Simple, source);
    let unsafe_mangle = |source: &str| run(true, RenameMode::Mangle, source);
    let unsafe_smart = |source: &str| run(true, RenameMode::Smart, source);
    let modes: [Mode<'_>; 5] = [
        ("default", &default),
        ("unsafe", &unsafe_lane),
        ("unsafe rename simple", &unsafe_simple),
        ("unsafe rename mangle", &unsafe_mangle),
        ("unsafe rename smart", &unsafe_smart),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
