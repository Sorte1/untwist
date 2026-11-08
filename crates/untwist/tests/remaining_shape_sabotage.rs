use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::{
    trust_cookie_block_vars_for_tests, trust_inline_returners_for_tests,
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

fn keep_rows() -> Vec<(String, String)> {
    ROWS.lines()
        .filter_map(|line| {
            let (label, rest) = line.split_once('|')?;
            let (_, program) = rest.split_once('|')?;
            label
                .starts_with("keep-")
                .then(|| (label.to_string(), program.to_string()))
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
    let kept = keep_rows();
    assert!(!kept.is_empty());
    trust_inline_returners_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &transformed)];
    let diverged = node_rows::mismatches(&node, &kept, &modes);
    trust_inline_returners_for_tests(false);
    assert_eq!(diverged.len(), kept.len(), "{diverged:?}");
}
