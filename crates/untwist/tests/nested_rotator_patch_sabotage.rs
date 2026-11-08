use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_nested_rotator_builtins_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/nested_rotator_patch_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_refused_row_diverges_from_node_once_rotator_names_are_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("ref-"))
        .collect();
    let run = |source: &str| transformed(source);
    trust_nested_rotator_builtins_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_nested_rotator_builtins_for_tests(false);
    let missing: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| {
            !diverged
                .iter()
                .any(|row| row.starts_with(&format!("{label} ")))
        })
        .collect();
    assert!(sabotage.len() >= 5, "found {} rows", sabotage.len());
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}
