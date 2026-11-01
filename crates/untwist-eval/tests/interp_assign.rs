mod common;

use common::eval;

#[test]
fn assignment_yields_the_assigned_value_and_writes_through() {
    assert_eq!(
        eval("var f = 0x1cc; (f = f - 0x1cc)"),
        Ok("number:0".to_string())
    );
    assert_eq!(
        eval("var o = ''; o += 'ab'; o += 'c'; o"),
        Ok("string:abc".to_string())
    );
    assert_eq!(eval("var q = 5; q %= 4; q"), Ok("number:1".to_string()));
    assert_eq!(eval("var r = 1; r <<= 3; r"), Ok("number:8".to_string()));
}

#[test]
fn member_assignment_grows_arrays_and_writes_function_properties() {
    assert_eq!(
        eval("var s = []; s[3] = 1; s.length"),
        Ok("number:4".to_string())
    );
    assert_eq!(
        eval("function b() {} b['HujJww'] = !![]; b['HujJww']"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn postfix_increment_yields_the_old_value_and_prefix_the_new() {
    assert_eq!(
        eval("var t = 0; var u = t++; u"),
        Ok("number:0".to_string())
    );
    assert_eq!(eval("var t = 0; t++; t"), Ok("number:1".to_string()));
    assert_eq!(eval("var t = 0; ++t"), Ok("number:1".to_string()));
    assert_eq!(
        eval("var arr = [3]; --arr[0]; arr[0]"),
        Ok("number:2".to_string())
    );
}

#[test]
fn logical_assignment_short_circuits() {
    assert_eq!(eval("var v = 0; v ||= 9; v"), Ok("number:9".to_string()));
    assert_eq!(eval("var v = 1; v &&= 9; v"), Ok("number:9".to_string()));
    assert_eq!(eval("var v = null; v ??= 4; v"), Ok("number:4".to_string()));
}

#[test]
fn delete_removes_an_own_property() {
    assert_eq!(
        eval("var k = { c: 1 }; delete k.c; 'c' in k"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn destructuring_assignment_bails() {
    assert_eq!(
        eval("var a, b; [a, b] = [1, 2];"),
        Err("unsupported construct: destructuring assignment target".to_string())
    );
}

#[test]
fn compound_assignment_evaluates_a_member_target_only_once() {
    assert_eq!(
        eval("var i = 0; var arr = [10, 20]; arr[(i++, 0)] += 5; i"),
        Ok("number:1".to_string())
    );
}

#[test]
fn update_expression_evaluates_a_member_target_only_once() {
    assert_eq!(
        eval("var i = 0; var arr = [10, 20]; arr[(i++, 0)]++; i"),
        Ok("number:1".to_string())
    );
}

#[test]
fn compound_assignment_evaluates_a_side_effecting_object_expression_only_once() {
    assert_eq!(
        eval("var n = 0; var o = {p: 1}; (n++, o).p += 1; n"),
        Ok("number:1".to_string())
    );
}
