mod common;

use common::PureTestHost;
use oxc_syntax::operator::BinaryOperator as B;
use untwist_eval::ops::{binary_op, loose_equals, strict_equals};
use untwist_eval::value::{JsString, Value};

fn s(text: &str) -> Value<'static> {
    Value::Str(JsString::from_str(text))
}

fn apply(op: B, left: Value<'static>, right: Value<'static>) -> Value<'static> {
    let mut host = PureTestHost::default();
    binary_op(op, &left, &right, &mut host).expect("ok")
}

#[test]
fn addition_concatenates_when_either_side_is_a_string() {
    let joined = apply(B::Addition, Value::Number(353.0), s("Kh0Ph"));
    assert!(matches!(&joined, Value::Str(v) if v.to_lossy_string() == "353Kh0Ph"));
    let summed = apply(B::Addition, Value::Number(1.0), Value::Bool(true));
    assert!(matches!(summed, Value::Number(n) if n == 2.0));
    let subtracted = apply(B::Subtraction, s("0x1cc"), Value::Number(0x100 as f64));
    assert!(matches!(subtracted, Value::Number(n) if n == 204.0));
}

#[test]
fn bitwise_and_shifts_follow_to_int32_and_to_uint32() {
    assert!(matches!(
        apply(B::ShiftRightZeroFill, Value::Number(-1.0), Value::Number(0.0)),
        Value::Number(n) if n == 4294967295.0
    ));
    assert!(matches!(
        apply(B::ShiftRight, Value::Number(-8.0), Value::Number(1.0)),
        Value::Number(n) if n == -4.0
    ));
    assert!(matches!(
        apply(B::ShiftLeft, Value::Number(1.0), Value::Number(33.0)),
        Value::Number(n) if n == 2.0
    ));
    assert!(matches!(
        apply(B::BitwiseAnd, Value::Number(255.0), Value::Number(-2.0)),
        Value::Number(n) if n == 254.0
    ));
}

#[test]
fn exponentiation_follows_the_js_spec_not_rust_powf() {
    assert!(matches!(
        apply(B::Exponential, Value::Number(1.0), Value::Number(f64::NAN)),
        Value::Number(n) if n.is_nan()
    ));
    assert!(matches!(
        apply(B::Exponential, Value::Number(-1.0), Value::Number(f64::INFINITY)),
        Value::Number(n) if n.is_nan()
    ));
    assert!(matches!(
        apply(B::Exponential, Value::Number(f64::NAN), Value::Number(0.0)),
        Value::Number(n) if n == 1.0
    ));
    assert!(matches!(
        apply(B::Exponential, Value::Number(-8.0), Value::Number(1.0 / 3.0)),
        Value::Number(n) if n.is_nan()
    ));
}

#[test]
fn relational_comparison_uses_code_unit_order_for_two_strings() {
    assert!(matches!(
        apply(B::LessThan, s("a"), s("b")),
        Value::Bool(true)
    ));
    assert!(matches!(
        apply(B::LessThan, Value::Number(f64::NAN), Value::Number(1.0)),
        Value::Bool(false)
    ));
    assert!(matches!(
        apply(
            B::GreaterEqualThan,
            Value::Number(f64::NAN),
            Value::Number(1.0)
        ),
        Value::Bool(false)
    ));
}

#[test]
fn strict_equality_treats_nan_and_minus_zero_the_js_way() {
    assert!(!strict_equals(
        &Value::Number(f64::NAN),
        &Value::Number(f64::NAN)
    ));
    assert!(strict_equals(&Value::Number(-0.0), &Value::Number(0.0)));
    assert!(!strict_equals(&Value::Number(1.0), &s("1")));
    assert!(strict_equals(&Value::Undefined, &Value::Undefined));
    assert!(!strict_equals(&Value::Null, &Value::Undefined));
    let shared = Value::array(Vec::new(), None);
    assert!(strict_equals(&shared, &shared.clone()));
    assert!(!strict_equals(&shared, &Value::array(Vec::new(), None)));
}

#[test]
fn abstract_equality_follows_the_spec_table() {
    let mut host = PureTestHost::default();
    assert_eq!(
        loose_equals(&Value::Null, &Value::Undefined, &mut host).ok(),
        Some(true)
    );
    assert_eq!(
        loose_equals(&Value::Null, &Value::Number(0.0), &mut host).ok(),
        Some(false)
    );
    assert_eq!(
        loose_equals(&s(""), &Value::Number(0.0), &mut host).ok(),
        Some(true)
    );
    assert_eq!(
        loose_equals(&s("0"), &Value::Bool(false), &mut host).ok(),
        Some(true)
    );
}
