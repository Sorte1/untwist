mod common;

use common::eval;

#[test]
fn a_self_rewriting_memoising_function_returns_the_same_array_object() {
    let source = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
var first = a();
var second = a();
first[2] = 'x';
second.length";
    assert_eq!(eval(source), Ok("number:3".to_string()));
}

#[test]
fn the_decoder_shape_runs_outer_then_inner_and_inner_only_afterwards() {
    let source = "\
function a() { var h = ['zero', 'one']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
b(0x1) + '|' + b(0x1) + '|' + b(0x0)";
    assert_eq!(eval(source), Ok("string:one|one|zero".to_string()));
}

#[test]
fn arguments_is_an_aliasable_object() {
    let source = "\
function outer(c, d) {
  return (outer = function (f, g) {
    c = arguments;
    c[7] = 'cached';
    return c[7] + ':' + c.length;
  }), outer(c, d);
}
outer(1, 2)";
    assert_eq!(
        eval(source),
        Err("unsupported construct: an arguments index write past its length, which the evaluator does not model".to_string())
    );
}

#[test]
fn method_calls_bind_this_and_arrows_inherit_it() {
    assert_eq!(
        eval("var o = { n: 3, get: function () { return this.n; } }; o.get()"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        eval("var o = { n: 3, get: function () { var f = () => this.n; return f(); } }; o.get()"),
        Ok("number:3".to_string())
    );
}

#[test]
fn hoisted_inner_function_declarations_close_over_the_outer_decoder() {
    let source = "\
function b(f) { return f + 1; }
(function () { return i(1); function i(c) { return b(c - -0x1); } })()";
    assert_eq!(eval(source), Ok("number:3".to_string()));
}

#[test]
fn missing_arguments_are_undefined_and_extra_ones_are_kept_in_arguments() {
    assert_eq!(
        eval("function f(a, b) { return String(b) + ':' + arguments.length; } f(1)"),
        Ok("string:undefined:1".to_string())
    );
}

#[test]
fn call_depth_is_capped_rather_than_overflowing_the_rust_stack() {
    assert_eq!(
        eval("function d(c) { return d(c + 1); } d(0)"),
        Err("call depth cap exceeded".to_string())
    );
}

#[test]
fn calling_a_non_function_throws_a_catchable_type_error() {
    assert_eq!(
        eval("var x = 1; x()"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn optional_call_on_undefined_callee_short_circuits() {
    assert_eq!(eval("var a; a?.b()"), Ok("undefined".to_string()));
}

#[test]
fn optional_call_operator_on_present_member_that_is_nullish_short_circuits() {
    assert_eq!(eval("var a = {}; a.b?.()"), Ok("undefined".to_string()));
}

#[test]
fn whole_chain_short_circuits_before_a_following_member_access() {
    assert_eq!(eval("var a; a?.b().c"), Ok("undefined".to_string()));
}

#[test]
fn method_call_binds_this_to_the_receiver_object() {
    assert_eq!(
        eval("var o = { f: function () { return this.v; }, v: 3 }; o.f()"),
        Ok("number:3".to_string())
    );
}

#[test]
fn callee_object_is_evaluated_exactly_once_for_a_method_call() {
    assert_eq!(
        eval("var n = 0; var o = { f: function () { return 1; } }; (n++, o).f(); n"),
        Ok("number:1".to_string())
    );
}

#[test]
fn a_named_function_expression_can_call_itself_by_its_own_name() {
    assert_eq!(
        eval("var f = function g(n) { return n ? g(n - 1) : 0; }; f(3)"),
        Ok("number:0".to_string())
    );
}

#[test]
fn a_named_function_expressions_own_name_is_typeof_function() {
    assert_eq!(
        eval("var f = function g() { return typeof g; }; f()"),
        Ok("string:function".to_string())
    );
}

#[test]
fn a_parameter_shadows_the_named_function_expressions_own_name() {
    assert_eq!(
        eval("var f = function g(g) { return g; }; f(5)"),
        Ok("number:5".to_string())
    );
}

#[test]
fn a_function_declaration_can_still_call_itself_by_its_own_name() {
    assert_eq!(
        eval("function h(n) { return n ? h(n - 1) : 0; } h(3)"),
        Ok("number:0".to_string())
    );
}

#[test]
fn the_own_name_binding_is_local_to_the_function_expression_and_does_not_leak_out() {
    assert_eq!(
        eval("var g = 1; var f = function g() { return g; }; typeof f()"),
        Ok("string:function".to_string())
    );
    assert_eq!(
        eval("var g = 1; var f = function g() {}; g"),
        Ok("number:1".to_string())
    );
}

#[test]
fn a_parenthesised_member_callee_keeps_its_receiver() {
    let source = "var o = { x: 1, f: function () { return this['x']; } }; (o['f'])()";
    let alloc = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&alloc, source, oxc_span::SourceType::cjs())
        .with_options(oxc_parser::ParseOptions {
            preserve_parens: true,
            ..oxc_parser::ParseOptions::default()
        })
        .parse();
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let (is_one, rendered) = untwist_eval::interp::eval_script_in(
        &parsed.program,
        untwist_eval::budget::Budget::default(),
        |result, _| {
            (
                matches!(result, Ok(untwist_eval::value::Value::Number(n)) if n == 1.0),
                format!("{result:?}"),
            )
        },
    );
    assert!(is_one, "expected 1, got {rendered}");
}
