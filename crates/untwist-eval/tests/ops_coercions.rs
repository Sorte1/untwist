mod common;

use common::PureTestHost;
use untwist_eval::ops::{
    number_key, string_key, to_boolean, to_js_string, to_number_value, to_property_key, type_of,
};
use untwist_eval::value::{JsString, PropKey, Value};

#[test]
fn truthiness_matches_js() {
    assert!(!to_boolean(&Value::Number(0.0)));
    assert!(!to_boolean(&Value::Number(-0.0)));
    assert!(!to_boolean(&Value::Number(f64::NAN)));
    assert!(to_boolean(&Value::Number(1.0)));
    assert!(!to_boolean(&Value::Str(JsString::empty())));
    assert!(to_boolean(&Value::Str(JsString::from_str("0"))));
    assert!(!to_boolean(&Value::Undefined));
    assert!(!to_boolean(&Value::Null));
    assert!(to_boolean(&Value::array(Vec::new(), None)));
}

#[test]
fn typeof_never_throws_and_calls_null_an_object() {
    assert_eq!(type_of(&Value::Null), "object");
    assert_eq!(type_of(&Value::Undefined), "undefined");
    assert_eq!(type_of(&Value::Number(f64::NAN)), "number");
    assert_eq!(type_of(&Value::Str(JsString::empty())), "string");
    assert_eq!(type_of(&Value::array(Vec::new(), None)), "object");
}

#[test]
fn number_coercion_covers_the_hex_string_index_case() {
    let mut host = PureTestHost::default();
    let value = Value::Str(JsString::from_str("0x1cc"));
    assert_eq!(to_number_value(&value, &mut host).ok(), Some(460.0));
    assert_eq!(to_number_value(&Value::Null, &mut host).ok(), Some(0.0));
    assert!(to_number_value(&Value::Undefined, &mut host)
        .ok()
        .is_some_and(f64::is_nan));
    assert_eq!(
        to_number_value(&Value::Bool(true), &mut host).ok(),
        Some(1.0)
    );
}

#[test]
fn string_coercion_uses_ryu_js_formatting() {
    let mut host = PureTestHost::default();
    let rendered = to_js_string(&Value::Number(1e21), &mut host)
        .ok()
        .map(|s| s.to_lossy_string());
    assert_eq!(rendered.as_deref(), Some("1e+21"));
    let minus_zero = to_js_string(&Value::Number(-0.0), &mut host)
        .ok()
        .map(|s| s.to_lossy_string());
    assert_eq!(minus_zero.as_deref(), Some("0"));
}

#[test]
fn property_keys_normalise_integers_and_minus_zero() {
    let mut host = PureTestHost::default();
    assert_eq!(
        to_property_key(&Value::Number(1.0), &mut host).ok(),
        Some(PropKey::Index(1))
    );
    assert_eq!(
        to_property_key(&Value::Number(-0.0), &mut host).ok(),
        Some(PropKey::Index(0))
    );
    assert_eq!(
        to_property_key(&Value::Number(1.5), &mut host).ok(),
        Some(PropKey::Name(JsString::from_str("1.5")))
    );
    assert_eq!(
        to_property_key(&Value::Str(JsString::from_str("2")), &mut host).ok(),
        Some(PropKey::Index(2))
    );
}

#[test]
fn string_key_normalises_canonical_integer_indices() {
    assert_eq!(string_key(JsString::from_str("2")), PropKey::Index(2));
    assert_eq!(
        string_key(JsString::from_str("02")),
        PropKey::Name(JsString::from_str("02"))
    );
    assert_eq!(
        string_key(JsString::from_str("x")),
        PropKey::Name(JsString::from_str("x"))
    );
}

#[test]
fn number_key_normalises_minus_zero_and_non_indices() {
    assert_eq!(number_key(-0.0), PropKey::Index(0));
    assert_eq!(number_key(1.0), PropKey::Index(1));
    assert_eq!(number_key(1.5), PropKey::Name(JsString::from_str("1.5")));
}

#[test]
fn value_string_helper_builds_a_str_value() {
    assert!(matches!(Value::string("x"), Value::Str(_)));
}
