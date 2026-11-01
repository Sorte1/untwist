mod common;

use common::PureTestHost;
use oxc_syntax::operator::UnaryOperator as U;
use untwist_eval::ops::unary_op;
use untwist_eval::value::{JsString, Value};

fn num(value: &Value<'_>) -> f64 {
    match value {
        Value::Number(n) => *n,
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn bitwise_not_uses_to_int32_through_strings() {
    let mut host = PureTestHost::default();
    let minus_one = unary_op(U::BitwiseNot, &Value::Number(-1.0), &mut host).expect("ok");
    assert_eq!(num(&minus_one), 0.0);
    let from_string = unary_op(
        U::BitwiseNot,
        &Value::Str(JsString::from_str("a")),
        &mut host,
    )
    .expect("ok");
    assert_eq!(num(&from_string), -1.0);
    let truncating = unary_op(U::BitwiseNot, &Value::Number(2.9), &mut host).expect("ok");
    assert_eq!(num(&truncating), -3.0);
}

#[test]
fn unary_minus_on_a_hex_string_matches_the_calls_transform_fixture() {
    let mut host = PureTestHost::default();
    let value = unary_op(
        U::UnaryNegation,
        &Value::Str(JsString::from_str("0xa6")),
        &mut host,
    )
    .expect("ok");
    assert_eq!(num(&value), -166.0);
}

#[test]
fn unary_minus_preserves_negative_zero() {
    let mut host = PureTestHost::default();
    let value = unary_op(U::UnaryNegation, &Value::Number(0.0), &mut host).expect("ok");
    assert!(num(&value).is_sign_negative());
}

#[test]
fn typeof_and_void_and_not() {
    let mut host = PureTestHost::default();
    let kind = unary_op(U::Typeof, &Value::Null, &mut host).expect("ok");
    assert!(matches!(&kind, Value::Str(s) if s.to_lossy_string() == "object"));
    let void = unary_op(U::Void, &Value::Number(1.0), &mut host).expect("ok");
    assert!(matches!(void, Value::Undefined));
    let negated = unary_op(U::LogicalNot, &Value::Number(0.0), &mut host).expect("ok");
    assert!(matches!(negated, Value::Bool(true)));
}
