use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const ROWS: &str = include_str!("data/name_key_rows.txt");
const MUTANT: &str = "inert-name-keys";

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
fn a_loop_that_only_reads_the_global_names_decodes_and_keeps_the_machinery() {
    let wrong: Vec<String> = rows()
        .into_iter()
        .filter(|(label, source)| {
            let code = transformed(false, source);
            let decoded = !code.contains("console.log(bQ(");
            let kept = code.contains("function aQ");
            decoded != label.starts_with("dec-") || (decoded && !kept)
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
fn every_decoding_row_diverges_from_node_when_the_names_it_reads_are_not_observed() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let decoding: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("dec-"))
        .collect();
    let run = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = mismatches(&node, &decoding, &modes);
    let same: Vec<&str> = decoding
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
