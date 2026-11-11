use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/control_flow_switch_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn is_text_row(label: &str) -> bool {
    label.starts_with("text-")
}

#[test]
fn a_dispatcher_whose_bindings_other_code_can_reach_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let default = |source: &str| run(false, source);
    let modes: [Mode<'_>; 1] = [("default", &default)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_unsafe_lane_prints_what_node_prints_where_no_code_text_runs() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| !is_text_row(label))
        .collect();
    let unsafe_lane = |source: &str| run(true, source);
    let modes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
