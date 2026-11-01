mod common;

use common::eval;

#[test]
fn the_high_preset_fake_class_shape_runs() {
    let source = "\
var C = function (fn) { this['hjROCd'] = fn; this['EwIbIj'] = [1, 0, 1]; };
C['prototype']['tlFPzk'] = function () { return this['EwIbIj'][0] + 1; };
C['prototype']['other'] = function () { return this['hjROCd']; };
new C(7)['tlFPzk']()";
    assert_eq!(eval(source), Ok("number:2".to_string()));
}

#[test]
fn a_constructor_returning_an_object_wins_over_this() {
    assert_eq!(
        eval("function C() { return { a: 5 }; } new C().a"),
        Ok("number:5".to_string())
    );
    assert_eq!(
        eval("function C() { this.a = 1; return 9; } new C().a"),
        Ok("number:1".to_string())
    );
}

#[test]
fn prototype_methods_see_the_instance_as_this() {
    let source = "\
function C(v) { this.v = v; }
C.prototype.get = function () { return this.v; };
var made = new C('x');
made.get()";
    assert_eq!(eval(source), Ok("string:x".to_string()));
}

#[test]
fn constructing_a_non_function_throws() {
    assert_eq!(
        eval("var x = 1; new x()"),
        Err("throw error:TypeError".to_string())
    );
}
