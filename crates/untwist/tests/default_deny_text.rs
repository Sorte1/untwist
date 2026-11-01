mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, q_refused};
use refused_rows::{check, check_with};

const TOLERATE_NEWER_NODE_UNICODE: bool = true;

const REFUSES_FINAL_SIGMA_BEFORE_U0295: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('AΣʕB'.toLowerCase().charCodeAt(1)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_final_sigma_before_u0295() {
    check_with(
        &q_refused(REFUSES_FINAL_SIGMA_BEFORE_U0295, "s0|963\ns1|963\ns2|963\n"),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_FINAL_SIGMA_BEFORE_UNICODE_17_MARK: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('AΣ᫏B'.toLowerCase().charCodeAt(1)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_final_sigma_before_unicode_17_mark() {
    check_with(
        &q_refused(
            REFUSES_FINAL_SIGMA_BEFORE_UNICODE_17_MARK,
            "s0|962\ns1|962\ns2|962\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_FINAL_SIGMA_AFTER_UNICODE_17_MARK: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('A᫏Σ'.toLowerCase().charCodeAt(2)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_final_sigma_after_unicode_17_mark() {
    check_with(
        &q_refused(
            REFUSES_FINAL_SIGMA_AFTER_UNICODE_17_MARK,
            "s0|963\ns1|963\ns2|963\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_FINAL_SIGMA_ASTRAL_UNICODE_17_MARK: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ('AΣ\u{11B60}B'.toLowerCase().charCodeAt(1)); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_final_sigma_astral_unicode_17_mark() {
    check_with(
        &q_refused(
            REFUSES_FINAL_SIGMA_ASTRAL_UNICODE_17_MARK,
            "s0|962\ns1|962\ns2|962\n",
        ),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}

const REFUSES_NON_ASCII_LOWER_CASE_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (('\u00c9COLE'.toLowerCase())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_non_ascii_lower_case_refused() {
    check(&q_refused(
        REFUSES_NON_ASCII_LOWER_CASE_REFUSED,
        "s0|école\ns1|école\ns2|école\n",
    ));
}

const REFUSES_ATOB_INVALID_CAUGHT: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ try { atob('%'); return 'ok'; } catch (z) { return 'caught'; } })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_atob_invalid_caught() {
    check(&q_refused(
        REFUSES_ATOB_INVALID_CAUGHT,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const KEEPS_FOLD_FINAL_SIGMA_UNICODE_17_MARK: &str =
    r#"console.log('A᫏Σ'.toLowerCase() === 'a᫏σ', 'AΣ᫏B'.toLowerCase().charCodeAt(1));"#;

#[test]
fn keeps_fold_final_sigma_unicode_17_mark() {
    check_with(
        &agrees(KEEPS_FOLD_FINAL_SIGMA_UNICODE_17_MARK, "true 962\n"),
        TOLERATE_NEWER_NODE_UNICODE,
    );
}
