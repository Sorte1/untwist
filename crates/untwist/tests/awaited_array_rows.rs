use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/awaited_array_rows.txt");
const MUTANT: &str = "awaited-arrays";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(mutant: bool) -> Options {
    Options {
        unpack: false,
        jsx: false,
        disabled_passes: if mutant { vec![MUTANT] } else { vec![] },
        ..Options::default()
    }
}

fn transformed(mutant: bool, source: &str) -> String {
    Pipeline::new(options(mutant))
        .run(source)
        .code()
        .to_string()
}

fn machinery_kept(code: &str) -> bool {
    code.contains("function _0x")
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn a_join_over_an_awaited_promise_combinator_keeps_no_machinery() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| {
            machinery_kept(&transformed(false, source)) == label.starts_with("dec-")
        })
        .map(|(label, _)| label)
        .collect();
    assert!(wrong.is_empty(), "decided against the label: {wrong:?}");
}

#[test]
fn the_default_lane_runs_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run = |source: &str| transformed(false, source);
    let modes: [Mode<'_>; 1] = [("default", &run)];
    let wrong = mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_refusal_diverges_from_node_when_every_member_call_is_taken_as_a_string() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let refused: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    let run = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = mismatches(&node, &refused, &modes);
    let same: Vec<&str> = refused
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| {
            !diverged
                .iter()
                .any(|line| line.starts_with(&format!("{label} ")))
        })
        .collect();
    assert!(same.is_empty(), "these still print like node: {same:?}");
}
