use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/native_map_pending_rows.txt");

const REFUSED_BY_THE_NAMESPACE_SCAN: &[(&str, &str)] = &[(
    "ref-map-pending-rebound",
    "the rebound Map returns Reflect from a function, which the namespace allow-list pins before native-map-pending runs",
)];

fn refused_by_the_namespace_scan(label: &str) -> bool {
    REFUSED_BY_THE_NAMESPACE_SCAN
        .iter()
        .any(|(refused, _)| *refused == label)
}

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
            vec![
                "native-map-pending",
                untwist_deobfuscate::decoded::BUILTIN_ALIAS_CHECKS,
            ]
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

fn decoded(source: &str) -> bool {
    !transformed(&options(false, false), source).contains("console.log(bQ(")
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

fn patches_before_the_decode(label: &str) -> bool {
    !label.starts_with("dec-")
}

#[test]
fn the_default_lane_runs_like_node_and_unsafe_decodes_with_pristine_built_ins() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    assert!(programs.len() >= 10, "found {} rows", programs.len());
    let safe = options(false, false);
    let aggressive = options(true, false);
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&aggressive, source);
    let lanes = node_rows::pristine_under_unsafe(&node, &programs, (&run_safe, &run_unsafe));
    assert!(lanes.wrong.is_empty(), "{}", lanes.wrong.join("\n"));
    let expected: Vec<String> = programs
        .iter()
        .map(|(label, _)| label.clone())
        .filter(|label| patches_before_the_decode(label))
        .collect();
    assert_eq!(lanes.pristine, expected);
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
fn the_sabotage_rows_diverge_from_node_under_the_native_map_pending_mutant() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-") && !refused_by_the_namespace_scan(label))
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

#[test]
fn the_rows_the_namespace_scan_refuses_stay_undecoded_under_the_mutant() {
    let mutant = options(false, true);
    let programs = rows();
    for (label, reason) in REFUSED_BY_THE_NAMESPACE_SCAN {
        let (_, source) = programs
            .iter()
            .find(|(row, _)| row == label)
            .unwrap_or_else(|| panic!("{label} is not a row"));
        assert!(
            transformed(&mutant, source).contains("console.log(bQ("),
            "{label} decoded under the mutant, but {reason}"
        );
    }
}
