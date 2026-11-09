use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::lift_shell_bodies_with_bindings_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/shell_binding_rows.txt");

fn sabotage_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| label.starts_with("ref-"))
        .map(|(label, shell)| (label.to_string(), format!("{shell}()")))
        .collect()
}

fn run(deobfuscate: bool, unsafe_transforms: bool, called: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        deobfuscate,
        unsafe_transforms,
        ..Options::default()
    };
    let shell = called.strip_suffix("()").unwrap_or(called);
    Pipeline::new(options).run(shell).code().to_string()
}

#[test]
fn every_sabotage_row_diverges_from_node_once_shell_bodies_are_lifted_with_their_bindings() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage = sabotage_rows();
    assert!(
        sabotage.len() >= 15,
        "found {} sabotage rows",
        sabotage.len()
    );
    lift_shell_bodies_with_bindings_for_tests(true);
    let default = |called: &str| run(true, false, called);
    let unsafe_lane = |called: &str| run(true, true, called);
    let plain = |called: &str| run(false, false, called);
    let modes: [Mode<'_>; 3] = [
        ("default", &default),
        ("unsafe", &unsafe_lane),
        ("no-deobfuscate", &plain),
    ];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    lift_shell_bodies_with_bindings_for_tests(false);
    let names: Vec<&str> = diverged
        .iter()
        .filter_map(|row| row.split(" (").next())
        .collect();
    let missing: Vec<&str> = sabotage
        .iter()
        .map(|(label, _)| label.as_str())
        .filter(|label| !names.contains(label))
        .collect();
    assert!(
        missing.is_empty(),
        "did not diverge from node under the mutant: {missing:?}"
    );
}
