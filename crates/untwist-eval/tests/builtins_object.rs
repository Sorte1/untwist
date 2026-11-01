mod common;

use common::{eval, eval_with};
use untwist_eval::budget::Budget;

#[test]
fn object_keys_matches_for_in_order() {
    assert_eq!(
        eval("Object['keys']({ a: 1, b: 2 })['join']('')"),
        Ok("string:ab".to_string())
    );
    assert_eq!(
        eval("var o = { a: 1 }; var seen = ''; for (var k in o) seen += k; seen + Object['keys'](o)[0]"),
        Ok("string:aa".to_string())
    );
}

#[test]
fn has_own_property_and_object_to_string() {
    assert_eq!(
        eval("({ a: 1 })['hasOwnProperty']('a')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(eval("'' + {}"), Ok("string:[object Object]".to_string()));
    assert_eq!(eval("'' + []"), Ok("string:".to_string()));
    assert_eq!(eval("[1,2] + [3]"), Ok("string:1,23".to_string()));
}

#[test]
fn assign_and_freeze_and_create() {
    assert_eq!(
        eval("var t = {}; Object['assign'](t, { a: 1 }); t.a"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("Object['freeze']({ a: 1 }).a"),
        Ok("number:1".to_string())
    );
    assert!(eval("Object['freeze']([1]).length").is_err());
    assert!(eval("typeof Object['create'](null)").is_err());
    assert_eq!(
        eval("typeof Object['create']({})"),
        Ok("string:object".to_string())
    );
}

#[test]
fn json_round_trips() {
    assert_eq!(
        eval("JSON['parse']('{\"a\":[1,2]}').a[1]"),
        Ok("number:2".to_string())
    );
    assert_eq!(
        eval("JSON['stringify']({ a: 1 })"),
        Ok("string:{\"a\":1}".to_string())
    );
    assert_eq!(
        eval("JSON['stringify']([1,'x'])"),
        Ok("string:[1,\"x\"]".to_string())
    );
}

#[test]
fn json_stringify_rejects_cyclic_structures() {
    assert_eq!(
        eval("var o = {}; o.self = o; JSON['stringify'](o)"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn json_parse_bails_on_pathological_nesting() {
    let source = format!("JSON['parse']('{}')", "[".repeat(200_000));
    assert_eq!(eval(&source), Err("nesting depth cap exceeded".to_string()));
}

#[test]
fn json_parse_bounds_array_breadth_by_fuel() {
    let source = "JSON['parse']('[' + '1,'['repeat'](20000) + '1]')";
    assert_eq!(
        eval_with(source, Budget::new(10_000)),
        Err("step budget exhausted".to_string())
    );
}

#[test]
fn json_parse_bounds_string_length_by_fuel() {
    let source = "JSON['parse']('\"' + 'a'['repeat'](20000) + '\"')";
    assert_eq!(
        eval_with(source, Budget::new(10_000)),
        Err("step budget exhausted".to_string())
    );
}

#[test]
fn get_prototype_of_reflects_object_create_and_plain_objects() {
    assert_eq!(
        eval("var p = {}; Object['getPrototypeOf'](Object['create'](p)) === p"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("Object['getPrototypeOf']({}) !== null"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn json_parse_rejects_malformed_numbers_and_raw_control_characters() {
    assert_eq!(
        eval("JSON['parse']('01')"),
        Err("throw error:SyntaxError".to_string())
    );
    assert_eq!(
        eval("JSON['parse']('+1')"),
        Err("throw error:SyntaxError".to_string())
    );
    assert_eq!(
        eval("JSON['parse']('\"' + String['fromCharCode'](9) + '\"')"),
        Err("throw error:SyntaxError".to_string())
    );
}

#[test]
fn json_parse_rejects_an_unknown_escape() {
    assert_eq!(
        eval("var r; try { r = JSON['parse']('\"\\\\x\"'); } catch (e) { r = 'threw'; } r"),
        Ok("string:threw".to_string())
    );
    assert_eq!(
        eval("JSON['parse']('\"\\\\n\"')['length']"),
        Ok("number:1".to_string())
    );
}

#[test]
fn json_stringify_escapes_lone_surrogates_and_refuses_to_json() {
    assert_eq!(
        eval("JSON['stringify']('\\ud800')['length']"),
        Ok("number:8".to_string())
    );
    assert_eq!(
        eval("JSON['stringify']({ toJSON: function () { return 1; } })"),
        Err("unsupported construct: toJSON".to_string())
    );
}
