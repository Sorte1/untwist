use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip};

const ROWS: &str = include_str!("data/key_judged_rows.txt");

const PATCHED_BUILT_IN_ROWS: [&str; 22] = [
    "k1-n9",
    "k2-n10",
    "k5-n1",
    "k5-n2",
    "k5-n3",
    "k5-n4",
    "k5-n5",
    "k5-n6",
    "k5-n7",
    "k5-n8",
    "k5-n11",
    "k5-sib-01",
    "k5-sib-02",
    "k5-sib-03",
    "k5-sib-04",
    "k5-sib-05",
    "k5-sib-06",
    "k5-sib-07",
    "k5-sib-08",
    "k5-sib-09",
    "k5-sib-10",
    "k5-sib-11",
];

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
fn a_global_property_judged_by_key_runs_like_node_by_default_and_as_pristine_built_ins_under_unsafe(
) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 200, "found {} rows", programs.len());
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
