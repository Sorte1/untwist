use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/remaining_shape_rows.txt");

fn rows() -> Vec<(String, String, String)> {
    ROWS.lines()
        .filter_map(|line| {
            let (label, rest) = line.split_once('|')?;
            let (marker, program) = rest.split_once('|')?;
            Some((label.to_string(), marker.to_string(), program.to_string()))
        })
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

#[test]
fn each_dec_row_loses_its_marker_in_both_lanes() {
    for (label, marker, source) in rows() {
        assert!(
            label.starts_with("dec-") || label.starts_with("keep-"),
            "{label}"
        );
        if !label.starts_with("dec-") {
            continue;
        }
        for lane in [false, true] {
            let code = transformed(lane, &source);
            assert!(!code.contains(&marker), "{label} (unsafe={lane}):\n{code}");
        }
    }
}

#[test]
fn every_row_prints_like_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .into_iter()
        .map(|(label, _, program)| (label, program))
        .collect();
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
