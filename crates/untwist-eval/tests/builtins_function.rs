mod common;

use common::eval;

#[test]
fn call_and_apply_set_this_and_spread_arguments() {
    assert_eq!(
        eval("function f(a, b) { return this.n + a + b; } f['call']({ n: 1 }, 2, 3)"),
        Ok("number:6".to_string())
    );
    assert_eq!(
        eval("function f(a, b) { return a + b; } f['apply'](null, [2, 3])"),
        Ok("number:5".to_string())
    );
    assert_eq!(
        eval("function f() { return arguments.length; } f['apply'](null)"),
        Ok("number:0".to_string())
    );
}

#[test]
fn bind_partially_applies_and_pins_this() {
    assert_eq!(
        eval("function f(a, b) { return this.n + a + b; } f['bind']({ n: 1 }, 2)(3)"),
        Ok("number:6".to_string())
    );
}

#[test]
fn to_string_returns_source_and_a_bound_function_bails() {
    assert_eq!(
        eval("(function(){return 0;})['toString']()['indexOf']('return')"),
        Ok("number:11".to_string())
    );
    assert_eq!(
        eval("(function(){return 0;})['bind']()['toString']()['indexOf']('native code')"),
        Err("unsupported construct: the source text of a bound function".to_string())
    );
}

#[test]
fn string_coercion_of_a_function_uses_to_string() {
    assert_eq!(
        eval("('' + function(){return 0;})['indexOf']('function')"),
        Ok("number:0".to_string())
    );
}

#[test]
fn apply_accepts_an_array_like_arguments_object() {
    assert_eq!(
        eval(
            "function f(a, b) { return a + b; } \
             function g() { return f['apply'](null, arguments); } \
             g(2, 3)"
        ),
        Ok("number:5".to_string())
    );
}

#[test]
fn apply_with_a_non_object_second_argument_throws() {
    assert_eq!(
        eval("function f() {} f['apply'](null, 1)"),
        Err("throw error:TypeError".to_string())
    );
}
