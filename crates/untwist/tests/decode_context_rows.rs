mod refused_rows;

use refused_rows::{check, Row};

fn decoded(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(\"s0|",
    }
}

fn q_refused(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: "console.log(qb(0))",
    }
}

const A_HOLE_IN_THE_STRING_ARRAY_LITERAL_IS_NOT_A_STRING_ARRAY: &str = r#"function qa(){ var h=['s0',,'s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (i in e); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_hole_in_the_string_array_literal_is_not_a_string_array() {
    check(&q_refused(
        A_HOLE_IN_THE_STRING_ARRAY_LITERAL_IS_NOT_A_STRING_ARRAY,
        "s0|true\nundefined|false\ns2|true\n",
    ));
}

const A_PARAMETER_NAMED_UNDEFINED_IN_THE_STRING_ARRAY_IS_REFUSED: &str = r#"(function(undefined){
function qa(){ var h=['s0',undefined,'s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|'; }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));
})('X');"#;

#[test]
fn a_parameter_named_undefined_in_the_string_array_is_refused() {
    check(&q_refused(
        A_PARAMETER_NAMED_UNDEFINED_IN_THE_STRING_ARRAY_IS_REFUSED,
        "s0|\nX|\ns2|\n",
    ));
}

const A_VAR_NAMED_UNDEFINED_IN_THE_STRING_ARRAY_IS_REFUSED: &str = r#"var undefined = 'U';
function qa(){ var h=['s0',undefined,'s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|'; }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_var_named_undefined_in_the_string_array_is_refused() {
    check(&q_refused(
        A_VAR_NAMED_UNDEFINED_IN_THE_STRING_ARRAY_IS_REFUSED,
        "s0|\nU|\ns2|\n",
    ));
}

const A_LET_NAMED_UNDEFINED_AROUND_THE_STRING_ARRAY_IS_REFUSED: &str = r#"{ let undefined = 'L';
function qa(){ var h=['s0',undefined,'s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|'; }
}
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_let_named_undefined_around_the_string_array_is_refused() {
    check(&q_refused(
        A_LET_NAMED_UNDEFINED_AROUND_THE_STRING_ARRAY_IS_REFUSED,
        "s0|\nL|\ns2|\n",
    ));
}

const A_STRICT_NAMED_STRING_ARRAY_FUNCTION_THAT_THROWS_IS_REFUSED: &str = r#"'use strict';
var qa = function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); };
function qb(i){ var e; try { e = qa(); } catch (z) { return 'caught' + i; } return e[i]; }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_strict_named_string_array_function_that_throws_is_refused() {
    check(&q_refused(
        A_STRICT_NAMED_STRING_ARRAY_FUNCTION_THAT_THROWS_IS_REFUSED,
        "caught0\ncaught1\ncaught2\n",
    ));
}

const A_DECODER_IN_STRICT_CODE_IS_STRICT_WHEN_THE_ARRAY_IS_SLOPPY: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
(function(){ 'use strict';
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return (function f(){ f = 1; return typeof f; })(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));
})();"#;

#[test]
fn a_decoder_in_strict_code_is_strict_when_the_array_is_sloppy() {
    check(&decoded(
        A_DECODER_IN_STRICT_CODE_IS_STRICT_WHEN_THE_ARRAY_IS_SLOPPY,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_STRICT_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED: &str = r#"'use strict';
function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { undeclaredZ = 1; return 'none:' + typeof undeclaredZ; } catch (z) { return 'caught:' + z.name; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_strict_undeclared_assignment_in_a_decoder_is_refused() {
    check(&q_refused(
        A_STRICT_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED,
        "s0|caught:ReferenceError\ns1|caught:ReferenceError\ns2|caught:ReferenceError\n",
    ));
}

const A_STRICT_FUNCTION_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ 'use strict'; var e=qa(); return e[i] + '|' + ((function(){ try { undeclaredZ = 1; return 'none'; } catch (z) { return 'caught:' + z.name; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_strict_function_undeclared_assignment_in_a_decoder_is_refused() {
    check(&q_refused(
        A_STRICT_FUNCTION_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED,
        "s0|caught:ReferenceError\ns1|caught:ReferenceError\ns2|caught:ReferenceError\n",
    ));
}

const A_SLOPPY_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ undeclaredY = 5; return undeclaredY + 1; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_sloppy_undeclared_assignment_in_a_decoder_is_refused() {
    check(&q_refused(
        A_SLOPPY_UNDECLARED_ASSIGNMENT_IN_A_DECODER_IS_REFUSED,
        "s0|6\ns1|6\ns2|6\n",
    ));
}

const A_FOR_IN_THAT_DELETES_A_LATER_KEY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = {a: 1, b: 2, c: 3}, s = ''; for (var k in o) { s += k; if (k === 'a') delete o.b; } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_for_in_that_deletes_a_later_key_is_refused() {
    check(&q_refused(
        A_FOR_IN_THAT_DELETES_A_LATER_KEY_IS_REFUSED,
        "s0|ac\ns1|ac\ns2|ac\n",
    ));
}

const A_FOR_IN_THAT_SHRINKS_ITS_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3], s = ''; for (var k in a) { s += k; a.length = 1; } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_for_in_that_shrinks_its_array_is_refused() {
    check(&q_refused(
        A_FOR_IN_THAT_SHRINKS_ITS_ARRAY_IS_REFUSED,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const A_FOR_IN_SKIPS_THE_INDEX_A_POP_REMOVES: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3], s = ''; for (var k in a) { s += k; a.pop(); } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_for_in_skips_the_index_a_pop_removes() {
    check(&decoded(
        A_FOR_IN_SKIPS_THE_INDEX_A_POP_REMOVES,
        "s0|01\ns1|01\ns2|01\n",
    ));
}

const A_FOR_IN_SKIPS_THE_INDICES_A_SPLICE_REMOVES: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = ['x', 'y', 'z'], s = ''; for (var k in a) { s += k; if (k === '0') a.splice(0, 2); } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_for_in_skips_the_indices_a_splice_removes() {
    check(&decoded(
        A_FOR_IN_SKIPS_THE_INDICES_A_SPLICE_REMOVES,
        "s0|0\ns1|0\ns2|0\n",
    ));
}
