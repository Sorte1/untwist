mod common;

use untwist_deobfuscate::control_flow_object;
use untwist_testing::Allocator;

const UNRESOLVED: &str = "function hi() {\n  var obj = { aokfw: function (a, b) { return a || b; } };\n  console.log(obj.aokfw(p, q));\n}";
const VAR_BOUND: &str = "function hi(p, q) {\n  var obj = { aokfw: function (a, b) { return a || b; } };\n  console.log(obj.aokfw(p, q));\n}";
const LET_BOUND: &str = "function hi() {\n  let p = 1, q = 2;\n  var obj = { aokfw: function (a, b) { return a || b; } };\n  console.log(obj.aokfw(p, q));\n}";
const LITERALS: &str = "function hi() {\n  var obj = { aokfw: function (a, b) { return a || b; } };\n  console.log(obj.aokfw(1, \"x\"));\n}";
const ARITHMETIC: &str = "function hi() {\n  var obj = { aokfw: function (a, b) { return a + b; } };\n  console.log(obj.aokfw(p, q));\n}";

#[test]
fn conservative_keeps_the_references_answer_for_an_unresolved_argument() {
    let alloc = Allocator::default();
    let code = common::conservative_run(&alloc, UNRESOLVED, control_flow_object::boxed);
    assert!(code.contains("p || q"), "{code}");
}

#[test]
fn the_default_refuses_an_unresolved_argument() {
    let alloc = Allocator::default();
    let code = common::default_run(&alloc, UNRESOLVED, control_flow_object::boxed);
    assert!(code.contains("obj.aokfw(p, q)"), "{code}");
}

#[test]
fn the_default_folds_when_every_argument_is_a_parameter() {
    let alloc = Allocator::default();
    let code = common::default_run(&alloc, VAR_BOUND, control_flow_object::boxed);
    assert!(code.contains("p || q"), "{code}");
}

#[test]
fn the_default_refuses_a_let_bound_argument_because_of_the_tdz() {
    let alloc = Allocator::default();
    let code = common::default_run(&alloc, LET_BOUND, control_flow_object::boxed);
    assert!(code.contains("obj.aokfw(p, q)"), "{code}");
}

#[test]
fn the_default_folds_literal_arguments() {
    let alloc = Allocator::default();
    let code = common::default_run(&alloc, LITERALS, control_flow_object::boxed);
    assert!(code.contains("1 || \"x\""), "{code}");
}

#[test]
fn an_arithmetic_forwarder_is_unaffected_in_both_modes() {
    let alloc = Allocator::default();
    assert!(common::default_run(&alloc, ARITHMETIC, control_flow_object::boxed).contains("p + q"));
    let alloc = Allocator::default();
    assert!(
        common::conservative_run(&alloc, ARITHMETIC, control_flow_object::boxed).contains("p + q")
    );
}

#[test]
fn the_refusal_says_why() {
    let (_code, notes) = common::run_all_stages_extended(UNRESOLVED);
    assert!(
        notes
            .iter()
            .any(|note| note.contains(untwist_deobfuscate::notes::LOGICAL_FORWARDER_ARGUMENT)),
        "{notes:?}"
    );
}

const INLINE_RECEIVER: &str = "function hi() {\n  console.log({ aokfw: function (a, b) { return a || b; }, rLxJs: \"x\" }.aokfw(p, q));\n}";

fn forwarder_call(params: &str, prelude: &str, arguments: &str) -> String {
    format!(
        "function hi({params}) {{\n  {prelude}var obj = {{ aokfw: function (a, b) {{ return a || b; }} }};\n  console.log(obj.aokfw({arguments}));\n}}"
    )
}

fn refused(source: &str) {
    let (code, notes) = common::run_all_stages_extended(source);
    assert!(code.contains("obj.aokfw("), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains(untwist_deobfuscate::notes::LOGICAL_FORWARDER_ARGUMENT)),
        "{notes:?}"
    );
}

fn folds(source: &str, expected: &str) {
    let alloc = Allocator::default();
    let code = common::default_run(&alloc, source, control_flow_object::boxed);
    assert!(code.contains(expected), "{code}");
}

#[test]
fn an_in_operator_over_literals_is_refused_because_it_throws() {
    refused(&forwarder_call("", "", "1, 1 in 2"));
}

#[test]
fn a_bigint_under_an_operator_is_refused() {
    refused(&forwarder_call("", "", "1, 1n + 1"));
}

#[test]
fn bare_bigint_literals_fold() {
    folds(&forwarder_call("", "", "1n, 2n"), "1n || 2n");
}

#[test]
fn a_parameter_under_negation_is_refused_because_to_numeric_runs_code() {
    refused(&forwarder_call("p, q", "", "p, -q"));
}

#[test]
fn a_parameter_under_logical_not_folds() {
    folds(&forwarder_call("p, q", "", "p, !q"), "p || !q");
}

#[test]
fn a_parameter_under_strict_equality_folds() {
    folds(&forwarder_call("p, q", "", "p, q === 1"), "p || q === 1");
}

#[test]
fn unaries_over_literals_fold() {
    folds(&forwarder_call("", "", "-1, !0"), "-1 || !0");
}

#[test]
fn a_binary_over_literals_folds() {
    folds(&forwarder_call("", "", "1 + 2, \"x\""), "1 + 2 || \"x\"");
}

#[test]
fn a_shadowed_undefined_is_refused_because_of_the_tdz() {
    refused(&forwarder_call(
        "",
        "let undefined = 1;\n  ",
        "undefined, 2",
    ));
}

#[test]
fn an_unshadowed_undefined_folds() {
    folds(&forwarder_call("", "", "undefined, 2"), "undefined || 2");
}

#[test]
fn an_inline_receiver_is_refused_and_says_why() {
    let (code, notes) = common::run_all_stages_extended(INLINE_RECEIVER);
    assert!(!code.contains("p || q"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains(untwist_deobfuscate::notes::LOGICAL_FORWARDER_ARGUMENT)),
        "{notes:?}"
    );
}

const LATER_PARAMETER: &str = "function f(a = { aokfw: function (x, y) {\n  return x || y;\n} }.aokfw(1, b), b) {\n  return a;\n}";

#[test]
fn a_read_of_a_later_parameter_inside_a_default_is_a_tdz_read_and_is_refused() {
    let (code, notes) = common::run_all_stages_extended(LATER_PARAMETER);
    assert!(code.contains(".aokfw(1, b), b)"), "{code}");
    assert!(!code.contains("1 || b"), "{code}");
    let refusals = notes
        .iter()
        .filter(|note| note.contains(untwist_deobfuscate::notes::LOGICAL_FORWARDER_ARGUMENT))
        .count();
    assert_eq!(refusals, 1, "{notes:?}");
}

#[test]
fn a_read_of_an_earlier_parameter_inside_a_default_still_folds() {
    let source = "function f(b, c, a = { aokfw: function (x, y) {\n  return x || y;\n} }.aokfw(c, b)) {\n  return a;\n}";
    let (code, _notes) = common::run_all_stages_extended(source);
    assert!(code.contains("a = c || b"), "{code}");
}
