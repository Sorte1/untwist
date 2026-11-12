#[path = "common/filtered_rows.rs"]
mod filtered_rows;

use filtered_rows::{options, rows, transformed};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

fn decoded(source: &str) -> bool {
    !transformed(&options(false, None), source).contains("bQ(")
}

#[test]
fn every_row_has_a_known_prefix() {
    let programs = rows();
    assert!(programs.len() >= 6, "found {} rows", programs.len());
    for (label, _) in programs {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        assert_eq!(
            decoded(&source),
            should_decode,
            "{label}: decoded={} but expected decode={should_decode}",
            decoded(&source)
        );
    }
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = options(false, None);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_decoding_rows_also_run_like_node_in_the_unsafe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let decoding: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("dec-"))
        .collect();
    let aggressive = options(true, None);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &decoding, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const LOOPING_FILTER: &str = "(function () { var that = {}; var pred = function (s) { for (var i = 0; i < 1e9; i++) {} return s.charCodeAt(0) == 122; }; var b; for (var k in that) { if (pred(k)) { b = k; break; } } var r = that[b]; r(); })();\n";

#[test]
fn many_filter_loops_with_looping_predicates_are_judged_in_bounded_time() {
    let source = format!(
        "{}console.log('abc'.charAt(0));\n",
        LOOPING_FILTER.repeat(800)
    );
    let started = std::time::Instant::now();
    let code = transformed(&options(false, None), &source);
    let elapsed = started.elapsed();
    assert!(code.contains("console.log"), "{}", code.len());
    assert!(
        elapsed < std::time::Duration::from_secs(30),
        "a {}-byte program took {elapsed:?}",
        source.len()
    );
}
