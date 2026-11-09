use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    trust_callee_objects_for_tests, trust_cookie_block_vars_for_tests,
    trust_inline_returners_for_tests, trust_sequence_object_reads_for_tests,
};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/remaining_shape_rows.txt");
const GUARD: &str = include_str!("data/cookie_block_guard.txt");

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

type Switch = fn(bool);

const KEPT: [(&str, Switch); 3] = [
    (
        "keep-a-returner-call-that-is-not-self-defending",
        trust_inline_returners_for_tests,
    ),
    (
        "keep-a-callee-object-read-before-its-assignment",
        trust_callee_objects_for_tests,
    ),
    (
        "keep-a-sequence-object-read-before-its-assignment",
        trust_sequence_object_reads_for_tests,
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
    for (label, switch) in KEPT {
        let kept = row(label);
        assert_eq!(kept.len(), 1, "{label}");
        switch(true);
        let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
        let diverged = node_rows::mismatches(&node, &kept, &modes);
        switch(false);
        assert_eq!(
            diverged.len(),
            1,
            "{label} prints like node under its mutant"
        );
    }
}
