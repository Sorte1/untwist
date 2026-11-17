use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_root_alias_reach_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const REFUSED: &[&str] = &[
    "ka03", "ka15", "kw04", "ko01", "al6", "wf1", "wf6", "kd01", "ke01", "kf01",
];

fn default_lane(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn every_reached_alias_row_diverges_from_node_once_the_reach_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/global_alias_reach");
    let sabotage: Vec<(String, String)> = REFUSED
        .iter()
        .map(|label| {
            let source = std::fs::read_to_string(data.join(format!("{label}.js"))).unwrap();
            (label.to_string(), source)
        })
        .collect();
    trust_root_alias_reach_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &default_lane)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_root_alias_reach_for_tests(false);
    assert_eq!(diverged.len(), sabotage.len(), "{diverged:?}");
}
