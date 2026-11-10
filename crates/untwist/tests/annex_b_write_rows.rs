use std::sync::{Mutex, PoisonError};

use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_annex_b_writes_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

static SERIAL: Mutex<()> = Mutex::new(());

const ROWS: &str = include_str!("data/annex_b_write_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_block_function_write_is_seen_in_both_lanes() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(node) = node_or_skip() else {
        return;
    };
    let default = |source: &str| run(false, source);
    let unsafe_lane = |source: &str| run(true, source);
    let modes: [Mode<'_>; 2] = [("default", &default), ("unsafe", &unsafe_lane)];
    let wrong = node_rows::mismatches(&node, &rows(), &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_row_diverges_from_node_once_the_write_is_trusted_away() {
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    trust_annex_b_writes_for_tests(true);
    let default = |source: &str| run(false, source);
    let modes: [Mode<'_>; 1] = [("mutant", &default)];
    let diverged = node_rows::mismatches(&node, &programs, &modes);
    trust_annex_b_writes_for_tests(false);
    for (label, _) in &programs {
        assert!(
            diverged
                .iter()
                .any(|row| row.starts_with(&format!("{label} (mutant):"))),
            "{label} did not diverge from node under the mutant"
        );
    }
}
