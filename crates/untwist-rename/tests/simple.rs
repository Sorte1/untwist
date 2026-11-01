mod common;

use common::{assert_scope_safe, rename_compact};
use untwist_rename::{RenameConfig, RenameMode};

fn simple(source: &str) -> String {
    let config = RenameConfig {
        mode: RenameMode::Simple,
        rename_all: false,
        extended: true,
    };
    let output = rename_compact(source, config);
    assert_scope_safe(source, &output);
    output
}

#[test]
fn each_function_restarts_the_sequence() {
    assert_eq!(
        simple("function _0xa(_0xb) { var _0xc = _0xb; return _0xc; }"),
        "function a(a){var b=a;return b}"
    );
}

#[test]
fn block_scopes_continue_their_functions_sequence() {
    assert_eq!(
        simple("function _0xf() { var _0xa = 1; { var _0xb = 2; let _0xc = 3; _0xc; } }"),
        "function a(){var a=1;{var b=2;let c=3;c}}"
    );
}

#[test]
fn a_name_referenced_from_an_enclosing_scope_is_skipped() {
    assert_eq!(
        simple("var _0x1 = 1; var _0x2 = 2; function _0x3(_0x4) { return _0x1 + _0x4; }"),
        "var a=1;var b=2;function c(b){return a+b}"
    );
}

#[test]
fn an_enclosing_name_that_is_never_referenced_inside_may_be_reused() {
    assert_eq!(
        simple("var _0x1 = 1; function _0x2(_0x3) { return _0x3; } _0x1;"),
        "var a=1;function b(a){return a}a;"
    );
}

#[test]
fn globals_and_reserved_words_are_never_handed_out() {
    assert_eq!(
        simple("function _0xa() { return a + b + c; }"),
        "function d(){return a+b+c}"
    );
}

#[test]
fn running_simple_twice_changes_nothing() {
    let once = simple("var _0x1 = 1; var _0x2 = 2; function _0x3(_0x4) { return _0x1 + _0x4; }");
    assert_eq!(simple(&once), once);
}

fn fixed_point(source: &str) -> String {
    let once = simple(source);
    let twice = simple(&once);
    assert_eq!(once, twice, "Simple must settle in one run");
    once
}

#[test]
fn an_already_minified_program_settles_in_one_run() {
    assert_eq!(fixed_point("var b = 1; var a = 2;"), "var a=1;var b=2;");
}

#[test]
fn a_rotated_declaration_list_settles_in_one_run() {
    assert_eq!(
        fixed_point("var c = 1, a = 2, b = 3; function d() { return a + b + c; } d();"),
        "var a=1,b=2,c=3;function d(){return b+c+a}d();"
    );
}

#[test]
fn a_minified_function_and_its_caller_settle_in_one_run() {
    assert_eq!(
        fixed_point("function e(t, n) { var r = t + n; return r; } var a = e(1, 2); a;"),
        "function a(a,b){var c=a+b;return c}var b=a(1,2);b;"
    );
}

#[test]
fn a_pending_descendant_holder_does_not_capture_an_enclosing_name() {
    assert_eq!(
        fixed_point("var a = 1; function f() { var b = a; }"),
        "var a=1;function b(){var b=a}"
    );
}

#[test]
fn an_arrow_unit_restarts_the_sequence() {
    assert_eq!(
        simple("var _0xa = 1; var _0xb = (_0xc) => _0xa + _0xc;"),
        "var a=1;var b=b=>a+b;"
    );
}

#[test]
fn a_class_static_block_restarts_the_sequence() {
    assert_eq!(
        simple("var _0xa = 1; class _0xK { static { var _0xb = _0xa; _0xb; } }"),
        "var a=1;class b{static{var b=a;b}}"
    );
}

#[test]
fn a_var_the_block_reads_before_it_is_declared_settles_in_one_run() {
    assert_eq!(
        fixed_point("function f(p) { { let q = b; q; } var b = 1; }"),
        "function a(a){{let b=c;b}var c=1}"
    );
}
