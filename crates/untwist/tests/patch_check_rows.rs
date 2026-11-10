use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/patch_check_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

#[test]
fn a_patch_behind_a_rewritten_key_object_or_call_result_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let safe = Options::default();
    let run_safe = |source: &str| Pipeline::new(safe.clone()).run(source).code().to_string();
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
