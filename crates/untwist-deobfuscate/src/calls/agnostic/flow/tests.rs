use super::Flow;
use oxc_allocator::Allocator;
use oxc_span::Span;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fn span_of(source: &str, needle: &str) -> Span {
    let start = source.find(needle).expect("needle present") as u32;
    Span::new(start, start + needle.len() as u32)
}

fn reaches_at(source: &str, needle: &str) -> Option<Span> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let flow = Flow::collect(&parsed.program, &semantics.scoping, None);
    flow.sink_of(span_of(source, needle))
}

fn reaches(source: &str) -> Option<Span> {
    reaches_at(source, "o[k]")
}

fn inert(source: &str) {
    assert!(reaches(source).is_none(), "expected inert: {source}");
}

fn reaches_sink(source: &str) {
    assert!(reaches(source).is_some(), "expected a sink: {source}");
}

#[test]
fn unused_value_is_inert() {
    inert("const o = {}; function f(k) { var x = o[k]; return 1; }");
}

#[test]
fn value_returned_to_outside_is_inert() {
    inert("const o = {}; export function f(k) { return o[k]; }");
}

#[test]
fn value_concatenated_into_a_string_is_inert() {
    inert("const o = {}; export function f(k) { return \"\" + o[k]; }");
}

#[test]
fn a_called_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { return o[k](); }");
}

#[test]
fn a_value_stored_to_a_property_reaches_a_sink() {
    reaches_sink(
        "const o = {}; const t = {}; function f(k) { t.h = o[k]; t.h.call(String.prototype, \"x\"); }",
    );
}

#[test]
fn a_value_passed_as_an_argument_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { bar(o[k]); }");
}

#[test]
fn a_value_installed_as_a_getter_reaches_a_sink() {
    reaches_sink("const o = {}; const d = {}; function f(k) { d.get = o[k]; }");
}

#[test]
fn a_value_installed_as_a_thenable_reaches_a_sink() {
    reaches_sink("const o = {}; const p = {}; function f(k) { p.then = o[k]; }");
}

#[test]
fn a_value_in_an_array_element_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { const a = [o[k]]; a[0](); }");
}

#[test]
fn a_value_bound_to_a_param_reaches_a_sink() {
    reaches_sink("const o = {}; function g(p) { p(); } function f(k) { g(o[k]); }");
}

#[test]
fn a_returned_value_that_is_called_reaches_a_sink() {
    reaches_sink("const o = {}; function make(k) { return o[k]; } function f(k) { make(k)(); }");
}

#[test]
fn an_iife_return_that_is_called_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { (function () { return o[k]; })()(); }");
}

#[test]
fn a_destructured_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { var { a } = o[k]; return a; }");
}

#[test]
fn a_value_destructured_by_assignment_reaches_a_sink() {
    reaches_sink("const o = {}; var a; function f(k) { ({ a } = o[k]); }");
}

#[test]
fn a_value_iterated_by_for_of_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { for (const x of o[k]) { x; } }");
}

#[test]
fn a_value_enumerated_by_for_in_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { for (const x in o[k]) { x; } }");
}

#[test]
fn a_thrown_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { throw o[k]; }");
}

#[test]
fn a_value_in_a_with_head_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { with (o[k]) { k; } }");
}

#[test]
fn an_awaited_value_reaches_a_sink() {
    reaches_sink("const o = {}; async function f(k) { await o[k]; }");
}

#[test]
fn a_yielded_value_reaches_a_sink() {
    reaches_sink("const o = {}; function* f(k) { yield o[k]; }");
}

#[test]
fn a_superclass_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { class C extends o[k] {} return C; }");
}

#[test]
fn a_class_field_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { class C { x = o[k]; } return C; }");
}

#[test]
fn a_value_spread_into_a_call_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { g(...o[k]); }");
}

#[test]
fn a_value_spread_into_an_array_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { var a = [...o[k]]; return a; }");
}

#[test]
fn a_value_spread_into_an_object_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { var a = { ...o[k] }; return a; }");
}

#[test]
fn a_value_interpolated_into_a_tagged_template_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { tag`x ${o[k]} y`; }");
}

#[test]
fn a_value_returned_from_a_getter_reaches_a_sink() {
    reaches_sink(
        "const o = {}; function f(k) { var obj = { get g() { return o[k]; } }; return obj; }",
    );
}

#[test]
fn a_value_passed_to_a_function_that_reads_arguments_reaches_a_sink() {
    reaches_sink(
        "const o = {}; function g() { return arguments[0](); } function f(k) { g(o[k]); }",
    );
}

#[test]
fn a_value_passed_to_a_rest_parameter_reaches_a_sink() {
    reaches_sink("const o = {}; function g(...xs) { xs[0](); } function f(k) { g(o[k]); }");
}

#[test]
fn a_value_stored_then_read_by_an_unknown_key_reaches_a_sink() {
    reaches_sink("const o = {}; const t = {}; function f(k) { t.foo = o[k]; var g = t[k]; g(); }");
}

#[test]
fn a_value_used_as_an_unknown_key_receiver_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k, m) { o[k][m](); }");
}

#[test]
fn a_value_passed_to_a_shadowed_native_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { var String = bar; return String(o[k]); }");
}

#[test]
fn a_value_passed_to_a_reassigned_global_native_reaches_a_sink() {
    reaches_sink("const o = {}; String = bar; function f(k) { return String(o[k]); }");
}

#[test]
fn a_value_passed_to_a_native_after_an_unknown_global_write_reaches_a_sink() {
    reaches_sink("const o = {}; window[x] = bar; function f(k) { return String(o[k]); }");
}

#[test]
fn a_value_passed_to_an_unshadowed_native_is_inert() {
    inert("const o = {}; export function f(k) { return String(o[k]); }");
}

#[test]
fn a_parameter_default_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { function g(a = o[k]) { a(); } g(); }");
}

#[test]
fn an_arrow_parameter_default_value_reaches_a_sink() {
    reaches_sink("const o = {}; function f(k) { var g = (a = o[k]) => a(); g(); }");
}

const CLOSED_EVAL: &str = "eval('require;')";

#[test]
fn a_closed_eval_of_a_runner_used_only_by_typeof_is_inert() {
    let source = "const host = eval('require;'); console.log(typeof host);";
    assert!(reaches_at(source, CLOSED_EVAL).is_none(), "{source}");
}

#[test]
fn an_unused_closed_eval_of_a_runner_is_inert() {
    let source = "eval('require;');";
    assert!(reaches_at(source, CLOSED_EVAL).is_none(), "{source}");
}

#[test]
fn a_called_closed_eval_of_a_runner_reaches_a_sink() {
    let source = "const host = eval('require;'); host('vm');";
    assert!(reaches_at(source, CLOSED_EVAL).is_some(), "{source}");
}

#[test]
fn a_closed_eval_whose_runner_name_is_bound_is_not_a_read() {
    let source = "var require = 1; const host = eval('require;'); console.log(typeof host);";
    assert!(reaches_at(source, CLOSED_EVAL).is_some(), "{source}");
}
