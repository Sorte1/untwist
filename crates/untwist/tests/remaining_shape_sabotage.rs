use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    trust_callee_objects_for_tests, trust_cookie_block_vars_for_tests,
    trust_inline_returners_for_tests, trust_sequence_object_reads_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/remaining_shape_rows.txt");
const GUARD: &str = include_str!("data/cookie_block_guard.txt");

fn run(unsafe_transforms: bool, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn transformed(source: &str) -> String {
    run(false, source)
}

fn transformed_unsafe(source: &str) -> String {
    run(true, source)
}

type Switch = fn(bool);
type Lane = fn(&str) -> String;

const KEPT: [(&str, Switch, Lane); 4] = [
    (
        "keep-a-returner-call-that-is-not-self-defending",
        trust_inline_returners_for_tests,
        transformed_unsafe,
    ),
    (
        "keep-a-callee-object-read-before-its-assignment",
        trust_callee_objects_for_tests,
        transformed,
    ),
    (
        "keep-a-returner-variable-that-is-not-self-defending",
        trust_inline_returners_for_tests,
        transformed_unsafe,
    ),
    (
        "keep-a-sequence-object-read-before-its-assignment",
        trust_sequence_object_reads_for_tests,
        transformed,
    ),
];

fn row(label: &str) -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| {
            let (name, rest) = line.split_once('|')?;
            let (_, program) = rest.split_once('|')?;
            (name == label).then(|| (name.to_string(), program.to_string()))
        })
        .collect()
}

#[test]
fn the_checks_hold_up_the_kept_and_guarded_rows() {
    let (_, guard) = GUARD.trim_end().split_once('|').expect("guard row");
    assert!(transformed(guard).contains("arr0 = ["));
    trust_cookie_block_vars_for_tests(true);
    let decoded = !transformed(guard).contains("arr0 = [");
    trust_cookie_block_vars_for_tests(false);
    assert!(decoded, "the guard row stays refused under the mutant");
    let Some(node) = node_or_skip() else {
        return;
    };
    for (label, switch, lane) in KEPT {
        let kept = row(label);
        assert_eq!(kept.len(), 1, "{label}");
        switch(true);
        let modes: [Mode<'_>; 1] = [("mutant", &lane)];
        let diverged = node_rows::mismatches(&node, &kept, &modes);
        switch(false);
        assert_eq!(
            diverged.len(),
            1,
            "{label} prints like node under its mutant"
        );
    }
}
