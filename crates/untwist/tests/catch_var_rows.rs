use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/catch_var_rows.txt");
const BINDING_ROWS: &str = include_str!("data/catch_var_binding_rows.txt");

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

#[test]
fn a_var_redeclaring_a_catch_parameter_keeps_every_same_named_var_a_var() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_reviewed_function_keeps_its_outer_var() {
    let source = rows(ROWS)
        .into_iter()
        .find(|(label, _)| label == "catch-var-bare")
        .map(|(_, program)| program)
        .unwrap();
    let out = transformed(&Options::default(), &source);
    assert!(out.contains("var g = 1;"), "{out}");
    assert!(!out.contains("const g"), "{out}");
}

fn options(rename: RenameMode, unsafe_transforms: bool) -> Options {
    Options {
        rename,
        unsafe_transforms,
        ..Options::default()
    }
}

#[test]
fn a_catch_var_that_oxc_binds_to_another_symbol_keeps_its_reads_in_every_mode() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(BINDING_ROWS);
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let lanes = [
        ("default", options(RenameMode::None, false)),
        ("unsafe", options(RenameMode::None, true)),
        ("rename smart", options(RenameMode::Smart, false)),
        ("rename mangle", options(RenameMode::Mangle, false)),
        ("rename simple", options(RenameMode::Simple, false)),
        ("rename unique", options(RenameMode::Unique, false)),
        ("unsafe rename mangle", options(RenameMode::Mangle, true)),
    ];
    let runs: Vec<_> = lanes
        .iter()
        .map(|(_, options)| move |source: &str| transformed(options, source))
        .collect();
    let modes: Vec<Mode<'_>> = lanes
        .iter()
        .zip(&runs)
        .map(|((label, _), run)| (*label, run as &dyn Fn(&str) -> String))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_dead_catch_var_shadowing_a_function_expression_name_keeps_its_var() {
    let source = rows(BINDING_ROWS)
        .into_iter()
        .find(|(label, _)| label == "catch-fn-name-fn1")
        .map(|(_, program)| program)
        .unwrap();
    let out = transformed(&Options::default(), &source);
    assert!(out.contains("var g;"), "{out}");
}
