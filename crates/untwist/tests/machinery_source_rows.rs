use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/machinery_source_rows.txt");
const MUTANT: &str = "machinery-source-bound";

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
fn every_row_has_a_known_prefix_and_each_encoding_is_covered() {
    let programs = rows();
    for (label, _) in &programs {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
    for encoding in ["-plain", "-base64", "-rc4"] {
        assert!(
            programs
                .iter()
                .any(|(label, _)| label.starts_with("dec-") && label.ends_with(encoding)),
            "no dec- row for {encoding}"
        );
    }
    assert!(programs.iter().any(|(label, _)| label.starts_with("ref-")));
}

#[test]
fn a_decoders_own_cache_does_not_keep_the_machinery_but_an_outside_source_does() {
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
fn every_refusal_diverges_from_node_when_outside_sources_are_exempt_too() {
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
    assert_eq!(diverged.len(), refused.len(), "{}", diverged.join("\n"));
}

#[test]
fn a_source_outside_the_sites_check_keeps_the_machinery_under_the_mutant_too() {
    let still_removed: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| {
            label.starts_with("guard-") && !machinery_kept(&transformed(true, source))
        })
        .map(|(label, _)| label)
        .collect();
    assert!(still_removed.is_empty(), "{still_removed:?}");
}
