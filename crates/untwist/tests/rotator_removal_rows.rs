use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::remove_unsealed_rotators_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/rotator_removal_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter(|line| !line.trim().is_empty())
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

#[test]
fn each_rows_decode_decision_matches_its_label_in_both_lanes() {
    for (label, source) in rows() {
        let should_decode = label.starts_with("dec-");
        for lane in [false, true] {
            let code = transformed(lane, &source);
            assert_eq!(
                !code.contains("push("),
                should_decode,
                "{label} (unsafe={lane}):\n{code}"
            );
        }
    }
}

#[test]
fn the_rows_print_like_node_and_kept_rotators_need_the_seal() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let guarded: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| label.starts_with("keep-whole-"))
        .collect();
    assert!(!guarded.is_empty(), "no keep-whole- rows");
    remove_unsealed_rotators_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run_unsafe)];
    let diverged = node_rows::mismatches(&node, &guarded, &modes);
    remove_unsealed_rotators_for_tests(false);
    assert_eq!(diverged.len(), guarded.len(), "{}", diverged.join("\n"));
}
