mod common;

use common::{assert_scope_safe, rename};
use untwist_rename::{RenameConfig, RenameMode};

fn smart(source: &str) -> String {
    let config = RenameConfig {
        mode: RenameMode::Smart,
        rename_all: false,
        extended: true,
    };
    let output = rename(source, config);
    assert_scope_safe(source, &output);
    output
}

fn assert_has(code: &str, expected: &[&str]) {
    for want in expected {
        assert!(code.contains(want), "missing {want:?} in:\n{code}");
    }
}

fn assert_lacks(code: &str, unwanted: &[&str]) {
    for unwanted in unwanted {
        assert!(
            !code.contains(unwanted),
            "unexpected {unwanted:?} in:\n{code}"
        );
    }
}

#[test]
fn a_parameter_used_as_an_array_is_named_arr() {
    let code = smart("function _0xa1(_0xb1) { _0xb1.push(1); return _0xb1.length; } _0xa1([]);");
    assert_has(&code, &["(arr) {", "arr.push(1)"]);
    let code =
        smart("function _0xa1(_0xb1, _0xc1) { return _0xb1[_0xc1] + _0xb1.length; } _0xa1([], 0);");
    assert_has(&code, &["(arr, "]);
}

#[test]
fn a_parameter_used_with_string_methods_is_named_str() {
    let code = smart("function _0xa1(_0xb1) { return _0xb1.toLowerCase().length + _0xb1.charCodeAt(0); } _0xa1('x');");
    assert_has(&code, &["(str) {"]);
}

#[test]
fn a_parameter_only_awaited_through_then_is_named_promise() {
    let code = smart("function _0xa1(_0xb1) { return _0xb1.then(function () {}); } _0xa1(null);");
    assert_has(&code, &["(promise) {"]);
}

#[test]
fn a_parameter_read_for_coordinates_is_named_point_or_vec() {
    let code = smart("function _0xa1(_0xb1) { return _0xb1.x + _0xb1.y; } _0xa1({});");
    assert_has(&code, &["(point) {"]);
    let code = smart("function _0xa1(_0xb1) { return _0xb1.x + _0xb1.y + _0xb1.z; } _0xa1({});");
    assert_has(&code, &["(vec) {"]);
}

#[test]
fn a_parameter_used_only_in_arithmetic_is_named_num() {
    let code =
        smart("function _0xa1(_0xb1, _0xc1) { return _0xb1 * 2 - (_0xc1 - 1); } _0xa1(1, 2);");
    assert_has(&code, &["(num, num2) {"]);
}

#[test]
fn a_parameter_with_event_reads_is_named_event() {
    let code =
        smart("function _0xa1(_0xb1) { _0xb1.preventDefault(); return _0xb1.target; } _0xa1({});");
    assert_has(&code, &["(event) {"]);
}

#[test]
fn a_parameter_read_for_plain_properties_is_named_obj() {
    let code = smart("function _0xa1(_0xb1) { return _0xb1.players + _0xb1.account; } _0xa1({});");
    assert_has(&code, &["(obj) {"]);
}

#[test]
fn conflicting_usage_keeps_the_fallback() {
    let code =
        smart("function _0xa1(_0xb1) { _0xb1.push(1); return _0xb1.toLowerCase(); } _0xa1([]);");
    assert_has(&code, &["(v) {"]);
    assert_lacks(&code, &["getToLowerCase"]);
    let code =
        smart("function _0xa1(_0xb1) { return _0xb1 * 2 + g(_0xb1); } function g() {} _0xa1(1);");
    assert_has(&code, &["(v)"]);
    let code = smart("function _0xa1(_0xb1) { return _0xb1.x + _0xb1.y + _0xb1.foo; } _0xa1({});");
    assert_lacks(&code, &["point", "vec"]);
}

#[test]
fn a_function_returning_a_construction_is_a_factory_and_its_result_takes_the_type() {
    let code =
        smart("class Widget {} function _0xa1() { return new Widget(); } var _0xb1 = _0xa1();");
    assert_has(
        &code,
        &["function createWidget()", "var widget = createWidget()"],
    );
}

#[test]
fn a_function_returning_a_property_is_a_getter_and_its_result_takes_the_property() {
    let code =
        smart("var s = {}; function _0xa1(_0xc1) { return _0xc1.position; } var _0xb1 = _0xa1(s);");
    assert_has(
        &code,
        &["function getPosition(", "var position", " = getPosition("],
    );
}

#[test]
fn a_function_returning_a_named_local_takes_its_name() {
    let code = smart(
        "function _0xa1() { var _0xd1 = document.createDocumentFragment(); _0xd1.x = 1; return _0xd1; } var _0xb1 = _0xa1();",
    );
    assert_has(
        &code,
        &[
            "function createDocumentFragment",
            "var documentFragment = createDocumentFragment",
        ],
    );
    let code = smart(
        "function _0xa1(_0xe1) { var _0xd1 = _0xe1.fragment; return _0xd1; } var _0xb1 = _0xa1(document);",
    );
    assert_has(&code, &["function getFragment("]);
}

#[test]
fn a_forwarding_wrapper_takes_the_name_of_what_it_forwards_to() {
    let code = smart("function _0xa1(_0xc1) { return document.createTextNode(_0xc1); } function _0xd1(_0xc1) { return _0xa1(_0xc1); } var _0xb1 = _0xd1('x');");
    assert_has(
        &code,
        &["function createTextNode", "var textNode = createTextNode"],
    );
}

#[test]
fn a_predicate_result_is_a_boolean() {
    let code = smart("function _0xa1(_0xc1) { return _0xc1 > 1; } var _0xb1 = _0xa1(2);");
    assert_has(&code, &["function predicate(", "var bool = predicate(2)"]);
    let code = smart("function _0xa1(_0xc1) { return _0xc1 instanceof Widget; } class Widget {} var _0xb1 = _0xa1(2);");
    assert_has(
        &code,
        &["function isWidget(", "var isWidget2 = isWidget(2)"],
    );
}

#[test]
fn a_constructed_function_is_not_named_from_its_return() {
    let code = smart("function _0xa1() { this.q = 1; return new Widget(); } class Widget {} var _0xb1 = new _0xa1();");
    assert_lacks(&code, &["createWidget"]);
}

#[test]
fn an_argument_takes_the_name_of_the_parameter_it_is_passed_to() {
    let code = smart("function _0xa1(_0xb1) { _0xb1.push(1); } function _0xd1(_0xc1) { _0xa1(_0xc1); } _0xd1([]);");
    assert_has(&code, &["(arr) {\n  arr.push(1);", "(arr) {\n  fn(arr);"]);
}

#[test]
fn an_argument_passed_to_differently_named_parameters_keeps_the_fallback() {
    let code = smart("function _0xa1(_0xb1) { _0xb1.push(1); } function _0xe1(_0xb1) { return _0xb1.toLowerCase(); } function _0xd1(_0xc1) { _0xa1(_0xc1); _0xe1(_0xc1); } _0xd1([]);");
    assert_has(&code, &["(v) {"]);
}

#[test]
fn a_constructed_parameter_is_not_named_obj() {
    let code = smart("function _0xa1(_0xb1, _0xc1) { return new _0xb1(_0xc1) + _0xb1.BYTES_PER_ELEMENT; } _0xa1(Array, 1);");
    assert_lacks(&code, &["(obj, "]);
    assert_has(&code, &["(Ctor, "]);
}

#[test]
fn a_method_name_read_as_a_property_is_not_array_evidence() {
    let code = smart(
        "function _0xa1(_0xb1) { return _0xb1.map.spawns.length + _0xb1.map.size; } _0xa1({});",
    );
    assert_has(&code, &["(obj) {"]);
    assert_lacks(&code, &["(arr)"]);
}

#[test]
fn a_passed_name_that_the_value_use_contradicts_is_not_taken() {
    let code = smart(
        "function _0xa1() { function _0xb1(_0xc1) { return _0xc1 * 2; } function _0xd1(_0xe1) { return _0xe1.length + _0xe1.x + _0xb1(_0xe1); } return _0xd1('ab'); }",
    );
    assert_has(&code, &["(num) {"]);
    assert_lacks(&code, &["(num2)", "num.length"]);
}

#[test]
fn only_a_quiet_function_is_named_as_a_getter() {
    let code = smart(
        "function _0xa1() { function _0xb1(_0xc1) { _0xc1.count++; return _0xc1.total; } function _0xd1(_0xe1) { return _0xe1.total; } return _0xb1({}) + _0xd1({}); }",
    );
    assert_has(&code, &["function getTotal(obj) {\n    return obj.total;"]);
    assert_lacks(
        &code,
        &["function getTotal(obj) {\n    obj.count++", "getTotal2"],
    );
}

#[test]
fn a_cached_property_getter_is_still_a_getter() {
    let code = smart(
        "function _0xa1() { function _0xb1(_0xc1) { if (!_0xc1.cache) _0xc1.cache = new Map(); return _0xc1.cache; } return _0xb1({}); }",
    );
    assert_has(&code, &["function getCache("]);
}

#[test]
fn wrappers_of_one_function_are_told_apart_or_left_unnamed() {
    let code = smart(
        "function _0xa1() { function _0xa(_0xb) { return _0xb.items; } function _0xc(_0xd) { return _0xa(_0xd); } function _0xe(_0xf) { return _0xa(_0xf.inner); } function _0x5(_0x6) { return _0xa('logic', _0x6); } return [_0xc, _0xe, _0x5]; }",
    );
    assert_has(
        &code,
        &[
            "function getItems(",
            "function getItems2(",
            "function getItemsLogic(",
        ],
    );
    assert_lacks(&code, &["getItems3", "getItems4"]);
}

#[test]
fn a_parameter_that_is_constructed_is_a_constructor_even_when_also_called() {
    let code = smart(
        "function _0xa1(_0xb1, _0xd1) { const _0xc1 = new _0xb1(1); if (_0xc1.ok) _0xb1(2); const _0xe1 = _0xd1(); return [_0xc1, _0xe1]; } _0xa1(function () {}, function () {});",
    );
    assert_has(
        &code,
        &[
            "(Ctor, cb)",
            "const instance = new Ctor(1)",
            "const result = cb()",
        ],
    );
    assert_lacks(&code, &["cb2", "cb3"]);
}

#[test]
fn a_reassigned_call_result_does_not_keep_the_method_name() {
    let code = smart(
        "function _0xa1(_0xb1) { let _0x13 = _0xb1.get('a'); _0x13 = _0x13 || 0; let _0x14 = Math.round(_0xb1.x); if (_0x14) { _0x14 = 2; } let _0x15 = _0xb1.getSkin(); _0x15 = _0x15 || {}; return [_0x13, _0x14, _0x15]; } _0xa1(null);",
    );
    assert_has(&code, &["let num = Math.round", "let skin = "]);
    assert_lacks(&code, &["let get = ", "let round = "]);
    let code = smart(
        "function _0xa1(_0xb1) { let _0x14 = Math.round(_0xb1.x); _0x14 = _0x14 * 2; let _0x15 = _0xb1.load(); _0x15 = _0xb1.load(); return [_0x14, _0x15]; } _0xa1(null);",
    );
    assert_has(&code, &["let round = ", "let load = "]);
}
