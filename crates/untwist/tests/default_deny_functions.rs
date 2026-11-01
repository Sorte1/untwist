mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{decoded, q_refused};
use refused_rows::check;

const DECODES_TAG_BOUND_GENERATOR: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (Object.prototype.toString.call(function*(){}.bind(null))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn decodes_tag_bound_generator() {
    check(&decoded(DECODES_TAG_BOUND_GENERATOR, "s0|[object GeneratorFunction]\ns1|[object GeneratorFunction]\ns2|[object GeneratorFunction]\n"));
}

const REFUSES_CONSTRUCTOR_PLACEHOLDER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String([1].constructor === undefined) + typeof ({}).constructor)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_constructor_placeholder() {
    check(&q_refused(
        REFUSES_CONSTRUCTOR_PLACEHOLDER,
        "s0|falsefunction\ns1|falsefunction\ns2|falsefunction\n",
    ));
}

const REFUSES_ASYNC_FUNCTION_CONSTRUCTOR: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String((async function(){}).constructor === (function(){}).constructor))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_async_function_constructor() {
    check(&q_refused(
        REFUSES_ASYNC_FUNCTION_CONSTRUCTOR,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const REFUSES_OBJECT_CREATE_MATH_TAG: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((String(Object.create(Math)))); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_object_create_math_tag() {
    check(&q_refused(
        REFUSES_OBJECT_CREATE_MATH_TAG,
        "s0|[object Math]\ns1|[object Math]\ns2|[object Math]\n",
    ));
}

const REFUSES_ANNEX_B_BLOCK_FUNCTION_VAR: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var x = 1; { function x(){} } return typeof x; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_annex_b_block_function_var() {
    check(&q_refused(
        REFUSES_ANNEX_B_BLOCK_FUNCTION_VAR,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const REFUSES_ANNEX_B_IF_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ if (true) function g(){ return 1; } return typeof g; })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_annex_b_if_function() {
    check(&q_refused(
        REFUSES_ANNEX_B_IF_FUNCTION,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const REFUSES_SWITCH_CASE_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ switch (1) { case 1: function g(){} } return typeof g; })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_switch_case_function() {
    check(&q_refused(
        REFUSES_SWITCH_CASE_FUNCTION,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const REFUSES_FOR_OF_HEAD_TYPEOF_TDZ: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var x = 'o'; try { for (const x of (typeof x, [1])) {} return 'no'; } catch (z) { return 'tdz'; } })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_for_of_head_typeof_tdz() {
    check(&q_refused(
        REFUSES_FOR_OF_HEAD_TYPEOF_TDZ,
        "s0|tdz\ns1|tdz\ns2|tdz\n",
    ));
}
