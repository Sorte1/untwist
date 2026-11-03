use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip};

const ROWS: &str = include_str!("data/fail_closed_rows.txt");

const PATCHED_BUILT_IN_ROWS: [&str; 4] = ["g-d1", "g-e5", "g-r1", "g-r2"];

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
fn every_fail_closed_shape_runs_like_node_by_default_and_as_pristine_built_ins_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 120, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let rows = node_rows::pristine_under_unsafe(&node, &programs, (&run_safe, &run_unsafe));
    assert!(rows.wrong.is_empty(), "{}", rows.wrong.join("\n"));
    assert_eq!(rows.pristine, PATCHED_BUILT_IN_ROWS);
}
