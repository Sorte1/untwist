use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::fold_dead_member_results_in_callees_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/dead_member_rows.txt");

fn rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn transformed(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn folded(code: &str) -> bool {
    !code.contains("\"ab\"") && !code.contains("\"cd\"")
}

#[test]
fn each_rows_fold_decision_matches_its_label() {
    for (label, source) in rows() {
        let code = transformed(&source);
        assert_eq!(
            folded(&code),
            label.starts_with("fold-"),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn the_folds_hold_up_against_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run = |source: &str| transformed(source);
    let modes: [Mode<'_>; 1] = [("unsafe", &run)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let kept: Vec<(String, String)> = programs
        .into_iter()
        .filter(|(label, _)| label.starts_with("keep-"))
        .collect();
    fold_dead_member_results_in_callees_for_tests(true);
    let diverged = node_rows::mismatches(&node, &kept, &modes);
    fold_dead_member_results_in_callees_for_tests(false);
    assert_eq!(diverged.len(), kept.len(), "{}", diverged.join("\n"));
}
