use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/opaque_branch_rows.txt");
const MUTANT: &str = "opaque-branches";

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(mutant: bool, source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        disabled_passes: if mutant { vec![MUTANT] } else { vec![] },
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
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
        .filter(|(label, source)| {
            let decoded = !transformed(false, source).contains("console.log(bQ(");
            decoded != label.starts_with("dec-")
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
fn every_decoding_row_refuses_when_no_branch_is_chosen() {
    let decided: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| {
            label.starts_with("dec-") && !transformed(true, source).contains("console.log(bQ(")
        })
        .map(|(label, _)| label)
        .collect();
    assert!(
        decided.is_empty(),
        "decoded without the branch choice: {decided:?}"
    );
}
