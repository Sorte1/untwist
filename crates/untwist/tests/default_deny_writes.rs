mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::q_refused;
use refused_rows::check;

const REFUSES_STRICT_FN_NAME_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
(function(){ 'use strict';
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var f = function g(){}; f.name = 'x'; return f.name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));
})();"#;

#[test]
fn refuses_strict_fn_name_write() {
    check(&q_refused(
        REFUSES_STRICT_FN_NAME_WRITE,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const REFUSES_SLOPPY_FN_NAME_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var f = function g(){}; f.name = 'x'; return f.name; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_sloppy_fn_name_write() {
    check(&q_refused(
        REFUSES_SLOPPY_FN_NAME_WRITE,
        "s0|g\ns1|g\ns2|g\n",
    ));
}

const REFUSES_STRICT_FN_LENGTH_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
(function(){ 'use strict';
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var f = function(a){}; f.length = 5; return f.length; } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));
})();"#;

#[test]
fn refuses_strict_fn_length_write() {
    check(&q_refused(
        REFUSES_STRICT_FN_LENGTH_WRITE,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const REFUSES_FN_LENGTH_SLOPPY_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var f = function(a, b){}; f.length = 9; return f.length; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_fn_length_sloppy_write() {
    check(&q_refused(
        REFUSES_FN_LENGTH_SLOPPY_WRITE,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const REFUSES_ARROW_NAME_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var f = () => 1; f.name = 'z'; return f.name; })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_arrow_name_write() {
    check(&q_refused(REFUSES_ARROW_NAME_WRITE, "s0|f\ns1|f\ns2|f\n"));
}

const REFUSES_REGEXP_SOURCE_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var r = /a/g; r.source = 'z'; return r.source; })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_regexp_source_write() {
    check(&q_refused(
        REFUSES_REGEXP_SOURCE_WRITE,
        "s0|a\ns1|a\ns2|a\n",
    ));
}

const REFUSES_ARGUMENTS_CALLEE_WRITE_ENUMERABLE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var a = (function(){ return arguments; })(1); a.callee = 9; var k = []; for (var x in a) k.push(x); return k.join(); })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_arguments_callee_write_enumerable() {
    check(&q_refused(
        REFUSES_ARGUMENTS_CALLEE_WRITE_ENUMERABLE,
        "s0|0\ns1|0\ns2|0\n",
    ));
}

const REFUSES_BOUND_LENGTH_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var f = (function(a, b){}).bind(null); f.length = 5; return f.length; })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_bound_length_write() {
    check(&q_refused(REFUSES_BOUND_LENGTH_WRITE, "s0|2\ns1|2\ns2|2\n"));
}

const REFUSES_ARGUMENTS_LENGTH_WRITE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + (((function(){ var a = (function(){ return arguments; })(1, 2); a.length = 1; return Array.prototype.join.call(a) + Object.keys(a).join(); })())); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_arguments_length_write() {
    check(&q_refused(
        REFUSES_ARGUMENTS_LENGTH_WRITE,
        "s0|10,1\ns1|10,1\ns2|10,1\n",
    ));
}

const REFUSES_DELETING_STRICT_ARGUMENTS_CALLEE: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ 'use strict'; var o = (function(){ return arguments; })(1); try { delete o.callee; return 'deleted'; } catch (e) { return e instanceof TypeError; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn refuses_deleting_strict_arguments_callee() {
    check(&q_refused(
        REFUSES_DELETING_STRICT_ARGUMENTS_CALLEE,
        "s0|true\ns1|true\ns2|true\n",
    ));
}
