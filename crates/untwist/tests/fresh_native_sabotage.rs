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
fn every_ref_row_prints_like_node_even_when_the_induction_is_trusted_everywhere() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let refused = with_prefix("ref-");
    assert!(refused.len() >= 8, "found {} rows", refused.len());
    let safe = options(false);
    let run_safe = |source: &str| transformed(&safe, source);
    let modes: [Mode<'_>; 1] = [("mutant", &run_safe)];
    trust_fresh_native_induction_for_tests(true);
    let wrong = node_rows::mismatches(&node, &refused, &modes);
    trust_fresh_native_induction_for_tests(false);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
