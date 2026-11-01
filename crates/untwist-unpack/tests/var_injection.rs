mod common;

use common::parse;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::model::Module;
use untwist_unpack::webpack::var_injection::inline_var_injections;

fn inline(source: &str) -> String {
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let cloned = untwist_core::parse::parse_program(&alloc, source).program;
    let _ = program;
    let mut module = Module::new("0".to_string(), cloned, false);
    inline_var_injections(&alloc, &mut module);
    module.code().trim_end().to_string()
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let program = untwist_core::parse::parse_program(&alloc, source).program;
    let module = Module::new("0".to_string(), program, false);
    module.code().trim_end().to_string()
}

#[test]
fn splices_injected_variables_and_the_body() {
    let source = "require.r(exports);\n(function (m, n) {\n  console.log(m, n);\n}).call(this, require(1), require(2));\n";
    assert_eq!(
        inline(source),
        "require.r(exports);\nvar m = require(1);\nvar n = require(2);\nconsole.log(m, n);"
    );
}

#[test]
fn accepts_exports_as_the_this_argument() {
    let source = "(function (global) {\n  global.x = 1;\n}).call(exports, window);\n";
    assert_eq!(inline(source), "var global = window;\nglobal.x = 1;");
}

#[test]
fn a_missing_argument_yields_an_uninitialised_var() {
    let source = "(function (a, b) {\n  a();\n}).call(this, 1);\n";
    assert_eq!(inline(source), "var a = 1;\nvar b;\na();");
}

#[test]
fn unrelated_statements_are_untouched() {
    for source in [
        "(function (m) {\n  m();\n}).call(other, 1);\n",
        "(function (m) {\n  m();\n}).call(this);\n",
        "(function ({ m }) {\n  m();\n}).call(this, 1);\n",
        "(function (m) {\n  m();\n}).apply(this, [1]);\n",
    ] {
        let expected = printed(source);
        assert_eq!(inline(source), expected);
    }
}

#[test]
fn nested_injections_are_not_touched() {
    let source = "function f() {\n  (function (m) {\n    m();\n  }).call(this, 1);\n}\n";
    assert!(inline(source).contains(".call(this, 1)"));
}

#[test]
fn a_spread_argument_after_the_receiver_is_left_untouched() {
    let source = "(function (m) {\n  m();\n}).call(this, ...x);\n";
    assert_eq!(inline(source), printed(source));
}

#[test]
fn a_spread_among_later_arguments_is_left_untouched() {
    let source = "(function (a, b, c) {}).call(this, 1, ...y, 3);\n";
    assert_eq!(inline(source), printed(source));
}

#[test]
fn extra_arguments_beyond_the_parameter_count_are_ignored() {
    let source = "(function (a) {}).call(this, 1, 2);\n";
    assert_eq!(inline(source), "var a = 1;");
}

#[test]
fn a_computed_call_property_is_accepted() {
    let source = "(function (m) {})[\"call\"](this, 1);\n";
    assert_eq!(inline(source), "var m = 1;");
}

#[test]
fn a_named_function_expression_callee_is_accepted() {
    let source = "(function foo(m) {}).call(this, 1);\n";
    assert_eq!(inline(source), "var m = 1;");
}
