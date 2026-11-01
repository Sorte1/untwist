mod common;

use common::eval;

#[test]
fn blocks_get_their_own_scope_for_let_but_not_for_var() {
    assert_eq!(
        eval("var a = 1; { let a = 2; } a"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("var a = 1; { var a = 2; } a"),
        Ok("number:2".to_string())
    );
}

#[test]
fn if_else_selects_a_branch_and_return_leaves_the_function() {
    assert_eq!(
        eval("function f(x) { if (x) { return 'yes'; } else { return 'no'; } } f(0)"),
        Ok("string:no".to_string())
    );
    assert_eq!(
        eval("function f() { if (!![]) return 1; return 2; } f()"),
        Ok("number:1".to_string())
    );
}

#[test]
fn a_bare_return_yields_undefined() {
    assert_eq!(
        eval("function f() { return; } f()"),
        Ok("undefined".to_string())
    );
}

#[test]
fn throw_produces_a_catchable_abrupt() {
    assert_eq!(eval("throw 'boom';"), Err("throw string:boom".to_string()));
}

#[test]
fn a_debugger_statement_is_a_no_op() {
    assert_eq!(eval("debugger; 1"), Ok("number:1".to_string()));
}

#[test]
fn an_uninitialised_var_in_a_nested_block_reuses_the_hoisted_binding() {
    assert_eq!(
        eval("function f() { var x = 1; if (!![]) { var x; x = 2; } return x; } f()"),
        Ok("number:2".to_string())
    );
}

#[test]
fn an_uninitialised_var_does_not_erase_a_value_assigned_before_the_declaration() {
    assert_eq!(
        eval("function f() { x = 5; { var x; return x; } } f()"),
        Ok("number:5".to_string())
    );
}

#[test]
fn an_uninitialised_var_inside_try_reuses_the_hoisted_binding() {
    assert_eq!(
        eval("function f() { var x = 1; try { var x; x = 3; } catch (e) {} return x; } f()"),
        Ok("number:3".to_string())
    );
}

#[test]
fn for_of_over_an_array_hole_yields_undefined() {
    assert_eq!(
        eval("var out = []; for (var x of [1,,3]) { out.push(x === undefined ? 'u' : x); } out.join(',')"),
        Ok("string:1,u,3".to_string())
    );
}

#[test]
fn for_of_over_arguments_iterates_its_values() {
    assert_eq!(
        eval("function f() { var out = []; for (var x of arguments) { out.push(x); } return out.join(','); } f('a', 'b', 'c')"),
        Ok("string:a,b,c".to_string())
    );
}

#[test]
fn a_labelled_continue_from_a_nested_for_of_reaches_the_outer_loop() {
    assert_eq!(
        eval(
            "var out = []; outer: for (var i of [1, 2, 3]) { for (var j of [10, 20]) { if (j === 20) continue outer; out.push(i + ':' + j); } } out.join(',')"
        ),
        Ok("string:1:10,2:10,3:10".to_string())
    );
}

#[test]
fn a_labelled_break_from_a_nested_for_of_stops_the_outer_loop() {
    assert_eq!(
        eval(
            "var out = []; outer: for (var i of [1, 2, 3]) { for (var j of [10, 20]) { if (i === 2) break outer; out.push(i + ':' + j); } } out.join(',')"
        ),
        Ok("string:1:10,1:20".to_string())
    );
}

#[test]
fn a_destructuring_for_of_left_side_bails() {
    assert_eq!(
        eval("for (var [a, b] of [[1, 2]]) { a; }"),
        Err("unsupported construct: destructuring for-in binding".to_string())
    );
}

#[test]
fn for_of_gives_each_let_iteration_its_own_binding_for_closures() {
    assert_eq!(
        eval(
            "var closures = []; for (let x of [1, 2, 3]) { closures.push(function () { return x; }); } var out = ''; for (var i = 0; i < closures.length; i++) { out += closures[i](); } out"
        ),
        Ok("string:123".to_string())
    );
}

#[test]
fn for_of_with_var_shares_one_binding_across_iterations_for_closures() {
    assert_eq!(
        eval(
            "var closures = []; for (var x of [1, 2, 3]) { closures.push(function () { return x; }); } var out = ''; for (var i = 0; i < closures.length; i++) { out += closures[i](); } out"
        ),
        Ok("string:333".to_string())
    );
}

#[test]
fn for_of_reads_the_live_array() {
    assert_eq!(
        eval("var a = [1, 2]; var s = ''; for (var x of a) { if (a.length < 4) a['push'](x * 10); a[1] = 5; s += x + ';'; } var b = [1, 2, 3]; for (var y of b) { b['pop'](); s += y; } s"),
        Ok("string:1;5;10;50;12".to_string())
    );
}
