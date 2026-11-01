mod common;

use common::eval;

#[test]
fn the_atob_loop_header_shape_runs() {
    let source = "\
var input = 'abc';
var out = '';
for (var idx = 0, ch; (ch = input['charAt'](idx++));) { out += ch; }
out";
    assert_eq!(eval(source), Ok("string:abc".to_string()));
}

#[test]
fn while_do_while_and_break_continue_work() {
    assert_eq!(
        eval("var i = 0; while (!![]) { i++; if (i > 2) break; } i"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        eval("var i = 0; do { i++; } while (i < 3); i"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        eval("var s = 0; for (var i = 0; i < 5; i++) { if (i % 2) continue; s += i; } s"),
        Ok("number:6".to_string())
    );
}

#[test]
fn labelled_break_leaves_the_outer_loop() {
    let source = "\
var hits = 0;
outer: for (var i = 0; i < 3; i++) { for (var j = 0; j < 3; j++) { hits++; if (j === 1) continue outer; if (i === 2) break outer; } }
hits";
    assert_eq!(eval(source), Ok("number:5".to_string()));
}

#[test]
fn for_in_enumerates_own_keys_in_insertion_order() {
    assert_eq!(
        eval("var o = { a: 1, b: 2 }; var k = ''; for (var key in o) { k += key; } k"),
        Ok("string:ab".to_string())
    );
}

#[test]
fn an_unbounded_loop_hits_the_iteration_cap_not_the_wall_clock() {
    assert_eq!(
        eval("for (; [];) {}"),
        Err("loop iteration cap exceeded".to_string())
    );
}
