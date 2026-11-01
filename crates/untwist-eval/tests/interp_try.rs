mod common;

use common::{eval, eval_with};
use untwist_eval::budget::Budget;

#[test]
fn catch_binds_the_thrown_value_and_the_block_continues() {
    assert_eq!(
        eval("var out = ''; try { throw 'x'; } catch (e) { out = 'caught:' + e; } out"),
        Ok("string:caught:x".to_string())
    );
}

#[test]
fn finally_runs_on_both_paths_and_does_not_swallow_the_completion() {
    assert_eq!(
        eval("var log = ''; function f() { try { return 'r'; } finally { log = 'f'; } } f() + log"),
        Ok("string:rf".to_string())
    );
    assert_eq!(
        eval(
            "var log = ''; try { throw 1; } catch (e) { log += 'c'; } finally { log += 'f'; } log"
        ),
        Ok("string:cf".to_string())
    );
}

#[test]
fn try_normal_plus_finally_abrupt_returns_the_finally_value() {
    assert_eq!(
        eval("function f() { try { 1; } finally { return 2; } } f()"),
        Ok("number:2".to_string())
    );
}

#[test]
fn try_return_plus_finally_return_returns_the_finally_value() {
    assert_eq!(
        eval("function f() { try { return 1; } finally { return 2; } } f()"),
        Ok("number:2".to_string())
    );
}

#[test]
fn try_uncaught_throw_plus_finally_normal_runs_the_finally_and_rethrows() {
    assert_eq!(
        eval("var g = 0; function f() { try { throw 'boom'; } finally { g = 1; } } var r; try { f(); } catch (e) { r = e + g; } r"),
        Ok("string:boom1".to_string())
    );
}

#[test]
fn try_bail_plus_finally_normal_does_not_replace_the_bail() {
    let result = eval("var g = 0; try { while (true) {} } finally { g = 1; } g");
    let Err(message) = result else {
        panic!("expected an Err, got {result:?}");
    };
    assert!(message.contains("loop iteration cap exceeded"));
    assert!(!message.contains("throw"));
}

#[test]
fn a_catch_cannot_swallow_a_bail() {
    assert_eq!(
        eval_with(
            "try { for (;;) {} } catch (e) { 'swallowed' }",
            Budget::new(50_000)
        ),
        Err("step budget exhausted".to_string())
    );
    assert_eq!(
        eval("try { unresolvedThing; } catch (e) { 'swallowed' }"),
        Err("unresolved global binding: unresolvedThing".to_string())
    );
}

#[test]
fn the_rotator_control_flow_shape_terminates() {
    let source = "\
var arr = ['a', 'b', 'c'];
var steps = 0;
while (!![]) {
  try {
    steps++;
    if (arr[0] === 'c') break;
    else { arr['push'](arr['shift']()); }
  } catch (e) { arr['push'](arr['shift']()); }
}
steps";
    assert_eq!(eval(source), Ok("number:3".to_string()));
}

#[test]
fn switch_falls_through_and_break_stops_it() {
    let source = "\
function f(x) { var out = ''; switch (x) { case 0: out += 'a'; case 1: out += 'b'; break; default: out += 'd'; } return out; }
f(0) + '|' + f(1) + '|' + f(9)";
    assert_eq!(eval(source), Ok("string:ab|b|d".to_string()));
}

#[test]
fn try_bail_plus_finally_return_does_not_replace_the_bail() {
    assert_eq!(
        eval("function f() { try { unresolvedThing; } finally { return 1; } } f()"),
        Err("unresolved global binding: unresolvedThing".to_string())
    );
}

#[test]
fn try_bail_plus_finally_break_does_not_replace_the_bail() {
    assert_eq!(
        eval("var g = 0; for (;;) { try { unresolvedThing; } finally { break; } } g"),
        Err("unresolved global binding: unresolvedThing".to_string())
    );
}
