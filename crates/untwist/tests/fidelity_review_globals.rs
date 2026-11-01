mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{decoded, q_refused};
use refused_rows::check;

const TYPEOF_SYMBOL_IN_A_POLYFILL_CHECK_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((typeof Symbol === 'function' && typeof Symbol.iterator === 'symbol') ? 'native' : 'poly'); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_symbol_in_a_polyfill_check_is_refused() {
    check(&q_refused(
        TYPEOF_SYMBOL_IN_A_POLYFILL_CHECK_IS_REFUSED,
        "s0|native\ns1|native\ns2|native\n",
    ));
}

const TYPEOF_AN_UNBOUND_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (typeof Array); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_an_unbound_array_is_refused() {
    check(&q_refused(
        TYPEOF_AN_UNBOUND_ARRAY_IS_REFUSED,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const TYPEOF_UNBOUND_ERROR_CONSTRUCTORS_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (typeof Error + ',' + typeof TypeError); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_unbound_error_constructors_is_refused() {
    check(&q_refused(
        TYPEOF_UNBOUND_ERROR_CONSTRUCTORS_IS_REFUSED,
        "s0|function,function\ns1|function,function\ns2|function,function\n",
    ));
}

const TYPEOF_ARRAY_IN_A_GUARD_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (typeof Array === 'undefined' ? 'old' : 'new'); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_array_in_a_guard_is_refused() {
    check(&q_refused(
        TYPEOF_ARRAY_IN_A_GUARD_IS_REFUSED,
        "s0|new\ns1|new\ns2|new\n",
    ));
}

const TYPEOF_SYMBOL_UNBOUND_IN_AN_IIFE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ return typeof Symbol; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_symbol_unbound_in_an_iife_is_refused() {
    check(&q_refused(
        TYPEOF_SYMBOL_UNBOUND_IN_AN_IIFE_IS_REFUSED,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const TYPEOF_URI_ERROR_UNBOUND_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (typeof URIError + typeof EvalError + typeof RangeError); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn typeof_uri_error_unbound_is_refused() {
    check(&q_refused(
        TYPEOF_URI_ERROR_UNBOUND_IS_REFUSED,
        "s0|functionfunctionfunction\ns1|functionfunctionfunction\ns2|functionfunctionfunction\n",
    ));
}

const CALLING_OBJECT_ON_A_PRIMITIVE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return typeof Object(1); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn calling_object_on_a_primitive_is_refused() {
    check(&q_refused(
        CALLING_OBJECT_ON_A_PRIMITIVE_IS_REFUSED,
        "s0|object\ns1|object\ns2|object\n",
    ));
}

const STRING_TO_STRING_ON_A_BOXED_STRING_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return String.prototype.toString.call(Object('x')); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn string_to_string_on_a_boxed_string_is_refused() {
    check(&q_refused(
        STRING_TO_STRING_ON_A_BOXED_STRING_IS_REFUSED,
        "s0|x\ns1|x\ns2|x\n",
    ));
}

const CALLING_OBJECT_WITHOUT_ARGUMENTS_MAKES_AN_OBJECT: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return Object.keys(Object()).length; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn calling_object_without_arguments_makes_an_object() {
    check(&decoded(
        CALLING_OBJECT_WITHOUT_ARGUMENTS_MAKES_AN_OBJECT,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const OBJECT_CALLED_AS_A_FUNCTION_RETURNS_ITS_OBJECT_OR_A_NEW_ONE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {a: 1}; try { return [Object(o) === o, new Object(o) === o, typeof Object(null), typeof new Object(), Object(undefined) !== Object(undefined)].join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn object_called_as_a_function_returns_its_object_or_a_new_one() {
    check(&decoded(OBJECT_CALLED_AS_A_FUNCTION_RETURNS_ITS_OBJECT_OR_A_NEW_ONE, "s0|true,true,object,object,true\ns1|true,true,object,object,true\ns2|true,true,object,object,true\n"));
}
