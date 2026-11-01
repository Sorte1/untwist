mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, q_refused};
use refused_rows::check;

const A_LET_LOOP_KEEPS_AN_UPDATE_THAT_CALLS_A_TEST_CLOSURE: &str = r#"var out = [], f, c = 0;
for (let i = 0; (f = function () { i++; }), c++ < 6 && i < 3; f(), out.push(i));
console.log(out.join(), c);"#;

#[test]
fn a_let_loop_keeps_an_update_that_calls_a_test_closure() {
    check(&agrees(
        A_LET_LOOP_KEEPS_AN_UPDATE_THAT_CALLS_A_TEST_CLOSURE,
        "0,0,0,0,0,0 7\n",
    ));
}

const A_LET_LOOP_KEEPS_AN_UPDATE_BESIDE_A_CLOSURE_IN_THE_TEST: &str = r#"var fs = [], out = [], c = 0;
for (let i = 0; fs.push(function () { return i++; }), c++ < 5 && i < 3; out.push(fs[fs.length - 1]()), out.push(i));
console.log(out.join(), fs.length);"#;

#[test]
fn a_let_loop_keeps_an_update_beside_a_closure_in_the_test() {
    check(&agrees(
        A_LET_LOOP_KEEPS_AN_UPDATE_BESIDE_A_CLOSURE_IN_THE_TEST,
        "0,0,0,0,0,0,0,0,0,0 6\n",
    ));
}

const A_NESTED_LET_LOOP_KEEPS_ITS_UPDATE_BESIDE_A_TEST_CLOSURE: &str = r#"var out = [];
for (let j = 0; j < 2; j++) {
  var f; var c = 0;
  for (let i = 0; (f = () => { i += 2; }), c++ < 5 && i < 4; f(), out.push(j + ':' + i));
}
console.log(out.join());"#;

#[test]
fn a_nested_let_loop_keeps_its_update_beside_a_test_closure() {
    check(&agrees(
        A_NESTED_LET_LOOP_KEEPS_ITS_UPDATE_BESIDE_A_TEST_CLOSURE,
        "0:0,0:0,0:0,0:0,0:0,1:0,1:0,1:0,1:0,1:0\n",
    ));
}

const A_LABELLED_LET_LOOP_KEEPS_ITS_UPDATE_BESIDE_A_TEST_CLOSURE: &str = r#"var out = [], f, c = 0;
outer: for (let i = 0; (f = function () { return ++i; }), c++ < 6 && i < 3; out.push(f()), out.push(i));
console.log(out.join());"#;

#[test]
fn a_labelled_let_loop_keeps_its_update_beside_a_test_closure() {
    check(&agrees(
        A_LABELLED_LET_LOOP_KEEPS_ITS_UPDATE_BESIDE_A_TEST_CLOSURE,
        "1,0,1,0,1,0,1,0,1,0,1,0\n",
    ));
}

const A_FOR_IN_LET_HEAD_KEEPS_ITS_CLOSURE_IN_THE_HEAD: &str = r#"var g;
for (let k in (g = () => typeof k, { a: 1 }));
try { console.log(g()); } catch (e) { console.log('T'); }"#;

#[test]
fn a_for_in_let_head_keeps_its_closure_in_the_head() {
    check(&agrees(
        A_FOR_IN_LET_HEAD_KEEPS_ITS_CLOSURE_IN_THE_HEAD,
        "T\n",
    ));
}

const A_FOR_OF_LET_HEAD_KEEPS_ITS_CLOSURE_IN_THE_HEAD: &str = r#"var v = 'outer', g;
for (let v of (g = () => v, ['in'])) {}
try { console.log(g()); } catch (e) { console.log('T'); }"#;

#[test]
fn a_for_of_let_head_keeps_its_closure_in_the_head() {
    check(&agrees(
        A_FOR_OF_LET_HEAD_KEEPS_ITS_CLOSURE_IN_THE_HEAD,
        "T\n",
    ));
}

const A_FOR_LET_INIT_KEEPS_ITS_CLOSURE_IN_THE_INIT: &str = r#"var g, out = [];
for (let i = (g = () => i, 0); i < 2; i++) out.push(i);
try { console.log(out.join(), g()); } catch (e) { console.log(out.join(), 'T'); }"#;

#[test]
fn a_for_let_init_keeps_its_closure_in_the_init() {
    check(&agrees(
        A_FOR_LET_INIT_KEEPS_ITS_CLOSURE_IN_THE_INIT,
        "0,1 0\n",
    ));
}

const A_FOR_LET_INIT_CLOSURE_DOES_NOT_REACH_THE_OUTER_NAME: &str = r#"var i = 'outer', g;
for (let i = (g = () => i, 5); i < 6; i++);
console.log(g());"#;

#[test]
fn a_for_let_init_closure_does_not_reach_the_outer_name() {
    check(&agrees(
        A_FOR_LET_INIT_CLOSURE_DOES_NOT_REACH_THE_OUTER_NAME,
        "5\n",
    ));
}

const A_DECODER_WITH_A_FOR_IN_LET_HEAD_CLOSURE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var g; for (let k in (g = () => typeof k, { a: 1 })) {} try { return g(); } catch (z) { return 'T'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn a_decoder_with_a_for_in_let_head_closure_is_refused() {
    check(&q_refused(
        A_DECODER_WITH_A_FOR_IN_LET_HEAD_CLOSURE_IS_REFUSED,
        "s0|T\ns1|T\ns2|T\n",
    ));
}
