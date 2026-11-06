use std::path::Path;

use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::drop_builtin_import_guards_for_tests;
use untwist_testing::node::run;
use untwist_testing::node_rows::{node_or_skip, observed};

const ROWS: &str = include_str!("data/builtin_import_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn printed_as_module(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    let out = run(node, &path).unwrap().expect("finishes");
    observed(&out)
}

#[test]
fn every_guard_row_diverges_from_node_once_every_import_is_read_as_a_codeless_builtin() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let guards: Vec<(String, String)> = rows()
        .into_iter()
        .filter(|(label, _)| label.starts_with("guard-"))
        .collect();
    assert!(guards.len() >= 2, "found {} guard rows", guards.len());
    drop_builtin_import_guards_for_tests(true);
    let outputs: Vec<String> = guards
        .iter()
        .map(|(_, source)| transformed(false, source))
        .collect();
    drop_builtin_import_guards_for_tests(false);
    for ((label, source), output) in guards.iter().zip(outputs) {
        let before = printed_as_module(&node, dir.path(), source);
        let after = printed_as_module(&node, dir.path(), &output);
        assert_ne!(
            after, before,
            "{label} did not diverge from node under the mutant"
        );
    }
}
