use oxc_syntax::operator::{BinaryOperator as B, UnaryOperator as U};
use untwist_eval::fold::{fold_binary, fold_unary, value_to_const, Const};
use untwist_eval::value::Value;

fn n(v: f64) -> Const {
    Const::Num(v)
}
fn s(v: &str) -> Const {
    Const::Str(v.to_string())
}

#[test]
fn arithmetic_and_bitwise() {
    assert_eq!(
        fold_binary(B::Addition, &n(-66078.0), &n(-519880.0)),
        Some(n(-585958.0))
    );
    assert_eq!(
        fold_binary(B::Multiplication, &n(23.0), &n(44188.0)),
        Some(n(1016324.0))
    );
    assert_eq!(fold_binary(B::Division, &n(10.0), &n(2.0)), Some(n(5.0)));
    assert_eq!(
        fold_binary(B::Division, &n(1000.0), &n(30.0)),
        Some(n(1000.0 / 30.0))
    );
    assert_eq!(
        fold_binary(B::ShiftRightZeroFill, &n(-1.0), &n(0.0)),
        Some(n(4294967295.0))
    );
    assert_eq!(fold_binary(B::BitwiseXOR, &n(5.0), &n(3.0)), Some(n(6.0)));
    assert_eq!(
        fold_binary(B::Exponential, &n(2.0), &n(10.0)),
        Some(n(1024.0))
    );
    assert_eq!(fold_binary(B::Remainder, &n(-7.0), &n(3.0)), Some(n(-1.0)));
}

#[test]
fn exponential_nan_cases() {
    let Some(Const::Num(one_to_infinity)) = fold_binary(B::Exponential, &n(1.0), &n(f64::INFINITY))
    else {
        panic!("expected a numeric result");
    };
    assert!(one_to_infinity.is_nan());

    let Some(Const::Num(neg_one_to_neg_infinity)) =
        fold_binary(B::Exponential, &n(-1.0), &n(f64::NEG_INFINITY))
    else {
        panic!("expected a numeric result");
    };
    assert!(neg_one_to_neg_infinity.is_nan());

    let Some(Const::Num(two_to_nan)) = fold_binary(B::Exponential, &n(2.0), &n(f64::NAN)) else {
        panic!("expected a numeric result");
    };
    assert!(two_to_nan.is_nan());

    assert_eq!(fold_binary(B::Exponential, &n(2.0), &n(3.0)), Some(n(8.0)));
    assert_eq!(
        fold_binary(B::Exponential, &n(-0.0), &n(-1.0)),
        Some(n(f64::NEG_INFINITY))
    );
}

#[test]
fn plus_concatenates_when_a_side_is_a_string() {
    assert_eq!(
        fold_binary(B::Addition, &n(66078.0), &s("test")),
        Some(s("66078test"))
    );
    assert_eq!(fold_binary(B::Addition, &s("a"), &s("b")), Some(s("ab")));
    assert_eq!(
        fold_binary(B::Subtraction, &s("0xa6"), &n(-331.0)),
        Some(n(497.0))
    );
}

#[test]
fn unary_minus_on_string_coerces() {
    assert_eq!(fold_unary(U::UnaryNegation, &s("0xa6")), Some(n(-166.0)));
    assert_eq!(fold_unary(U::LogicalNot, &n(0.0)), Some(Const::Bool(true)));
    assert_eq!(fold_unary(U::BitwiseNot, &n(2.9)), Some(n(-3.0)));
    assert_eq!(fold_unary(U::Typeof, &s("x")), Some(s("string")));
}

#[test]
fn equality_tables() {
    assert_eq!(
        fold_binary(B::Equality, &Const::Null, &Const::Undefined),
        Some(Const::Bool(true))
    );
    assert_eq!(
        fold_binary(B::Equality, &Const::Null, &n(0.0)),
        Some(Const::Bool(false))
    );
    assert_eq!(
        fold_binary(B::Equality, &s(""), &n(0.0)),
        Some(Const::Bool(true))
    );
    assert_eq!(
        fold_binary(B::StrictEquality, &n(f64::NAN), &n(f64::NAN)),
        Some(Const::Bool(false))
    );
    assert_eq!(
        fold_binary(B::StrictEquality, &s("a"), &n(1.0)),
        Some(Const::Bool(false))
    );
    assert_eq!(fold_binary(B::In, &s("a"), &s("b")), None);
}

#[test]
fn a_value_turns_into_its_const_and_an_object_into_none() {
    assert_eq!(
        value_to_const(&Value::string("0x1cc")),
        Some(Const::Str("0x1cc".to_string()))
    );
    assert_eq!(
        value_to_const(&Value::Bool(false)),
        Some(Const::Bool(false))
    );
    assert_eq!(value_to_const(&Value::Null), Some(Const::Null));
    assert_eq!(value_to_const(&Value::Undefined), Some(Const::Undefined));
    assert_eq!(value_to_const(&Value::array(Vec::new(), None)), None);
}

#[test]
fn negative_zero_survives_a_fold() {
    let Some(Const::Num(n)) = fold_unary(U::UnaryNegation, &Const::Num(0.0)) else {
        panic!("expected a number");
    };
    assert!(n.is_sign_negative());
}

#[test]
fn constant_folding_runs_on_a_host_that_owns_no_realm() {
    untwist_eval::test_hooks::on_budget_host(|host| {
        let joined = untwist_eval::ops::binary_op(
            B::Addition,
            &Value::Number(1.0),
            &Value::Str(untwist_eval::value::JsString::from_str("x")),
            host,
        );
        assert!(matches!(joined, Ok(Value::Str(text)) if text.to_lossy_string() == "1x"));
        assert_eq!(
            untwist_eval::ops::unary_op(U::LogicalNot, &Value::Bool(true), host)
                .ok()
                .as_ref()
                .and_then(value_to_const),
            Some(Const::Bool(false))
        );
    });
    assert_eq!(fold_binary(B::Addition, &n(1.0), &n(2.0)), Some(n(3.0)));
    assert_eq!(fold_unary(U::UnaryNegation, &n(4.0)), Some(n(-4.0)));
}
