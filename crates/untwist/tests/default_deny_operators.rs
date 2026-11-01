mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, q_refused};
use refused_rows::check;

const REFUSES_IN_PRIMITIVE_RIGHT_SKIPS_KEY_COERCION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var c = 0; try { ({ toString: function(){ c++; return 'a'; } }) in 'str'; } catch (z) {} return c; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_in_primitive_right_skips_key_coercion() {
    check(&q_refused(
        REFUSES_IN_PRIMITIVE_RIGHT_SKIPS_KEY_COERCION,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const REFUSES_NULL_MEMBER_KEY_COERCION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var c = 0; try { null[{ toString: function(){ c++; return 'a'; } }]; } catch (z) {} return c; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_null_member_key_coercion() {
    check(&q_refused(
        REFUSES_NULL_MEMBER_KEY_COERCION,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const REFUSES_MEMBER_ASSIGN_KEY_BEFORE_VALUE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', o = {}; o[{ toString: function(){ log += 'k'; return 'p'; } }] = (log += 'v', 1); return log; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_member_assign_key_before_value() {
    check(&q_refused(
        REFUSES_MEMBER_ASSIGN_KEY_BEFORE_VALUE,
        "s0|vk\ns1|vk\ns2|vk\n",
    ));
}

const REFUSES_COMPOUND_MEMBER_ASSIGN_ORDER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', o = { p: 1 }; o[{ toString: function(){ log += 'k'; return 'p'; } }] += (log += 'v', 1); return log + o.p; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_compound_member_assign_order() {
    check(&q_refused(
        REFUSES_COMPOUND_MEMBER_ASSIGN_ORDER,
        "s0|kvk2\ns1|kvk2\ns2|kvk2\n",
    ));
}

const REFUSES_UPDATE_MEMBER_KEY_COERCION_COUNT: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', o = { p: 1 }; o[{ toString: function(){ log += 'k'; return 'p'; } }]++; return log + o.p; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_update_member_key_coercion_count() {
    check(&q_refused(
        REFUSES_UPDATE_MEMBER_KEY_COERCION_COUNT,
        "s0|kk2\ns1|kk2\ns2|kk2\n",
    ));
}

const REFUSES_ARRAY_MEMBER_ASSIGN_KEY_ORDER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', a = [0]; a[{ toString: function(){ log += 'k'; return '0'; } }] = (log += 'v', 5); return log + a[0]; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_array_member_assign_key_order() {
    check(&q_refused(
        REFUSES_ARRAY_MEMBER_ASSIGN_KEY_ORDER,
        "s0|vk5\ns1|vk5\ns2|vk5\n",
    ));
}

const REFUSES_LOGICAL_ASSIGN_MEMBER_KEY_COUNT: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', o = { p: 0 }; o[{ toString: function(){ log += 'k'; return 'p'; } }] ||= (log += 'v', 2); return log + o.p; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_logical_assign_member_key_count() {
    check(&q_refused(
        REFUSES_LOGICAL_ASSIGN_MEMBER_KEY_COUNT,
        "s0|kvk2\ns1|kvk2\ns2|kvk2\n",
    ));
}

const REFUSES_EXPONENT_ASSIGN_MEMBER_ORDER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var log = '', o = { p: 2 }; o[{ toString: function(){ log += 'k'; return 'p'; } }] **= (log += 'v', 3); return log + o.p; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_exponent_assign_member_order() {
    check(&q_refused(
        REFUSES_EXPONENT_ASSIGN_MEMBER_ORDER,
        "s0|kvk8\ns1|kvk8\ns2|kvk8\n",
    ));
}

const REFUSES_IN_PRIMITIVE_RIGHT_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ try { return 'a' in 'abc'; } catch (z) { return 'caught'; } })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_in_primitive_right_refused() {
    check(&q_refused(
        REFUSES_IN_PRIMITIVE_RIGHT_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const REFUSES_OBJECT_LITERAL_COMPUTED_OBJECT_KEY_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var log = ''; var o = { [{ toString: function(){ log += 'k'; return 'p'; } }]: (log += 'v', 1) }; return log + o.p; })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_object_literal_computed_object_key_refused() {
    check(&q_refused(
        REFUSES_OBJECT_LITERAL_COMPUTED_OBJECT_KEY_REFUSED,
        "s0|kv1\ns1|kv1\ns2|kv1\n",
    ));
}

const KEEPS_FOLD_MEMBER_ASSIGN_ORDER: &str = r#"var log = '', o = {};
o[{ toString: function () { log += 'k'; return 'p'; } }] = (log += 'v', 1);
console.log(log);"#;

#[test]
fn keeps_fold_member_assign_order() {
    check(&agrees(KEEPS_FOLD_MEMBER_ASSIGN_ORDER, "vk\n"));
}
