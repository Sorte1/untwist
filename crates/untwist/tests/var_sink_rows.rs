use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/var_sink_rows.txt");

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
fn a_bare_var_moved_to_its_first_use_runs_as_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let safe = safe_options();
    let unsafe_transforms = unsafe_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_wall_of_wrapper_flags_lands_beside_each_wrapper_in_both_lanes() {
    let source = row("wrapper-wall");
    for options in [safe_options(), unsafe_options()] {
        let code = transformed(&options, &source);
        let at = |needle: &str| code.find(needle).unwrap_or_else(|| panic!("{needle}: {code}"));
        assert!(at("var hasA;") < at("function reqA()"), "{code}");
        assert!(at("function reqA()") < at("var hasB;"), "{code}");
        assert!(at("var hasB;") < at("function reqB()"), "{code}");
        assert!(at("c = {") < at("var a;"), "{code}");
    }
}

#[test]
fn a_bare_var_merges_into_its_later_declaration_in_both_lanes() {
    let source = row("merge-into-later-declaration");
    for options in [safe_options(), unsafe_options()] {
        let code = transformed(&options, &source);
        assert_eq!(code.matches("var a").count() + code.matches("const a").count() + code.matches("let a").count(), 1, "{code}");
    }
}
