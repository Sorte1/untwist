mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::eval_script_decoding;
use untwist_eval::value::{AbruptKind, Attrs, JsString, PropKey, PropMap, StoredKey, Value};
use untwist_testing::node::printed_by_node;

const TYPE_ERROR: &str = "throw error:TypeError";

fn without_kind(rendered: String) -> String {
    ["string:", "number:", "boolean:"]
        .iter()
        .find_map(|kind| rendered.strip_prefix(kind))
        .map_or_else(|| rendered.clone(), str::to_string)
}

fn render(value: &Value<'_>) -> String {
    without_kind(common::render(value))
}

fn evaluated(source: &str) -> Result<String, String> {
    common::eval(source).map(without_kind)
}

fn evaluated_decoding(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    eval_script_decoding(&parsed.program, Budget::default(), |result| match result {
        Ok(value) => Some(render(&value)),
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

fn answers_as_node_or_bails(source: &str, node_prints: &str) {
    if let Some(printed) = node_on_script(source) {
        assert_eq!(printed, node_prints, "node on the input: {source}");
    }
    if let Ok(value) = evaluated(source) {
        assert_eq!(value, node_prints, "outside decoding: {source}");
    }
    if let Err(thrown) = evaluated(source) {
        if thrown.starts_with("throw ") {
            assert_eq!(thrown, node_prints, "outside decoding: {source}");
        }
    }
    if let Some(value) = evaluated_decoding(source) {
        assert_eq!(value, node_prints, "while decoding: {source}");
    }
}

#[test]
fn a_function_has_no_enumerable_own_key() {
    answers_as_node("Object.keys(function(){}).length", "0");
    answers_as_node(
        "function F() {} F.a = 1; Object.keys(F).join('|') + ';' + Object.entries(F)[0][0]",
        "a;a",
    );
    answers_as_node(
        "function F() {} F.a = 1; var s = ''; for (var k in F) { s = s + k + '|'; } s",
        "a|",
    );
    answers_as_node(
        "function F() {} F.a = 1; Object.keys(Object.assign({}, F)).join('|')",
        "a",
    );
    answers_as_node("function F() {} JSON.stringify(Object.keys(F))", "[]");
}

#[test]
fn for_in_over_an_array_omits_its_length() {
    answers_as_node(
        "var s = ''; for (var k in [1, 2]) { s = s + k + ','; } s",
        "0,1,",
    );
}

#[test]
fn builtins_install_their_spec_attributes() {
    answers_as_node(
        "Object.keys(Math).length + '|' + Object.keys(JSON).length + '|' + Object.keys(Object).length",
        "0|0|0",
    );
    answers_as_node("Math.PI = 1; Math.PI", "3.141592653589793");
    answers_as_node("delete Math.PI", "false");
    answers_as_node("'use strict'; Math.PI = 1", TYPE_ERROR);
    answers_as_node(
        "var e; try { null.x; } catch (x) { e = x; } Object.keys(e).length",
        "0",
    );
}

#[test]
fn a_function_prototype_is_writable_but_neither_enumerable_nor_configurable() {
    answers_as_node("function F() {} delete F.prototype", "false");
    answers_as_node(
        "function F() {} delete F.prototype; 'prototype' in F",
        "true",
    );
    answers_as_node(
        "function F() {} F.prototype = 7; F.a = 1; Object.keys(F).join('|') + typeof F.prototype",
        "anumber",
    );
    answers_as_node(
        "'use strict'; function F() {} delete F.prototype",
        TYPE_ERROR,
    );
}

#[test]
fn a_defined_value_is_neither_enumerable_nor_writable_and_json_skips_it() {
    answers_as_node(
        "var o = { a: 1 }; Object.defineProperty(o, 'x', { value: 2 }); o.x = 3; \
         o.x + '|' + Object.keys(o).join(',') + '|' + JSON.stringify(o)",
        "2|a|{\"a\":1}",
    );
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 2, writable: true }); o.x = 3; o.x",
        "3",
    );
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 2, enumerable: true }); \
         Object.keys(o).join(',')",
        "x",
    );
}

#[test]
fn an_array_element_takes_a_definition_only_with_its_plain_attributes() {
    answers_as_node(
        "var a = [1]; Object.defineProperty(a, '0', { value: 2, writable: true, enumerable: true, configurable: true }); a[0] + '|' + Object.keys(a).join()",
        "2|0",
    );
    answers_as_node_or_bails(
        "var a = [1]; Object.defineProperty(a, '0', { value: 2 }); a[0] + '|' + Object.keys(a).join()",
        "2|0",
    );
    answers_as_node_or_bails(
        "var a = []; Object.defineProperty(a, '0', { value: 2 }); a[0] + '|' + Object.keys(a).length",
        "2|0",
    );
}

#[test]
fn redefining_a_fixed_property_throws_unless_it_changes_nothing() {
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
         Object.defineProperty(o, 'x', { value: 1 }); o.x",
        "1",
    );
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
         Object.defineProperty(o, 'x', { value: 2 })",
        TYPE_ERROR,
    );
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
         Object.defineProperty(o, 'x', { enumerable: true })",
        TYPE_ERROR,
    );
}

#[test]
fn deleting_a_fixed_property_answers_false_and_throws_in_strict_code() {
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); (delete o.x) + '|' + o.x",
        "false|1",
    );
    answers_as_node(
        "'use strict'; var o = {}; Object.defineProperty(o, 'x', { value: 1 }); delete o.x",
        TYPE_ERROR,
    );
    answers_as_node(
        "var o = { x: 1 }; (delete o.x) + '|' + ('x' in o)",
        "true|false",
    );
}

#[test]
fn a_strict_write_to_a_read_only_property_throws_and_a_sloppy_one_is_ignored() {
    answers_as_node(
        "'use strict'; var o = {}; Object.defineProperty(o, 'x', { value: 1 }); o.x = 2",
        TYPE_ERROR,
    );
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); o.x = 2; o.x",
        "1",
    );
    answers_as_node(
        "'use strict'; var p = {}; Object.defineProperty(p, 'x', { value: 1 }); \
         var o = Object.create(p); o.x = 2",
        TYPE_ERROR,
    );
    answers_as_node(
        "var p = {}; Object.defineProperty(p, 'x', { value: 1 }); \
         var o = Object.create(p); o.x = 2; o.x + '|' + o.hasOwnProperty('x')",
        "1|false",
    );
}

#[test]
fn a_strict_write_to_a_getter_only_accessor_throws_and_a_sloppy_one_is_ignored() {
    answers_as_node_or_bails(
        "'use strict'; var o = { get x() { return 1; } }; o.x = 2",
        TYPE_ERROR,
    );
    answers_as_node_or_bails("var o = { get x() { return 1; } }; o.x = 2; o.x", "1");
    answers_as_node_or_bails(
        "'use strict'; var o = {}; Object.defineProperty(o, 'x', { get: function () { return 1; } }); o.x = 2",
        TYPE_ERROR,
    );
}

#[test]
fn a_frozen_object_refuses_writes_and_a_strict_write_throws() {
    answers_as_node(
        "var o = Object.freeze({ a: 1 }); o.a = 2; o.b = 3; \
         [o.a, typeof o.b, Object.isFrozen(o), Object.isFrozen({}), delete o.a].join('|')",
        "1|undefined|true|false|false",
    );
    answers_as_node(
        "'use strict'; var o = Object.freeze({ a: 1 }); o.a = 2",
        TYPE_ERROR,
    );
    answers_as_node(
        "'use strict'; var o = Object.freeze({ a: 1 }); o.b = 2",
        TYPE_ERROR,
    );
    answers_as_node(
        "Object.isFrozen(1) + '|' + Object.isFrozen(Object.freeze({})) + '|' + Object.isFrozen({ a: 1 })",
        "true|true|false",
    );
    answers_as_node(
        "var o = Object.freeze({}); Object.defineProperty(o, 'x', { value: 1 })",
        TYPE_ERROR,
    );
    answers_as_node(
        "var o = Object.freeze({ a: 1 }); Object.assign(o, { a: 2 })",
        TYPE_ERROR,
    );
}

#[test]
fn freezing_and_redefining_existing_keys_keep_their_order() {
    answers_as_node(
        "var o = { a: 1, b: 2, c: 3 }; Object.defineProperty(o, 'a', { value: 9 }); \
         Object.freeze(o); var s = ''; for (var k in o) { s = s + k; } \
         Object.keys(o).join('') + '|' + s + '|' + o.a",
        "abc|abc|9",
    );
    answers_as_node(
        "var o = { a: 1, b: 2, c: 3 }; Object.defineProperty(o, 'a', { enumerable: false }); \
         var hidden = Object.keys(o).join(''); \
         Object.defineProperty(o, 'a', { enumerable: true }); hidden + '|' + Object.keys(o).join('')",
        "bc|abc",
    );
}

#[test]
fn the_attribute_bits_take_no_part_in_a_keys_hash_or_equality() {
    use std::hash::BuildHasher;
    let hasher = rustc_hash::FxBuildHasher;
    let fixed = StoredKey::named("x", Attrs::CONSTANT);
    let plain = StoredKey::from(PropKey::Name(JsString::from_str("x")));
    let lookup = PropKey::Name(JsString::from_str("x"));
    assert_eq!(fixed, plain);
    assert_eq!(hasher.hash_one(&fixed), hasher.hash_one(&lookup));
    assert_eq!(
        hasher.hash_one(StoredKey::from(PropKey::Index(7))),
        hasher.hash_one(PropKey::Index(7))
    );
    let mut props: PropMap<'static> = PropMap::default();
    props.insert(fixed, Value::Number(1.0));
    props.insert(plain, Value::Number(2.0));
    assert_eq!(props.len(), 1);
    let (stored, value) = props.get_index(0).unwrap();
    assert!(!stored.attrs().writable() && !stored.attrs().configurable());
    assert!(matches!(value, Value::Number(n) if *n == 2.0));
    assert!(matches!(props.get(&lookup), Some(Value::Number(n)) if *n == 2.0));
}

#[test]
fn an_inherited_get_in_a_descriptor_is_an_accessor() {
    answers_as_node_or_bails(
        "var d = Object.create({ get: function () { return 5; } }); var t = {}; \
         Object.defineProperty(t, 'x', d); t.x",
        "5",
    );
}

#[test]
fn a_read_only_key_on_a_built_in_prototype_refuses_an_inherited_write() {
    answers_as_node(
        "var o = Object.create(Math); o.PI = 1; o.PI + '|' + o.hasOwnProperty('PI')",
        "3.141592653589793|false",
    );
}

#[test]
fn redefining_a_fixed_zero_as_negative_zero_throws() {
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 0 }); \
         Object.defineProperty(o, 'x', { value: -0 })",
        TYPE_ERROR,
    );
}

#[test]
fn a_fixed_key_cannot_become_configurable() {
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
         Object.defineProperty(o, 'x', { value: 1, configurable: true })",
        TYPE_ERROR,
    );
}

#[test]
fn a_fixed_read_only_key_cannot_become_writable() {
    answers_as_node(
        "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); \
         Object.defineProperty(o, 'x', { writable: true })",
        TYPE_ERROR,
    );
}

#[test]
fn a_functions_name_and_length_do_not_take_a_write() {
    answers_as_node_or_bails(
        "function g(a) {} g.name = 'x'; g.length = 5; g.name + '|' + g.length",
        "g|1",
    );
    answers_as_node_or_bails("parseInt.name = 'x'; parseInt.name", "parseInt");
    answers_as_node_or_bails("'use strict'; function g(a) {} g.name = 'x'", TYPE_ERROR);
}
