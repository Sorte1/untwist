use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_fresh_native_induction_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/fresh_native_rows.txt");

fn with_prefix(prefix: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with(prefix))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
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
fn every_ref_row_diverges_from_node_once_the_induction_is_trusted_everywhere() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = with_prefix("ref-");
    assert!(sabotage.len() >= 6, "found {} rows", sabotage.len());
    let safe = options(false);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_safe)];
    trust_fresh_native_induction_for_tests(true);
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_fresh_native_induction_for_tests(false);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (mutant):").next())
        .collect();
    let kept: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !names.contains(label))
        .collect();
    assert!(
        kept.is_empty(),
        "these rows still print like node under the mutant: {kept:?}"
    );
}
