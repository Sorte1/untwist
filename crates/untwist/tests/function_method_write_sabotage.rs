use untwist::{Options, Pipeline};
use untwist_deobfuscate::mutants::ignore_function_method_writes_for_tests;
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/builtin_alias_rows.txt");
const ROW: &str = "ref-a-function-whose-apply-was-replaced-by-a-definer";

fn transformed(source: &str) -> String {
    let options = Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn a_replaced_apply_diverges_from_node_once_its_write_is_ignored() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let sabotage: Vec<(String, String)> = ROWS
        .lines()
        .filter_map(|line| line.split_once('|'))
        .filter(|(label, _)| *label == ROW)
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    assert_eq!(sabotage.len(), 1);
    let run = |source: &str| transformed(source);
    ignore_function_method_writes_for_tests(true);
    let modes: [Mode<'_>; 1] = [("mutant", &run)];
    let diverged = node_rows::mismatches(&node, &sabotage, &modes);
    ignore_function_method_writes_for_tests(false);
    assert_eq!(diverged.len(), 1, "{diverged:?}");
}
