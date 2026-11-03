use super::Flow;
use oxc_allocator::Allocator;
use oxc_span::Span;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;

fn span_of(source: &str, needle: &str) -> Span {
    let start = source.find(needle).expect("needle present") as u32;
    Span::new(start, start + needle.len() as u32)
}

fn reaches(source: &str) -> Option<Span> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let semantics = build_scoping(&parsed.program);
    let flow = Flow::collect(&parsed.program, &semantics.scoping, None);
    flow.sink_of(span_of(source, "o[k]"))
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
