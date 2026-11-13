use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/anonymous_name_rows.txt");

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

#[test]
fn an_anonymous_function_behind_a_comma_keeps_its_empty_name_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 9, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
