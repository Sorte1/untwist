mod common;

use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::object::Consulted;
use untwist_eval::value::{IntrinsicProto, JsString, PropKey};

fn consulted(decoder_body: &str, index: u32) -> Vec<(IntrinsicProto, Consulted)> {
    let program = format!(
        "function a() {{ var h = ['log', 'x']; a = function () {{ return h; }}; return a(); }}\n\
         function b(c) {{ var e = a(); {decoder_body} }}\nb({index});"
    );
    common::with_program(&program, |parsed| {
        let decoder_program = common::decoder_program(parsed, "a", &["b"], None);
        let calls = common::calls_for(parsed, 0);
        let mut evaluator = InterpEvaluator::default();
        evaluator.decode(&decoder_program, &calls).expect("decode");
        evaluator.proto_reads.clone()
    })
}

fn decode_bails(decoder_body: &str, index: u32) -> bool {
    let program = format!(
        "function a() {{ var h = ['log', 'x']; a = function () {{ return h; }}; return a(); }}\n\
         function b(c) {{ var e = a(); {decoder_body} }}\nb({index});"
    );
    common::with_program(&program, |parsed| {
        let decoder_program = common::decoder_program(parsed, "a", &["b"], None);
        let calls = common::calls_for(parsed, 0);
        let mut evaluator = InterpEvaluator::default();
        evaluator.decode(&decoder_program, &calls).is_err()
    })
}

fn key(text: &str) -> Consulted {
    Consulted::Key(PropKey::Name(JsString::from_str(text)))
}

fn has(found: &[(IntrinsicProto, Consulted)], root: IntrinsicProto, what: &Consulted) -> bool {
    found.iter().any(|(r, w)| *r == root && w == what)
}

#[test]
fn an_in_range_element_read_consults_no_prototype() {
    assert!(consulted("return e[c];", 0).is_empty());
}

#[test]
fn an_out_of_range_read_consults_both_prototypes_at_its_index() {
    let found = consulted("return e[c];", 7);
    for root in [IntrinsicProto::Array, IntrinsicProto::Object] {
        assert!(has(&found, root, &Consulted::AnyIndex), "{found:?}");
    }
}

#[test]
fn a_native_call_consults_only_what_it_reads() {
    let found = consulted(
        "return String.fromCharCode.apply(null, [104]) + e[c].charAt(0);",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::String, &key("charAt")),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|(_, what)| *what == Consulted::Any),
        "{found:?}"
    );
}

#[test]
fn to_primitive_consults_the_to_primitive_marker() {
    let found = consulted(
        "var i = e[c]; var o = { valueOf: function () { return i; } }; return '' + o;",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("@@toPrimitive")),
        "{found:?}"
    );
}

#[test]
fn a_string_member_walks_on_to_the_object_prototype() {
    let found = consulted("var i = e[c]; return i.foo === undefined ? i : i.foo;", 0);
    assert!(
        has(&found, IntrinsicProto::String, &key("foo")),
        "{found:?}"
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("foo")),
        "{found:?}"
    );
}

#[test]
fn an_undefined_element_read_consults_the_chain_at_its_index() {
    let found = consulted(
        "var p = [void 0]; p[1] = e[c]; return p[0] === undefined ? p[1] : 'x';",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Array, &Consulted::AnyIndex),
        "{found:?}"
    );
}

#[test]
fn a_created_key_consults_the_setter_search() {
    let found = consulted("var o = {}; o['k' + c] = e[c]; return o['k' + c];", 0);
    assert!(has(&found, IntrinsicProto::Object, &key("k0")), "{found:?}");
}

#[test]
fn for_in_consults_every_key() {
    let found = consulted(
        "var n = 0; for (var k in {}) { n++; } return n ? 'enum' : e[c];",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Object, &Consulted::Any),
        "{found:?}"
    );
}

#[test]
fn to_primitive_consults_value_of_and_to_string() {
    let found = consulted("return '' + [e[c]];", 0);
    for (root, name) in [
        (IntrinsicProto::Array, "valueOf"),
        (IntrinsicProto::Object, "valueOf"),
        (IntrinsicProto::Array, "toString"),
    ] {
        assert!(has(&found, root, &key(name)), "{name}: {found:?}");
    }
}

#[test]
fn dropped_computed_keys_do_not_shadow_a_later_distinct_key() {
    let found = consulted(
        "var o = {}; \
         for (var i = 0; i < 64; i++) { var g = 'garbage' + i; o[g]; } \
         var k1 = 'alpha' + ''; \
         var k2 = 'beta' + ''; \
         o[k1]; \
         o[k2]; \
         return e[c];",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("alpha")),
        "{found:?}"
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("beta")),
        "{found:?}"
    );
}

#[test]
fn dropped_computed_keys_do_not_shadow_a_later_distinct_created_key() {
    let found = consulted(
        "var o = {}; \
         for (var i = 0; i < 64; i++) { var g = 'garbage' + i; o[g] = i; } \
         var k1 = 'alpha' + ''; \
         var k2 = 'beta' + ''; \
         o[k1] = 1; \
         o[k2] = 2; \
         return e[c];",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("alpha")),
        "{found:?}"
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("beta")),
        "{found:?}"
    );
}

#[test]
fn instanceof_consults_has_instance() {
    let found = consulted("return e[c] instanceof String;", 0);
    assert!(
        has(&found, IntrinsicProto::Function, &key("@@hasInstance")),
        "{found:?}"
    );
}

#[test]
fn array_concat_consults_is_concat_spreadable() {
    let found = consulted("return [].concat(e[c]);", 0);
    assert!(
        has(&found, IntrinsicProto::Array, &key("@@isConcatSpreadable")),
        "{found:?}"
    );
}

#[test]
fn array_map_consults_species() {
    let found = consulted(
        "var arr = []; arr.push(e[c]); return arr.map(function (x) { return x; });",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Array, &key("@@species")),
        "{found:?}"
    );
}

#[test]
fn object_to_string_consults_to_string_tag() {
    let found = consulted(
        "var o = { valueOf: function () { return e[c]; } }; return Object.prototype.toString.call(o);",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("@@toStringTag")),
        "{found:?}"
    );
}

#[test]
fn string_match_consults_the_match_symbol() {
    let found = consulted("var s = '' + e[c]; return s.match(/x/);", 0);
    assert!(
        has(&found, IntrinsicProto::RegExp, &key("@@match")),
        "{found:?}"
    );
}

#[test]
fn for_of_consults_the_iterator_symbol() {
    let found = consulted("var n = 0; for (var v of [e[c]]) { n++; } return n;", 0);
    assert!(
        has(&found, IntrinsicProto::Array, &key("@@iterator")),
        "{found:?}"
    );
}

#[test]
fn regexp_exec_and_json_stringify_consult_the_spec_elided_reads() {
    let found = consulted(
        "var r = /x/; r.exec('' + e[c]); return JSON.stringify({ toJSON: e[c], v: 1 });",
        0,
    );
    assert!(
        has(&found, IntrinsicProto::RegExp, &key("exec")),
        "{found:?}"
    );
    assert!(
        has(&found, IntrinsicProto::Object, &key("toJSON")),
        "{found:?}"
    );
}

#[test]
fn a_boolean_property_read_bails_the_decode() {
    assert!(decode_bails("return true.valueOf ? e[c] : 'x';", 0));
}

#[test]
fn has_property_falls_back_through_the_intrinsic_member_chain() {
    let found = consulted("return ('length' in e) ? e[c] : 'x';", 0);
    assert!(
        has(&found, IntrinsicProto::Array, &key("length")),
        "{found:?}"
    );
}

#[test]
fn every_array_creating_method_consults_the_constructor_and_its_species() {
    for body in [
        "return [e[c]].concat([]);",
        "return [e[c]].slice(0);",
        "return [e[c]].splice(0);",
        "return [e[c]].map(function (x) { return x; });",
        "return [e[c]].filter(function () { return true; });",
    ] {
        let found = consulted(body, 0);
        for marker in ["constructor", "@@species"] {
            assert!(
                has(&found, IntrinsicProto::Array, &key(marker)),
                "{body} {marker} {found:?}"
            );
        }
    }
}
