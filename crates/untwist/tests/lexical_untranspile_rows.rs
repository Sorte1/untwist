use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const ROWS: &str = include_str!("data/lexical_untranspile_rows.txt");

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

fn row(label: &str) -> String {
    rows(ROWS)
        .into_iter()
        .find(|(name, _)| name == label)
        .map(|(_, program)| program)
        .unwrap_or_else(|| panic!("no row {label}"))
}

fn unsafe_options() -> Options {
    Options {
        unsafe_transforms: true,
        ..Options::default()
    }
}

fn unbraced_options() -> Options {
    Options {
        disabled_passes: vec!["block-statements"],
        ..Options::default()
    }
}

fn unsafe_output(label: &str) -> String {
    transformed(&unsafe_options(), &row(label))
}

#[test]
fn every_row_runs_as_node_runs_the_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = rows(ROWS);
    assert!(programs.len() >= 40, "found {} rows", programs.len());
    let safe = Options::default();
    let unsafe_transforms = unsafe_options();
    let unbraced = unbraced_options();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let run_unbraced = |source: &str| transformed(&unbraced, source);
    let modes: [Mode<'_>; 3] = [
        ("default", &run_safe),
        ("unsafe", &run_unsafe),
        ("unbraced", &run_unbraced),
    ];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_for_head_var_in_an_unbraced_loop_body_stays_var() {
    for label in [
        "for-head-var-in-unbraced-while",
        "for-head-var-in-unbraced-do",
        "for-head-var-in-unbraced-labelled-while",
        "for-head-var-in-unbraced-if-in-while",
    ] {
        let code = transformed(&unbraced_options(), &row(label));
        assert!(code.contains("for (var i;"), "{label}:\n{code}");
    }
}

#[test]
fn a_for_head_var_in_the_function_body_still_becomes_let() {
    for options in [Options::default(), unbraced_options()] {
        let code = transformed(&options, &row("for-head-var-in-function-body"));
        assert!(code.contains("for (let i;"), "{code}");
    }
}

#[test]
fn step_value_reads_that_are_not_plain_reads_keep_the_helper() {
    for label in [
        "for-of-step-value-called",
        "for-of-step-value-called-in-parens",
        "for-of-step-value-called-with-arguments",
        "for-of-delete-step-value-strict",
        "for-of-delete-step-value-sloppy",
        "for-of-compound-member-target-add",
        "for-of-compound-member-target-or",
        "for-of-compound-member-target-and",
        "for-of-compound-member-target-nullish",
        "for-of-compound-member-target-sub",
        "for-of-computed-member-compound",
    ] {
        let code = unsafe_output(label);
        assert!(
            code.contains("_createForOfIteratorHelper("),
            "{label}:\n{code}"
        );
    }
}

#[test]
fn plain_step_value_reads_still_head_the_loop() {
    for label in [
        "for-of-step-value-called-through-a-sequence",
        "for-of-step-value-method-on-the-value",
        "for-of-typeof-step-value",
        "for-of-delete-keyed-by-step-value",
        "for-of-delete-property-of-step-value",
        "for-of-plain-member-target",
        "for-of-compound-name-target",
    ] {
        let code = unsafe_output(label);
        assert!(!code.contains(".s();"), "{label}:\n{code}");
        assert!(code.contains(" of "), "{label}:\n{code}");
    }
}

#[test]
fn a_values_record_not_shown_empty_keeps_the_loop() {
    for label in [
        "tsc-values-record-stale-in-a-loop",
        "tsc-values-record-preset",
        "tsc-values-record-parameter",
        "tsc-values-record-in-an-outer-function",
        "tsc-values-record-in-a-while-without-reset",
    ] {
        let code = unsafe_output(label);
        assert!(code.contains("__values([1, 2])"), "{label}:\n{code}");
    }
}

#[test]
fn a_fresh_or_reset_values_record_still_becomes_for_of() {
    for label in [
        "tsc-values-record-fresh",
        "tsc-values-record-fresh-in-a-branch",
        "tsc-values-record-reset-in-a-loop",
    ] {
        let code = unsafe_output(label);
        assert!(!code.contains("__values(["), "{label}:\n{code}");
        assert!(code.contains(" of [1, 2]"), "{label}:\n{code}");
    }
}

#[test]
fn a_try_list_with_the_outer_region_first_keeps_the_machine() {
    let kept = unsafe_output("regenerator-try-list-outer-first");
    assert!(kept.contains("[[0, 5], [1, 3]]"), "{kept}");
    let restored = unsafe_output("regenerator-try-list-inner-first");
    assert!(restored.contains("function* ga()"), "{restored}");
}

#[test]
fn use_strict_is_not_promoted_where_strict_code_cannot_parse() {
    for label in [
        "strict-promotion-non-simple-parameters",
        "strict-promotion-default-parameter",
        "strict-promotion-destructured-parameter",
        "strict-promotion-arrow-rest-parameter",
        "strict-promotion-with-statement",
        "strict-promotion-nested-with-statement",
    ] {
        let code = unsafe_output(label);
        assert!(code.contains("(\"use strict\");"), "{label}:\n{code}");
    }
}

#[test]
fn use_strict_is_still_promoted_with_simple_parameters() {
    let source = row("strict-promotion-default-parameter").replacen("(a = 1)", "(a)", 1);
    let code = transformed(&unsafe_options(), &source);
    assert!(code.contains("\"use strict\";"), "{code}");
    assert!(!code.contains("(\"use strict\");"), "{code}");
}
