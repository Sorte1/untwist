use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::trust_holders_written_first_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/legacy_array_rows.txt");
const GUARDED: [&str; 2] = [
    "guard-atob-prologue-holder-written-in-one-branch",
    "guard-atob-prologue-holder-written-on-one-side-of-a-logical",
];

fn transformed(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_holder_written_only_on_one_path_diverges_from_node_once_that_write_is_trusted() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| GUARDED.contains(label))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert_eq!(sabotage.len(), GUARDED.len());
    trust_holders_written_first_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    trust_holders_written_first_for_tests(false);
    assert_eq!(diverged.len(), GUARDED.len(), "{diverged:?}");
}
