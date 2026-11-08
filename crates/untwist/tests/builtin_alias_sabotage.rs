use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_builtin_aliases_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/builtin_alias_rows.txt");
const REFUSED_ELSEWHERE: &[&str] = &[
    "ref-shift-replaced-through-proto",
    "ref-split-replaced-through-an-alias",
];

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn every_alias_row_diverges_from_node_once_aliases_are_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-") && !REFUSED_ELSEWHERE.contains(label))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert!(sabotage.len() >= 12, "found {} rows", sabotage.len());
    let run = |source: &str| transformed(source);
    trust_builtin_aliases_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_builtin_aliases_for_tests(false);
    let missing: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| {
            !diverged
                .iter()
                .any(|row| row.starts_with(&format!("{label} ")))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}
