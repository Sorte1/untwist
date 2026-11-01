mod common;

use common::eval;

#[test]
fn computed_and_static_member_reads() {
    assert_eq!(eval("var e = ['a', 'b']; e[1]"), Ok("string:b".to_string()));
    assert_eq!(
        eval("var e = ['a']; e['length']"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("var k = { c: 0x2f2 }; k.c"),
        Ok("number:754".to_string())
    );
    assert_eq!(eval("var k = { c: 1 }; k['c']"), Ok("number:1".to_string()));
}

#[test]
fn out_of_range_and_string_index_reads_match_js() {
    assert_eq!(eval("var e = ['a']; e[7]"), Ok("undefined".to_string()));
    assert_eq!(
        eval("var e = ['a', 'b']; e['1']"),
        Ok("string:b".to_string())
    );
    assert_eq!(eval("'ab'.length"), Ok("number:2".to_string()));
    assert_eq!(eval("'\\uD800'.length"), Ok("number:1".to_string()));
}

#[test]
fn reading_a_property_of_undefined_throws_a_catchable_type_error() {
    assert_eq!(
        eval("var e = []; e[0].length"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn optional_chaining_short_circuits() {
    assert_eq!(eval("var a = null; a?.b"), Ok("undefined".to_string()));
    assert_eq!(eval("var a = { b: 2 }; a?.b"), Ok("number:2".to_string()));
}

#[test]
fn multi_hop_optional_chains_short_circuit_the_whole_chain() {
    assert_eq!(eval("var a; a?.b.c"), Ok("undefined".to_string()));
    assert_eq!(eval("var a = null; a?.[0].m"), Ok("undefined".to_string()));
    assert_eq!(
        eval("var a = { b: { c: 7 } }; a?.b.c"),
        Ok("number:7".to_string())
    );
    assert_eq!(eval("var a = {}; a.b?.c.d"), Ok("undefined".to_string()));
}

#[test]
fn optional_chain_keys_evaluate_once_only_when_not_short_circuited() {
    assert_eq!(
        eval("var a = { b: { c: 7 } }; var n = 0; a?.b[(n++, \"c\")]; n"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("var a; var n = 0; a?.b[(n++, \"c\")]; n"),
        Ok("number:0".to_string())
    );
}

#[test]
fn parentheses_end_an_optional_chain() {
    assert_eq!(
        eval("var a; (a?.b).c"),
        Err("throw error:TypeError".to_string())
    );
}

#[test]
fn in_and_instanceof_use_the_object_model() {
    assert_eq!(
        eval("var k = { c: 1 }; 'c' in k"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("var k = { c: 1 }; 'd' in k"),
        Ok("boolean:false".to_string())
    );
    assert_eq!(
        eval("var e = ['a']; 0 in e"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn instanceof_walks_the_prototype_chain_and_bails_past_the_cap() {
    assert_eq!(
        eval("function C() {} var o = new C(); o instanceof C"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("function C() {} function D() {} var o = new C(); o instanceof D"),
        Ok("boolean:false".to_string())
    );
    assert_eq!(
        eval("function C() {} var o = Object['create'](Object['create'](C['prototype'])); o instanceof C"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval(
            "function C() {} var o = C['prototype']; for (var i = 0; i < 100; i++) { o = Object['create'](o); } o instanceof C"
        ),
        Err("prototype chain too deep".to_string())
    );
}

#[test]
fn instanceof_object_is_callable() {
    assert_eq!(
        eval("'x' instanceof Object"),
        Ok("boolean:false".to_string())
    );
}

const IMPLICIT_PROTOTYPE: &str = "unsupported construct: a prototype the evaluator leaves implicit";

#[test]
fn instanceof_across_the_builtin_prototypes_reaches_object_prototype() {
    for source in [
        "[] instanceof Object",
        "(function () {}) instanceof Object",
        "/a/ instanceof Object",
        "Object instanceof Object",
        "Object['getPrototypeOf'](Object['getPrototypeOf']([])) === Object['prototype']",
        "Object['getPrototypeOf'](RegExp['prototype']) === Object['prototype']",
        "'a,b'['split'](',') instanceof Object",
        "(function () { return arguments; })() instanceof Object",
        "(function () {})['bind'](null) instanceof Object",
    ] {
        assert_eq!(eval(source), Ok("boolean:true".to_string()), "{source}");
    }
    assert_eq!(
        eval("[] instanceof RegExp"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn a_prototype_left_unset_on_a_plain_object_or_an_error_still_refuses_outside_a_decode() {
    for source in [
        "Object['getPrototypeOf'](JSON['parse']('{}')) === Object['prototype']",
        "(function () { try { null['x']; } catch (e) { return e instanceof Object; } })()",
    ] {
        assert_eq!(
            eval(source),
            Err(IMPLICIT_PROTOTYPE.to_string()),
            "{source}"
        );
    }
}

#[test]
fn object_called_or_constructed_has_object_prototype() {
    for source in [
        "new Object() instanceof Object",
        "Object() instanceof Object",
        "Object['getPrototypeOf'](new Object()) === Object['prototype']",
        "({}) instanceof Object",
    ] {
        assert_eq!(eval(source), Ok("boolean:true".to_string()), "{source}");
    }
}
