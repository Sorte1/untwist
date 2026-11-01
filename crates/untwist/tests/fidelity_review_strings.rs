mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused};
use refused_rows::{check, check_with};

const TOLERATE_NEWER_NODE_UNICODE: bool = true;

const STRING_INDEX_OF_CLAMPS_A_NEGATIVE_POSITION_TO_ZERO: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('abc'.indexOf('b', -1) + ',' + 'abc'.indexOf('a', -1.5) + ',' + 'abc'.indexOf('c', -Infinity)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn string_index_of_clamps_a_negative_position_to_zero() {
    check(&decoded(
        STRING_INDEX_OF_CLAMPS_A_NEGATIVE_POSITION_TO_ZERO,
        "s0|1,0,2\ns1|1,0,2\ns2|1,0,2\n",
    ));
}

const STRING_INDEX_OF_CLAMPS_A_NEGATIVE_ZERO_FRACTION: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('abc'.indexOf('a', -0.5) + ',' + 'abc'.indexOf('a', -0) + ',' + 'abc'.indexOf('b', '-1')); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn string_index_of_clamps_a_negative_zero_fraction() {
    check(&decoded(
        STRING_INDEX_OF_CLAMPS_A_NEGATIVE_ZERO_FRACTION,
        "s0|0,0,1\ns1|0,0,1\ns2|0,0,1\n",
    ));
}

const STRING_INCLUDES_CLAMPS_A_NEGATIVE_POSITION_TO_ZERO: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('abc'.includes('a', -1) + ',' + 'abc'.includes('b', -2)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn string_includes_clamps_a_negative_position_to_zero() {
    check(&decoded(
        STRING_INCLUDES_CLAMPS_A_NEGATIVE_POSITION_TO_ZERO,
        "s0|true,true\ns1|true,true\ns2|true,true\n",
    ));
}

const A_FOLDED_STRING_INDEX_OF_CLAMPS_A_NEGATIVE_POSITION: &str =
    r#"console.log('abc'.indexOf('b', -1), '0.1'.indexOf('0', -1.5));"#;

#[test]
fn a_folded_string_index_of_clamps_a_negative_position() {
    check(&agrees(
        A_FOLDED_STRING_INDEX_OF_CLAMPS_A_NEGATIVE_POSITION,
        "1 0\n",
    ));
}

const LOWER_CASING_A_UNICODE_17_LATIN_LETTER_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var s = ''; for (var c = 0xa7c0; c < 0xa7e0; c++) { var l = String.fromCharCode(c).toLowerCase(); if (l !== String.fromCharCode(c)) s += c.toString(16) + '>' + l.charCodeAt(0).toString(16) + ' '; } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn lower_casing_a_unicode_17_latin_letter_is_refused() {
    check_with(&q_refused(LOWER_CASING_A_UNICODE_17_LATIN_LETTER_IS_REFUSED, "s0|a7c0>a7c1 a7c2>a7c3 a7c4>a794 a7c5>282 a7c6>1d8e a7c7>a7c8 a7c9>a7ca a7cb>264 a7cc>a7cd a7d0>a7d1 a7d6>a7d7 a7d8>a7d9 a7da>a7db a7dc>19b \ns1|a7c0>a7c1 a7c2>a7c3 a7c4>a794 a7c5>282 a7c6>1d8e a7c7>a7c8 a7c9>a7ca a7cb>264 a7cc>a7cd a7d0>a7d1 a7d6>a7d7 a7d8>a7d9 a7da>a7db a7dc>19b \ns2|a7c0>a7c1 a7c2>a7c3 a7c4>a794 a7c5>282 a7c6>1d8e a7c7>a7c8 a7c9>a7ca a7cb>264 a7cc>a7cd a7d0>a7d1 a7d6>a7d7 a7d8>a7d9 a7da>a7db a7dc>19b \n"), TOLERATE_NEWER_NODE_UNICODE);
}

const UPPER_CASING_A_UNICODE_17_LATIN_LETTER_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var s = ''; for (var c = 0xa7c0; c < 0xa7e0; c++) { var l = String.fromCharCode(c).toUpperCase(); if (l !== String.fromCharCode(c)) s += c.toString(16) + '>' + l.charCodeAt(0).toString(16) + ' '; } return s; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn upper_casing_a_unicode_17_latin_letter_is_refused() {
    check_with(&q_refused(UPPER_CASING_A_UNICODE_17_LATIN_LETTER_IS_REFUSED, "s0|a7c1>a7c0 a7c3>a7c2 a7c8>a7c7 a7ca>a7c9 a7cd>a7cc a7d1>a7d0 a7d7>a7d6 a7d9>a7d8 a7db>a7da \ns1|a7c1>a7c0 a7c3>a7c2 a7c8>a7c7 a7ca>a7c9 a7cd>a7cc a7d1>a7d0 a7d7>a7d6 a7d9>a7d8 a7db>a7da \ns2|a7c1>a7c0 a7c3>a7c2 a7c8>a7c7 a7ca>a7c9 a7cd>a7cc a7d1>a7d0 a7d7>a7d6 a7d9>a7d8 a7db>a7da \n"), TOLERATE_NEWER_NODE_UNICODE);
}

const CASE_MAPPING_BERIA_ERFE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var n = 0, h = 0; for (var c = 0x16e40; c < 0x16f00; c++) { var ch = String.fromCharCode(0xd800 + ((c - 0x10000) >> 10), 0xdc00 + ((c - 0x10000) & 1023)); var l = ch.toLowerCase(); if (l !== ch) { n++; h = (h * 31 + l.charCodeAt(1)) % 1000003; } } return n + ':' + h; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn case_mapping_beria_erfe_is_refused() {
    check_with(
        &q_refused(
            CASE_MAPPING_BERIA_ERFE_IS_REFUSED,
            "s0|32:764613\ns1|32:764613\ns2|32:764613\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const CASE_MAPPING_ACROSS_THE_A640_BLOCK_WITH_NEW_LETTERS_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var h = 0; for (var c = 0xa640; c < 0xa800; c++) { var l = String.fromCharCode(c).toLowerCase() + String.fromCharCode(c).toUpperCase(); for (var j = 0; j < l.length; j++) h = (h * 31 + l.charCodeAt(j)) % 1000003; } return h; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn case_mapping_across_the_a640_block_with_new_letters_is_refused() {
    check_with(
        &q_refused(
            CASE_MAPPING_ACROSS_THE_A640_BLOCK_WITH_NEW_LETTERS_IS_REFUSED,
            "s0|830991\ns1|830991\ns2|830991\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const A_FOLDED_CASE_MAPPING_OF_A_UNICODE_17_LETTER_MATCHES_NODE: &str =
    r#"console.log(escape('꟎꟒'.toLowerCase()), escape('꟏'.toUpperCase()));"#;

#[test]
fn a_folded_case_mapping_of_a_unicode_17_letter_matches_node() {
    check_with(
        &agrees(
            A_FOLDED_CASE_MAPPING_OF_A_UNICODE_17_LETTER_MATCHES_NODE,
            "%uA7CE%uA7D2 %uA7CF\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}
