use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    trust_nested_write_targets_for_tests, trust_write_only_order_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/write_only_rows.txt");

type Switch = fn(bool);

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

fn dropped(code: &str) -> bool {
    !code.contains("e = 1") && !code.contains("e = -620")
}

#[test]
fn each_rows_drop_decision_matches_its_label_in_both_lanes() {
    for lane in [false, true] {
        for (label, source) in rows() {
            let code = transformed(lane, &source);
            assert_eq!(
                dropped(&code),
                label.starts_with("drop-"),
                "{label} (unsafe={lane}):\n{code}"
            );
        }
    }
}

#[test]
fn the_rows_hold_up_against_node_and_the_kept_ones_need_their_checks() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows();
    let run_safe = |source: &str| transformed(false, source);
    let run_unsafe = |source: &str| transformed(true, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let checks: [(&str, Switch); 2] = [
        ("keep-a-let-set-before", trust_write_only_order_for_tests),
        ("keep-a-nested-write", trust_nested_write_targets_for_tests),
    ];
    for (prefix, switch) in checks {
        let guarded: Vec<(String, String)> = programs
            .iter()
            .filter(|(label, _)| label.starts_with(prefix))
            .cloned()
            .collect();
        assert!(!guarded.is_empty(), "no {prefix} rows");
        switch(true);
        let modes: [Mode<'_>; 1] = [("mutant", &run_safe)];
        let diverged = node_rows::mismatches(&node, &guarded, &modes);
        switch(false);
        assert_eq!(diverged.len(), guarded.len(), "{}", diverged.join("\n"));
    }
}
