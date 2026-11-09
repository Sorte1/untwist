use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/storage_call_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(unsafe_transforms: bool, mutant: Option<&'static str>, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: mutant.into_iter().collect(),
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn decoded(mutant: Option<&'static str>, source: &str) -> bool {
    !transformed(false, mutant, source).contains("bQ(")
}

fn labelled(prefix: &str) -> Vec<(String, String)> {
    rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect()
}

#[test]
fn every_row_has_a_known_prefix() {
    for (label, _) in rows() {
        assert!(
            label.starts_with("ref-") || label.starts_with("dec-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| decoded(None, source) != label.starts_with("dec-"))
        .map(|(label, _)| label)
        .collect();
    assert!(wrong.is_empty(), "decided against the label: {wrong:?}");
}

#[test]
fn both_lanes_run_like_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = |source: &str| transformed(false, None, source);
    let modes: [Mode<'_>; 1] = [("default", &safe)];
    let wrong = mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let aggressive = |source: &str| transformed(true, None, source);
    let modes: [Mode<'_>; 1] = [("unsafe", &aggressive)];
    let wrong = mismatches(&node, &labelled("dec-"), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_decoding_row_refuses_without_storage_calls() {
    let decided: Vec<String> = labelled("dec-")
        .into_iter()
        .filter(|(_, source)| decoded(Some("storage-calls"), source))
        .map(|(label, _)| label)
        .collect();
    assert!(
        decided.is_empty(),
        "decoded without storage calls: {decided:?}"
    );
}

#[test]
fn the_sabotage_rows_diverge_from_node_when_storage_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = labelled("ref-trust-");
    assert!(!sabotage.is_empty());
    let mutated = |source: &str| transformed(false, Some("trusted-storage"), source);
    let modes: [Mode<'_>; 1] = [("mutant", &mutated)];
    let diverged = mismatches(&node, &sabotage, &modes);
    let quiet: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| {
            !diverged
                .iter()
                .any(|row| row.split(" (mutant):").next() == Some(*label))
        })
        .collect();
    assert!(
        quiet.is_empty(),
        "did not diverge from node under the trusted-storage mutant: {quiet:?}"
    );
}
