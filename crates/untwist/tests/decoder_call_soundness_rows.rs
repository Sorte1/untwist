use untwist::{Options, Pipeline};
use untwist_deobfuscate::call_checks::CALL_SOUNDNESS_CHECKS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/decoder_call_soundness_rows.txt");
const MINIMUM_ROWS: usize = 30;

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn rows_labelled(prefix: &str) -> Vec<(String, String)> {
    rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with(prefix))
        .collect()
}

fn options(unsafe_transforms: bool, mutant: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: if mutant {
            vec![CALL_SOUNDNESS_CHECKS]
        } else {
            vec![]
        },
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
    let programs = rows();
    assert!(
        programs.len() >= MINIMUM_ROWS,
        "found {} rows",
        programs.len()
    );
    for (label, _) in programs {
        assert!(
            label.starts_with("dec-")
                || label.starts_with("ref-")
                || label.starts_with("guard-")
                || label.starts_with("probe-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn the_decoding_rows_lose_their_decoder_in_the_safe_lane() {
    let safe = options(false, false);
    for (label, source) in rows_labelled("dec-") {
        let output = transformed(&safe, &source);
        assert!(!output.contains("function dec("), "{label}:\n{output}");
    }
}

#[test]
fn every_row_prints_like_node_in_the_safe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = options(false, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("safe", &run_safe)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_decoding_and_guard_rows_print_like_node_in_the_unsafe_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let kept: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| !label.starts_with("ref-") && !label.starts_with("probe-"))
        .collect();
    let aggressive = options(true, false);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 1] = [("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &kept, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_sabotage_rows_diverge_from_node_without_the_soundness_checks() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = rows_labelled("ref-");
    let mutant = options(false, true);
    let run_mutant = |source: &str| transformed(&mutant, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_mutant)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    for (label, _) in &sabotage {
        assert!(
            names.contains(&label.as_str()),
            "{label} did not diverge from node without the checks"
        );
    }
}
