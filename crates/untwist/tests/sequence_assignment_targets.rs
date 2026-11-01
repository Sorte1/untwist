mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded};
use refused_rows::check;

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {a: 1}; o.a += (o.a = 5, 1); return o.a; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_IDENT_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = 1; a += (a = 5, 1); return a; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_ident_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_IDENT_TARGET_WRITTEN_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_COMPUTED_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {a: 1}; o['a'] += (o['a'] = 5, 1); return o.a; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_computed_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_COMPUTED_TARGET_WRITTEN_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_ARRAY_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1]; a[0] += (a[0] = 5, 1); return a[0]; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_array_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_ARRAY_TARGET_WRITTEN_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_CONCAT_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {s: 'x'}; o.s += (o.s = 'y', 'z'); return o.s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_concat_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_CONCAT_TARGET_WRITTEN_BY_RHS,
        "s0|xz\ns1|xz\ns2|xz\n",
    ));
}

const KEEPS_COMPOUND_ARGUMENTS_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ function f(a){ arguments[0] += (arguments[0] = 10, 1); return arguments[0]; } return f(1); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_arguments_target_written_by_rhs() {
    check(&agrees(
        KEEPS_COMPOUND_ARGUMENTS_TARGET_WRITTEN_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_CACHE_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var c = {cnt: 0}; function d(i){ c.cnt += (c.cnt = i * 10, 1); return c.cnt; } return d(1) + ',' + d(2); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_cache_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_CACHE_WRITTEN_BY_RHS,
        "s0|1,2\ns1|1,2\ns2|1,2\n",
    ));
}

const KEEPS_COMPOUND_SHIFT_TARGET_WRITTEN_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {v: 1}; o.v <<= (o.v = 3, 2); return o.v; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_shift_target_written_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_SHIFT_TARGET_WRITTEN_BY_RHS,
        "s0|4\ns1|4\ns2|4\n",
    ));
}

const KEEPS_MEMBER_ASSIGN_BASE_REASSIGNED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {}; var p = o; o.k = (o = {}, 5); return JSON.stringify([p, o]); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_member_assign_base_reassigned_by_rhs() {
    check(&agrees(
        KEEPS_MEMBER_ASSIGN_BASE_REASSIGNED_BY_RHS,
        "s0|[{\"k\":5},{}]\ns1|[{\"k\":5},{}]\ns2|[{\"k\":5},{}]\n",
    ));
}

const KEEPS_COMPOUND_IDENT_TARGET_WRITTEN_BY_CALL: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function () { var s = 'a'; function f() { s += 'b'; } s += (f(), 'c'); return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_ident_target_written_by_call() {
    check(&decoded(
        KEEPS_COMPOUND_IDENT_TARGET_WRITTEN_BY_CALL,
        "s0|ac\ns1|ac\ns2|ac\n",
    ));
}

const KEEPS_MEMBER_ASSIGN_BASE_REASSIGNED_BY_CALL: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function () { var o = {}, keep = o; function swap() { o = {}; } o.k = (swap(), 5); return JSON.stringify([keep, o]); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_member_assign_base_reassigned_by_call() {
    check(&agrees(
        KEEPS_MEMBER_ASSIGN_BASE_REASSIGNED_BY_CALL,
        "s0|[{\"k\":5},{}]\ns1|[{\"k\":5},{}]\ns2|[{\"k\":5},{}]\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CALL: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function () { var st = {n: 1}; function bump() { st.n = 10; } st.n += (bump(), 1); return st.n; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_call() {
    check(&decoded(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CALL,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_ASSIGN_ORDER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {a: 1}; o.a += (o.a = 5, 1); var i = 0, arr = []; arr[i++] = i; var x = 1; x += x++ + ++x; return [o.a, arr.join(), x].join(); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_assign_order() {
    check(&decoded(
        KEEPS_COMPOUND_ASSIGN_ORDER,
        "s0|2,1,5\ns1|2,1,5\ns2|2,1,5\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_GETTER: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1, get g(){ o.k = 10; return 0; }}; o.k += (o.g, 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_getter() {
    check(&agrees(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_GETTER,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPUTED_ASSIGN_KEY_UPDATED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [0,0,0], i = 0; a[i] = (i++, 5); return a.join('') + ',' + i; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_computed_assign_key_updated_by_rhs() {
    check(&decoded(
        KEEPS_COMPUTED_ASSIGN_KEY_UPDATED_BY_RHS,
        "s0|500,1\ns1|500,1\ns2|500,1\n",
    ));
}

const KEEPS_COMPUTED_COMPOUND_KEY_ASSIGNED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1,1,1], i = 0; a[i] += (i = 2, 1); return a.join(''); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_computed_compound_key_assigned_by_rhs() {
    check(&decoded(
        KEEPS_COMPUTED_COMPOUND_KEY_ASSIGNED_BY_RHS,
        "s0|211\ns1|211\ns2|211\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CLASS_STATIC_BLOCK: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; o.k += (class { static { o.k = 9; } }, 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_class_static_block() {
    check(&agrees(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CLASS_STATIC_BLOCK,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_ACROSS_YIELD: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; function* g(){ o.k += (yield 0, 1); } var it = g(); it.next(); o.k = 5; it.next(); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_across_yield() {
    check(&agrees(
        KEEPS_COMPOUND_MEMBER_TARGET_ACROSS_YIELD,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_IDENT_TARGET_UPDATED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var x = 1; x += (x++, 1); return x; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_ident_target_updated_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_IDENT_TARGET_UPDATED_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_IDENT_TARGET_DESTRUCTURED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var x = 1; x += ([x] = [5], 1); return x; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_ident_target_destructured_by_rhs() {
    check(&agrees(
        KEEPS_COMPOUND_IDENT_TARGET_DESTRUCTURED_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_DELETED_BY_RHS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; o.k += (delete o.k, 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_deleted_by_rhs() {
    check(&decoded(
        KEEPS_COMPOUND_MEMBER_TARGET_DELETED_BY_RHS,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_TAG: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; function t(){ o.k = 4; return 0; } o.k += (t`x`, 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_tag() {
    check(&agrees(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_TAG,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CONSTRUCTOR: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; function F(){ o.k = 6; } o.k += (new F(), 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn keeps_compound_member_target_written_by_constructor() {
    check(&decoded(
        KEEPS_COMPOUND_MEMBER_TARGET_WRITTEN_BY_CONSTRUCTOR,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const HOISTS_COMPOUND_IDENT_RHS_WRITING_ANOTHER_NAME: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var x = 1, y = 0; x += (y = 2, 1); return x + ',' + y; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn hoists_compound_ident_rhs_writing_another_name() {
    check(&decoded(
        HOISTS_COMPOUND_IDENT_RHS_WRITING_ANOTHER_NAME,
        "s0|2,2\ns1|2,2\ns2|2,2\n",
    ));
}

const HOISTS_MEMBER_ASSIGN_RHS_WITH_UNRELATED_CALL: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {}, log = []; o.k = (log.push(1), 5); return o.k + ',' + log.length; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn hoists_member_assign_rhs_with_unrelated_call() {
    check(&decoded(
        HOISTS_MEMBER_ASSIGN_RHS_WITH_UNRELATED_CALL,
        "s0|5,1\ns1|5,1\ns2|5,1\n",
    ));
}

const HOISTS_IDENT_ASSIGN_RHS_WITH_CALL_WRITING_TARGET: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var x = 1; function f(){ x = 7; } x = (f(), 3); return x; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn hoists_ident_assign_rhs_with_call_writing_target() {
    check(&decoded(
        HOISTS_IDENT_ASSIGN_RHS_WITH_CALL_WRITING_TARGET,
        "s0|3\ns1|3\ns2|3\n",
    ));
}

const HOISTS_COMPOUND_MEMBER_RHS_WITH_UNCALLED_FUNCTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {k: 1}; o.k += (function(){ o.k = 9; }, 1); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn hoists_compound_member_rhs_with_uncalled_function() {
    check(&decoded(
        HOISTS_COMPOUND_MEMBER_RHS_WITH_UNCALLED_FUNCTION,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const HOISTS_MEMBER_ASSIGN_RHS_OF_LITERALS: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {}; o.k = (0, 5); return o.k; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn hoists_member_assign_rhs_of_literals() {
    check(&decoded(
        HOISTS_MEMBER_ASSIGN_RHS_OF_LITERALS,
        "s0|5\ns1|5\ns2|5\n",
    ));
}
