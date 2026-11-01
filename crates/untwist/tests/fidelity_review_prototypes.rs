mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::q_refused;
use refused_rows::check;

const CREATE_FROM_A_NAMED_FUNCTION_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(function named(){}).name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_named_function_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_NAMED_FUNCTION_IS_REFUSED,
        "s0|named\ns1|named\ns2|named\n",
    ));
}

const CREATE_FROM_A_FUNCTION_READING_CALL_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return typeof Object.create(function(){}).call; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_function_reading_call_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_FUNCTION_READING_CALL_IS_REFUSED,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const CREATE_FROM_A_REGEXP_READING_SOURCE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(/a/).source; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_regexp_reading_source_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_REGEXP_READING_SOURCE_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const CREATE_FROM_A_GENERATOR_TAG_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.prototype.toString.call(Object.create(function*(){})); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_generator_tag_is_refused() {
    check(&q_refused(CREATE_FROM_A_GENERATOR_TAG_IS_REFUSED, "s0|[object GeneratorFunction]\ns1|[object GeneratorFunction]\ns2|[object GeneratorFunction]\n"));
}

const CREATE_FROM_A_FUNCTION_READING_LENGTH_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(function(a, b){}).length; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_function_reading_length_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_FUNCTION_READING_LENGTH_IS_REFUSED,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const CREATE_FROM_A_FUNCTION_WITH_IN_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return ('name' in Object.create(function f(){})) + ',' + ('length' in Object.create(function f(){})); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_function_with_in_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_FUNCTION_WITH_IN_IS_REFUSED,
        "s0|true,true\ns1|true,true\ns2|true,true\n",
    ));
}

const CREATE_FROM_A_REGEXP_READING_GLOBAL_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(/a/g).global; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_regexp_reading_global_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_REGEXP_READING_GLOBAL_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const CREATE_TWO_LEVELS_FROM_A_FUNCTION_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(Object.create(function named(){})).name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_two_levels_from_a_function_is_refused() {
    check(&q_refused(
        CREATE_TWO_LEVELS_FROM_A_FUNCTION_IS_REFUSED,
        "s0|named\ns1|named\ns2|named\n",
    ));
}

const CREATE_FROM_A_REGEXP_WITH_IN_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return 'lastIndex' in Object.create(/a/); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_regexp_with_in_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_REGEXP_WITH_IN_IS_REFUSED,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const CREATE_FROM_A_REGEXP_READING_LAST_INDEX_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.create(/a/g).lastIndex; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_regexp_reading_last_index_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_REGEXP_READING_LAST_INDEX_IS_REFUSED,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const CREATE_FROM_A_FUNCTION_CONVERTED_TO_STRING_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return String(Object.create(function f(){})); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_a_function_converted_to_string_is_refused() {
    check(&q_refused(
        CREATE_FROM_A_FUNCTION_CONVERTED_TO_STRING_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const CREATE_FROM_AN_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var o = Object.create([1, 2]); return [o.length, typeof o.join, Object.prototype.toString.call(o)].join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn create_from_an_array_is_refused() {
    check(&q_refused(CREATE_FROM_AN_ARRAY_IS_REFUSED, "s0|2,function,[object Object]\ns1|2,function,[object Object]\ns2|2,function,[object Object]\n"));
}

const NEW_WITH_A_FUNCTION_PROTOTYPE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { function F(){} F.prototype = function named(){}; var o = new F(); return [typeof o.call, o.name, 'length' in o].join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn new_with_a_function_prototype_is_refused() {
    check(&q_refused(
        NEW_WITH_A_FUNCTION_PROTOTYPE_IS_REFUSED,
        "s0|function,named,true\ns1|function,named,true\ns2|function,named,true\n",
    ));
}

const A_FUNCTION_PROTO_KEY_IN_A_LITERAL_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return ({__proto__: function named(){}}).name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_function_proto_key_in_a_literal_is_refused() {
    check(&q_refused(
        A_FUNCTION_PROTO_KEY_IN_A_LITERAL_IS_REFUSED,
        "s0|named\ns1|named\ns2|named\n",
    ));
}

const SET_PROTOTYPE_OF_A_FUNCTION_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var o = {}; Object.setPrototypeOf(o, function named(){}); return o.name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn set_prototype_of_a_function_is_refused() {
    check(&q_refused(
        SET_PROTOTYPE_OF_A_FUNCTION_IS_REFUSED,
        "s0|named\ns1|named\ns2|named\n",
    ));
}

const ASSIGNING_A_REGEXP_PROTO_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var o = {}; o.__proto__ = /a/g; return o.global; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn assigning_a_regexp_proto_is_refused() {
    check(&q_refused(
        ASSIGNING_A_REGEXP_PROTO_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}
