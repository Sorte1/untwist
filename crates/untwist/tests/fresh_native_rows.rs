use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/fresh_native_rows.txt");
const PREFIXES: [&str; 3] = ["dec-", "ref-", "sab-"];

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn with_prefix(prefix: &str) -> Vec<(String, String)> {
    rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn every_row_has_a_known_prefix() {
    let all = rows();
    assert!(all.len() >= 18, "found {} rows", all.len());
    for (label, _) in all {
        assert!(
            PREFIXES.iter().any(|prefix| label.starts_with(prefix)),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn the_safe_lane_drops_the_machinery_of_exactly_the_dec_rows() {
    for (label, source) in rows() {
        let clean = label.starts_with("dec-");
        let code = transformed(&options(false), &source);
        assert_eq!(
            !code.contains("function aQ"),
            clean,
            "{label}: expected clean={clean}:\n{code}"
        );
    }
}

#[test]
fn every_row_prints_like_node_in_the_safe_lane_and_dec_rows_in_both() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = options(false);
    let aggressive = options(true);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let safe_lane: [Mode<'_>; 1] = [("safe", &run_safe)];
    let both: [Mode<'_>; 2] = [("safe", &run_safe), ("unsafe", &run_unsafe)];
    let mut wrong = node_rows::mismatches(&node, &rows(), &safe_lane);
    wrong.extend(node_rows::mismatches(&node, &with_prefix("dec-"), &both));
    assert!(wrong.is_empty(), "{wrong:#?}");
}
