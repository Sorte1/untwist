mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::q_refused;
use refused_rows::check;

const INDEX_OF_WHOSE_FROM_INDEX_WRITES_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.indexOf(3, { valueOf: function(){ a[2] = 9; return 0; } }); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn index_of_whose_from_index_writes_the_array_is_refused() {
    check(&q_refused(
        INDEX_OF_WHOSE_FROM_INDEX_WRITES_THE_ARRAY_IS_REFUSED,
        "s0|-1\ns1|-1\ns2|-1\n",
    ));
}

const INCLUDES_WHOSE_FROM_INDEX_WRITES_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.includes(9, { valueOf: function(){ a[0] = 9; return 0; } }); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn includes_whose_from_index_writes_the_array_is_refused() {
    check(&q_refused(
        INCLUDES_WHOSE_FROM_INDEX_WRITES_THE_ARRAY_IS_REFUSED,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const INCLUDES_WHOSE_FROM_INDEX_SHRINKS_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.includes(undefined, { valueOf: function(){ a.length = 0; return 0; } }); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn includes_whose_from_index_shrinks_the_array_is_refused() {
    check(&q_refused(
        INCLUDES_WHOSE_FROM_INDEX_SHRINKS_THE_ARRAY_IS_REFUSED,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const JOIN_WHOSE_SEPARATOR_WRITES_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.join({ toString: function(){ a[2] = 9; return '-'; } }); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn join_whose_separator_writes_the_array_is_refused() {
    check(&q_refused(
        JOIN_WHOSE_SEPARATOR_WRITES_THE_ARRAY_IS_REFUSED,
        "s0|1-2-9\ns1|1-2-9\ns2|1-2-9\n",
    ));
}

const JOIN_WHOSE_ELEMENT_WRITES_A_LATER_ONE_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [{ toString: function(){ a[1] = 'Z'; return 'x'; } }, 'y']; return a.join(); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn join_whose_element_writes_a_later_one_is_refused() {
    check(&q_refused(
        JOIN_WHOSE_ELEMENT_WRITES_A_LATER_ONE_IS_REFUSED,
        "s0|x,Z\ns1|x,Z\ns2|x,Z\n",
    ));
}

const JOIN_WHOSE_ELEMENT_SHRINKS_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [{ toString: function(){ a.length = 1; return 'x'; } }, 'y', 'w']; return a.join('-'); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn join_whose_element_shrinks_the_array_is_refused() {
    check(&q_refused(
        JOIN_WHOSE_ELEMENT_SHRINKS_THE_ARRAY_IS_REFUSED,
        "s0|x--\ns1|x--\ns2|x--\n",
    ));
}

const SLICE_WHOSE_START_WRITES_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.slice({ valueOf: function(){ a[1] = 9; return 0; } }).join(); })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn slice_whose_start_writes_the_array_is_refused() {
    check(&q_refused(
        SLICE_WHOSE_START_WRITES_THE_ARRAY_IS_REFUSED,
        "s0|1,9,3\ns1|1,9,3\ns2|1,9,3\n",
    ));
}

const SLICE_WHOSE_END_SHRINKS_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var a = [1, 2, 3]; var s = a.slice(0, { valueOf: function(){ a.length = 1; return 3; } }); return s.length + ':' + s.join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn slice_whose_end_shrinks_the_array_is_refused() {
    check(&q_refused(
        SLICE_WHOSE_END_SHRINKS_THE_ARRAY_IS_REFUSED,
        "s0|3:1,,\ns1|3:1,,\ns2|3:1,,\n",
    ));
}

const SPLICE_WHOSE_START_GROWS_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var a = [1, 2, 3]; var r = a.splice({ valueOf: function(){ a.push(4, 5); return -1; } }, 1); return r.join() + '/' + a.join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn splice_whose_start_grows_the_array_is_refused() {
    check(&q_refused(
        SPLICE_WHOSE_START_GROWS_THE_ARRAY_IS_REFUSED,
        "s0|3/1,2\ns1|3/1,2\ns2|3/1,2\n",
    ));
}

const SPLICE_WHOSE_COUNT_SHRINKS_THE_ARRAY_IS_REFUSED: &str = r#"function qa(){ var h=['s0','s1','s2']; qa=function(){return h;}; return qa(); }
function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { var a = [1, 2, 3, 4]; var r = a.splice(1, { valueOf: function(){ a.length = 2; return 2; } }); return r.join() + '/' + a.join(); } catch (z) { return 'caught'; } })()); }
function later(){ return qb(1); }
console.log(qb(0)); console.log(later()); console.log(qb(2));"#;

#[test]
fn splice_whose_count_shrinks_the_array_is_refused() {
    check(&q_refused(
        SPLICE_WHOSE_COUNT_SHRINKS_THE_ARRAY_IS_REFUSED,
        "s0|2,/1,\ns1|2,/1,\ns2|2,/1,\n",
    ));
}
