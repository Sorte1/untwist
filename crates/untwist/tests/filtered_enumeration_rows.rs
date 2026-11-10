use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/filtered_enumeration_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool, mutant: Option<&'static str>) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: mutant.into_iter().collect(),
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

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

fn assert_rows_diverge_under(mutant: &'static str, prefix: &str) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect();
    assert!(!sabotage.is_empty(), "no {prefix} rows");
    let mutated = options(false, Some(mutant));
    let run_mutant = |source: &str| transformed(&mutated, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_mutant)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    for (label, _) in &sabotage {
        assert!(
            names.contains(&label.as_str()),
            "{label} did not diverge from node under the {mutant} mutant"
        );
    }
}

#[test]
fn the_sabotage_rows_diverge_from_node_under_the_filtered_enumeration_mutant() {
    assert_rows_diverge_under("filtered-enumeration", "ref-");
}

#[test]
fn the_rewrite_order_rows_diverge_from_node_under_the_rewrite_order_mutant() {
    assert_rows_diverge_under("filtered-rewrite-order", "ref-order-");
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
