use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/var_placement_rows.txt");

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
        disabled_passes: vec!["inline-layers"],
        ..Options::default()
    }
}

fn safe_options() -> Options {
    Options {
        disabled_passes: vec!["inline-layers"],
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
fn a_var_declared_after_its_uses_is_placed_only_as_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 70, "found {} rows", programs.len());
    let safe = safe_options();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_core_js_trailing_var_becomes_a_const_at_its_write_in_both_lanes() {
    let source = row("core-js-get-built-in");
    for options in [Options::default(), unsafe_options()] {
        let code = transformed(&options, &source);
        assert!(code.contains("const v = m[k];"), "{code}");
        assert!(!code.contains("var v"), "{code}");
    }
}

#[test]
fn a_trailing_var_assigned_in_two_branches_moves_to_the_top_in_both_lanes() {
    let source = row("two-branches");
    for options in [Options::default(), unsafe_options()] {
        let code = transformed(&options, &source);
        let declared = code.find("let a;").expect(&code);
        let written = code.find("a = 1;").expect(&code);
        assert!(declared < written, "{code}");
        assert!(!code.contains("var a"), "{code}");
    }
}

#[test]
fn a_trailing_var_captured_in_a_loop_moves_to_the_top() {
    let source = row("closure-in-for-loop");
    let code = transformed(&Options::default(), &source);
    let declared = code.find("let a;").expect(&code);
    let written = code.find("a = i;").expect(&code);
    assert!(declared < written, "{code}");
}
