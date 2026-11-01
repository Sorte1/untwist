mod common;

use common::eval;

#[test]
fn parse_int_is_the_rotator_primitive() {
    assert_eq!(
        eval("parseInt('590708XPLBWj')"),
        Ok("number:590708".to_string())
    );
    assert_eq!(eval("parseInt('ff', 16)"), Ok("number:255".to_string()));
    assert_eq!(eval("parseInt('')"), Ok("number:NaN".to_string()));
    assert_eq!(eval("parseInt(0.0000005)"), Ok("number:5".to_string()));
}

#[test]
fn number_string_and_boolean_conversions() {
    assert_eq!(eval("Number('0x1cc')"), Ok("number:460".to_string()));
    assert_eq!(eval("Number()"), Ok("number:0".to_string()));
    assert_eq!(eval("String(255)"), Ok("string:255".to_string()));
    assert_eq!(eval("String(-0)"), Ok("string:0".to_string()));
    assert_eq!(eval("Boolean([])"), Ok("boolean:true".to_string()));
    assert_eq!(eval("Boolean('')"), Ok("boolean:false".to_string()));
}

#[test]
fn is_nan_and_is_finite() {
    assert_eq!(eval("isNaN('12px')"), Ok("boolean:true".to_string()));
    assert_eq!(eval("isNaN('12')"), Ok("boolean:false".to_string()));
    assert_eq!(eval("isFinite(Infinity)"), Ok("boolean:false".to_string()));
}

#[test]
fn parse_float_takes_the_decimal_prefix() {
    assert_eq!(eval("parseFloat('3.14abc')"), Ok("number:3.14".to_string()));
}
