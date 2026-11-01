use untwist_testing::node::assert_node_agrees;
use untwist_testing::{assert_transform, transform, Allocator};

#[test]
fn simplify() {
    assert_transform!(
        "-0x1021e + -0x7eac8 + 0x17 * 0xac9c",
        [untwist_unminify::number_expressions::boxed],
        @"431390;"
    );
}

#[test]
fn simplify_coerced_string() {
    assert_transform!(
        r#"-"0xa6" - -331; -"0xa6""#,
        [untwist_unminify::number_expressions::boxed],
        @r"
    165;
    -166;
    "
    );
}

#[test]
fn simplify_division() {
    assert_transform!(
        "10 / 2",
        [untwist_unminify::number_expressions::boxed],
        @"5;"
    );
}

#[test]
fn keep_divisions_if_it_results_in_a_decimal_number() {
    assert_transform!(
        "(-0x152f + 0x1281 * -0x1 + -0x18 * -0x1d1) / (0x83 * -0x1a + -0x19ea + 0x5f * 0x6a)",
        [untwist_unminify::number_expressions::boxed],
        @"1e3 / 30;"
    );
}

#[test]
fn string_and_number_concatenation() {
    assert_transform!(
        r#"0x1021e + "test""#,
        [untwist_unminify::number_expressions::boxed],
        @r#"("66078test");"#
    );
}

#[test]
fn ignore_lone_surrogate_string_operand() {
    assert_transform!(
        r#"var n = "\ud800" + 1;"#,
        [untwist_unminify::number_expressions::boxed],
        @r#"var n = "\ud800" + 1;"#
    );
}

const INEXACT_POWERS: &str = "console.log(10 ** -5, 10 ** 23, 2 ** 0.5, 7 ** -3);";

#[test]
fn a_power_that_is_not_provably_exact_is_left_alone() {
    let alloc = Allocator::default();
    let output = transform(
        &alloc,
        INEXACT_POWERS,
        &[untwist_unminify::number_expressions::boxed],
    );
    assert_eq!(output.matches("**").count(), 4, "{output}");
    assert_node_agrees(INEXACT_POWERS, &output);
}

const EXACT_POWERS: &str = "console.log(2 ** 10, 3 ** 0, 0 ** 3, 10 ** 15);";

#[test]
fn an_exact_integer_power_still_folds() {
    let alloc = Allocator::default();
    let output = transform(
        &alloc,
        EXACT_POWERS,
        &[untwist_unminify::number_expressions::boxed],
    );
    assert!(!output.contains("**"), "{output}");
    assert_node_agrees(EXACT_POWERS, &output);
}

#[test]
fn a_string_chain_folds_whole_and_counts_one_change_per_link() {
    let alloc = untwist_testing::Allocator::default();
    let (output, report) = untwist_testing::transform_report(
        &alloc,
        "x = 'a' + 'b' + 'c' + 'd'; y = 3 + 'a' + 'b'; z = q + 'a' + 'b';",
        &[untwist_unminify::number_expressions::boxed],
        untwist_core::pass::PassOptions::default(),
    );
    untwist_testing::insta::assert_snapshot!(output, @r#"
    x = "abcd";
    y = "3ab";
    z = q + "a" + "b";
    "#);
    assert_eq!(report.per_pass, [("number-expressions", 5)]);
}
