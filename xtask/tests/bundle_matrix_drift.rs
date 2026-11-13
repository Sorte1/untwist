use std::path::{Path, PathBuf};

use xtask::bundle_matrix::checks::CHECKS;
use xtask::bundle_matrix::manifest::{load, sums, verify};
use xtask::bundle_matrix::observe::LANES;
use xtask::bundle_matrix::EXPECT;
use xtask::obf_matrix::expect::{glob, load as load_rules};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn every_committed_cell_matches_its_checksums_and_has_an_expected_stdout() {
    let root = root();
    let cells = load(&root).unwrap();
    assert!(cells.len() > 100, "{} cells", cells.len());
    let problems = verify(&root, &cells, &sums(&root).unwrap()).unwrap();
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn every_expect_rule_names_a_lane_a_check_and_at_least_one_cell() {
    let root = root();
    let ids: Vec<String> = load(&root).unwrap().iter().map(|c| c.id()).collect();
    for rule in load_rules(&root.join(EXPECT)).unwrap() {
        assert!(CHECKS.contains(&rule.check.as_str()), "{rule:?}");
        assert!(
            LANES.iter().any(|(lane, _)| glob(&rule.cell, lane)),
            "{rule:?}"
        );
        assert!(
            ids.iter().any(|id| glob(&rule.fixture, id)),
            "{rule:?} matches no cell"
        );
    }
}
