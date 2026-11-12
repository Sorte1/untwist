#[path = "common/storage_rows.rs"]
mod storage_rows;

use storage_rows::{labelled, rows, transformed};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

fn decoded(mutant: Option<&'static str>, source: &str) -> bool {
    !transformed(false, mutant, source).contains("bQ(")
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
