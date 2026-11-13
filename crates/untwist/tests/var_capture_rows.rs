use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/var_capture_rows.txt");

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

fn rows(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect()
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn row(label: &str) -> String {
    rows(ROWS)
        .into_iter()
        .find(|(name, _)| name == label)
        .map(|(_, program)| program)
        .unwrap()
}

#[test]
fn a_var_read_by_later_closures_becomes_lexical_only_as_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 50, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (wrong, witnesses) = node_rows::unsafe_dynamic_code_witnesses(wrong, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(witnesses.is_empty(), "{witnesses:?}");
}

#[test]
fn a_var_read_by_a_later_arrow_becomes_const_in_both_lanes() {
    let source = row("called-twice");
    for options in [Options::default(), unsafe_options()] {
        let code = transformed(&options, &source);
        assert!(code.contains("const a = x;"), "{code}");
    }
}

#[test]
fn a_var_read_by_a_hoisted_declaration_stays_a_var() {
    let source = row("hoisted-declaration-reads");
    let code = transformed(&Options::default(), &source);
    assert!(code.contains("var a = 1;"), "{code}");
}
