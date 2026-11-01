mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, Interp};
use untwist_eval::value::AbruptKind;
use untwist_testing::node::printed_by_node;

const TYPE_ERROR: &str = "throw error:TypeError";

fn without_kind(rendered: String) -> String {
    ["string:", "number:", "boolean:"]
        .iter()
        .find_map(|kind| rendered.strip_prefix(kind))
        .map_or_else(|| rendered.clone(), str::to_string)
}

fn evaluated(source: &str) -> Result<String, String> {
    common::eval(source).map(without_kind)
}

fn evaluated_decoding(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    eval_script_decoding(&parsed.program, Budget::default(), |result| match result {
        Ok(value) => Some(without_kind(common::render(&value))),
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Throw(value) => Some(format!("throw {}", common::render(&value))),
            AbruptKind::Bail(_) => None,
        },
    })
}

fn node_on_script(source: &str) -> Option<String> {
    let quoted = serde_json::to_string(source).unwrap();
    printed_by_node(&format!(
        "try {{ console.log(String((0, eval)({quoted}))); }} \
         catch (e) {{ console.log('throw error:' + e.constructor.name); }}"
    ))
    .map(|text| text.trim_end_matches('\n').to_string())
}

fn answers_as_node(source: &str, node_prints: &str) {
    if let Some(printed) = node_on_script(source) {
        assert_eq!(printed, node_prints, "node on the input: {source}");
    }
    let outside = evaluated(source).unwrap_or_else(|thrown| thrown);
    assert_eq!(outside, node_prints, "outside decoding: {source}");
    if let Some(value) = evaluated_decoding(source) {
        assert_eq!(value, node_prints, "while decoding: {source}");
    }
}

fn answers_as_node_when_decoding(source: &str, node_prints: &str) {
    if let Some(printed) = node_on_script(source) {
        assert_eq!(printed, node_prints, "node on the input: {source}");
    }
    if let Some(value) = evaluated_decoding(source) {
        assert_eq!(value, node_prints, "while decoding: {source}");
    }
}

fn answers_as_node_or_bails(source: &str, node_prints: &str) {
    if let Some(printed) = node_on_script(source) {
        assert_eq!(printed, node_prints, "node on the input: {source}");
    }
    match evaluated(source) {
        Ok(value) => assert_eq!(value, node_prints, "outside decoding: {source}"),
        Err(thrown) if thrown.starts_with("throw ") => {
            assert_eq!(thrown, node_prints, "outside decoding: {source}");
        }
        Err(_) => {}
    }
    if let Some(value) = evaluated_decoding(source) {
        assert_eq!(value, node_prints, "while decoding: {source}");
    }
}

#[test]
fn a_read_gives_the_same_object_every_time() {
    answers_as_node("function f() {} f.prototype === f.prototype", "true");
    answers_as_node(
        "var f = function () {}; var p = f.prototype; f.prototype === p && typeof p === 'object'",
        "true",
    );
    answers_as_node("function f() {} f.prototype.x = 4; f.prototype.x", "4");
}

#[test]
fn new_before_any_read_uses_the_prototype_a_later_read_returns() {
    answers_as_node("function f() { this.a = 1; } new f().a", "1");
    answers_as_node(
        "function f() {} var o = new f(); Object.getPrototypeOf(o) === f.prototype",
        "true",
    );
    answers_as_node(
        "function f() {} var o = new f(); f.prototype.x = 6; o.x",
        "6",
    );
    answers_as_node(
        "function f() {} var a = new f(); var b = new f(); Object.getPrototypeOf(a) === Object.getPrototypeOf(b)",
        "true",
    );
}

#[test]
fn a_write_of_prototype_is_what_new_then_uses() {
    answers_as_node(
        "function f() { this.a = 2; } f.prototype = 1; new f().a + typeof f.prototype",
        "2number",
    );
    answers_as_node_or_bails(
        "function f() {} f.prototype = 1; Object.getPrototypeOf(new f()) === Object.prototype",
        "true",
    );
    answers_as_node("function f() {} f.prototype = {x: 5}; new f().x", "5");
    answers_as_node(
        "function f() {} f.prototype = 5; typeof f.prototype",
        "number",
    );
    answers_as_node(
        "function f() {} f.prototype = {x: 1}; f.hasOwnProperty('prototype'); new f().x",
        "1",
    );
    answers_as_node(
        "function f() {} f.prototype = {x: 1}; delete f.prototype; new f().x",
        "1",
    );
    answers_as_node(
        "function f() {} var before = f.prototype; f.prototype = {}; f.prototype === before",
        "false",
    );
}

#[test]
fn a_write_of_prototype_keeps_it_hidden_and_fixed() {
    answers_as_node(
        "function f() {} f.prototype = {}; Object.keys(f).length",
        "0",
    );
    answers_as_node(
        "function f() {} f.prototype = {}; var n = 0; for (var k in f) { n = n + 1; } n",
        "0",
    );
    answers_as_node(
        "function f() {} f.prototype = {}; delete f.prototype",
        "false",
    );
    answers_as_node(
        "function f() {} f.prototype = 1; f.a = 2; Object.keys(f).join('|')",
        "a",
    );
    answers_as_node_or_bails(
        "function f() {} Object.assign(f, {prototype: 3}); Object.keys(f).length + typeof f.prototype",
        "0number",
    );
}

#[test]
fn instanceof_reaches_the_lazy_prototype() {
    answers_as_node("function f() {} new f() instanceof f", "true");
    answers_as_node("function f() {} ({}) instanceof f", "false");
    answers_as_node(
        "function f() {} var o = new f(); f.prototype = {}; o instanceof f",
        "false",
    );
    answers_as_node(
        "function f() {} var o = new f(); var p = f.prototype; f.prototype = 1; o instanceof f",
        "throw error:TypeError",
    );
}

#[test]
fn instanceof_with_a_generator_or_bound_function_answers_as_node() {
    answers_as_node_or_bails("function* g() {} ({}) instanceof g", "false");
    answers_as_node("function f() {} ({}) instanceof f.bind()", "false");
    answers_as_node("function f() {} new f() instanceof f.bind()", "true");
}

#[test]
fn get_prototype_of_sees_the_prototype_a_new_made() {
    answers_as_node(
        "function f() {} var p = Object.getPrototypeOf(new f()); p === f.prototype && p !== Object.prototype",
        "true",
    );
    answers_as_node_or_bails(
        "function f() {} Object.getPrototypeOf(f) === Function.prototype",
        "true",
    );
}

#[test]
fn own_property_questions_count_the_prototype() {
    answers_as_node("function f() {} f.hasOwnProperty('prototype')", "true");
    answers_as_node("function f() {} 'prototype' in f", "true");
    answers_as_node("var a = () => 1; 'prototype' in a", "false");
    answers_as_node("var a = () => 1; a.hasOwnProperty('prototype')", "false");
    answers_as_node(
        "function f() {} f.a = 1; Object.keys(f).join() + f.hasOwnProperty('prototype')",
        "atrue",
    );
}

#[test]
fn enumeration_never_lists_the_prototype() {
    answers_as_node("function f() {} Object.keys(f).length", "0");
    answers_as_node(
        "function f() {} var n = 0; for (var k in f) { n = n + 1; } n",
        "0",
    );
    answers_as_node("function f() {} JSON.stringify(Object.entries(f))", "[]");
    answers_as_node_when_decoding(
        "function f() {} f.a = 1; Object.getOwnPropertyNames(f).indexOf('prototype') >= 0",
        "true",
    );
    answers_as_node_when_decoding(
        "function f() {} f.a = 1; Object.getOwnPropertyNames(f).join()",
        "length,name,arguments,caller,prototype,a",
    );
}

#[test]
fn define_property_sees_the_default_prototype_attributes() {
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {value: 7}); f.prototype",
        "7",
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {value: 7, writable: false}); f.prototype = 8; f.prototype",
        "7",
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {configurable: true})",
        TYPE_ERROR,
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {enumerable: true})",
        TYPE_ERROR,
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {value: 2, writable: true}); Object.keys(f).length + '' + f.prototype",
        "02",
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {writable: false}); typeof f.prototype",
        "object",
    );
}

#[test]
fn delete_of_the_prototype_is_refused() {
    answers_as_node("function f() {} delete f.prototype", "false");
    answers_as_node(
        "function f() {} delete f.prototype; typeof f.prototype",
        "object",
    );
    answers_as_node(
        "'use strict'; function f() {} delete f.prototype",
        TYPE_ERROR,
    );
    answers_as_node("var a = () => 1; delete a.prototype", "true");
}

#[test]
fn freeze_and_is_frozen_keep_the_prototype() {
    answers_as_node("function f() {} Object.isFrozen(f)", "false");
    answers_as_node_or_bails(
        "function f() {} Object.freeze(f); f.prototype = 1; typeof f.prototype",
        "object",
    );
    answers_as_node_or_bails(
        "function f() {} Object.freeze(f); Object.isFrozen(f)",
        "true",
    );
}

#[test]
fn a_function_as_a_prototype_link_shows_its_own_prototype() {
    answers_as_node(
        "function g() {} g.q = 9; function f() {} f.prototype = g; new f().q",
        "9",
    );
    answers_as_node_or_bails(
        "function g() {} var o = Object.create(g); o.prototype === g.prototype",
        "true",
    );
    answers_as_node_or_bails(
        "function g() {} var o = Object.create(g); typeof o.prototype",
        "object",
    );
    answers_as_node(
        "function g() {} function f() {} f.prototype = g; typeof new f().prototype",
        "object",
    );
    answers_as_node_or_bails(
        "function g() {} var o = Object.create(g); 'prototype' in o",
        "true",
    );
    answers_as_node_or_bails(
        "function g() {} var o = Object.create(g); o.hasOwnProperty('prototype')",
        "false",
    );
}

#[test]
fn a_bound_function_and_a_method_take_no_prototype_of_their_own() {
    answers_as_node_or_bails("function f() {} typeof f.bind(null).prototype", "undefined");
    answers_as_node("var o = {m() {}}; typeof o.m.prototype", "undefined");
    answers_as_node("var a = () => 1; typeof a.prototype", "undefined");
}

#[test]
fn a_prototype_write_through_a_sloppy_and_a_strict_path_agree() {
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {value: 1, writable: false}); f.prototype = 2; f.prototype",
        "1",
    );
    answers_as_node_or_bails(
        "function f() {} Object.defineProperty(f, 'prototype', {value: 1, writable: false}); (function () { 'use strict'; f.prototype = 2; })()",
        TYPE_ERROR,
    );
}

fn live_objects_after(source: &str) -> Vec<usize> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = &parsed.program;
    let mut interp = Interp::new(program.source_text, Budget::default());
    let env = interp.global_env();
    interp.hoist(&program.body, &env).expect("hoists");
    let mut counts = vec![interp.realm.live_tracked().0];
    for statement in &program.body {
        interp.run_root_statement(statement).expect("runs");
        counts.push(interp.realm.live_tracked().0);
    }
    counts
}

#[test]
fn a_closure_that_never_touches_prototype_makes_no_prototype_object() {
    let counts = live_objects_after(
        "function a() {}\nfunction b() {}\nvar c = function () {};\nb.x = 1;\na.call(null);\nb.hasOwnProperty('x');\n",
    );
    let baseline = counts[0];
    assert_eq!(counts[1..3], [baseline; 2], "{counts:?}");
    assert_eq!(counts[3..], [baseline + 1; 4], "{counts:?}");
}

#[test]
fn the_first_touch_makes_exactly_one_prototype_object() {
    let counts = live_objects_after(
        "function a() {}\nvar x = a.prototype;\nvar y = a.prototype;\nnew a();\n",
    );
    assert_eq!(counts[1], counts[0], "{counts:?}");
    assert_eq!(counts[2], counts[1] + 1, "{counts:?}");
    assert_eq!(counts[3], counts[2], "{counts:?}");
    assert_eq!(counts[4], counts[3], "{counts:?}");
}
