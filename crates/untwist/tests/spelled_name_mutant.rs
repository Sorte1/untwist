use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::drop_decoder_name_watch_for_tests;
use untwist_testing::js_files_in;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const SABOTAGE: &[&str] = &[
    "decoded-key-read.js",
    "global-key-read.js",
    "pattern-key.js",
    "this-static-read.js",
];

fn default_lane(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn every_decoder_name_row_diverges_from_node_once_the_decoder_names_are_not_watched() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let rows: Vec<(String, String)> = js_files_in("fixtures/spelled-name")
        .into_iter()
        .filter(|(label, _)| SABOTAGE.iter().any(|name| label.ends_with(name)))
        .collect();
    assert_eq!(rows.len(), SABOTAGE.len());
    drop_decoder_name_watch_for_tests(true);
    let runs: [Mode<'_>; 1] = [("mutant", &default_lane)];
    let wrong = node_rows::mismatches(&node, &rows, &runs);
    drop_decoder_name_watch_for_tests(false);
    let missing: Vec<&str> = rows
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !wrong.iter().any(|row| row.starts_with(label)))
        .collect();
    assert!(
        missing.is_empty(),
        "no divergence under the mutant: {missing:?}"
    );
}
