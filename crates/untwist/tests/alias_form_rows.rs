use untwist::{Options, Pipeline};
use untwist_deobfuscate::wrappers::ALIAS_FORM_CHECKS;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/alias_form_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool, mutant: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        disabled_passes: if mutant {
            vec![ALIAS_FORM_CHECKS]
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

fn machinery_removed(options: &Options, source: &str) -> bool {
    !transformed(options, source).contains("function a(")
}

#[test]
fn every_row_has_a_known_prefix() {
    let programs = rows();
    assert!(programs.len() >= 20, "found {} rows", programs.len());
    for (label, _) in programs {
        assert!(
            label.starts_with("dec-") || label.starts_with("ref-") || label.starts_with("guard-"),
            "unexpected row label {label}"
        );
    }
}

#[test]
fn each_rows_decode_decision_matches_its_label_in_both_lanes() {
    for unsafe_transforms in [false, true] {
        let lane = options(unsafe_transforms, false);
        for (label, source) in rows() {
            let expected = label.starts_with("dec-");
            assert_eq!(
                machinery_removed(&lane, &source),
                expected,
                "{label} (unsafe={unsafe_transforms}):\n{}",
                transformed(&lane, &source)
            );
        }
    }
}

#[test]
fn every_row_prints_like_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = options(false, false);
    let aggressive = options(true, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let modes: [Mode<'_>; 2] = [("safe", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_sabotage_rows_diverge_from_node_under_the_alias_form_checks_mutant() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
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
            "{label} did not diverge from node under the mutant"
        );
    }
}
