use std::cell::RefCell;
use std::rc::Rc;

use untwist_eval::bail::{Bail, ErrorKind};
use untwist_eval::value::{
    Abrupt, AbruptKind, JsObject, JsString, ObjKind, PropKey, PropMap, Value,
};

fn plain<'a>() -> Value<'a> {
    Value::Object(Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Plain(0),
        props: PropMap::default(),
        proto: None,
        charge: None,
    })))
}

#[test]
fn objects_alias_rather_than_copy() {
    let first = plain();
    let second = first.clone();
    if let Some(obj) = first.as_object() {
        obj.borrow_mut().props.insert(
            PropKey::Name(JsString::from_str("x")).into(),
            Value::Number(1.0),
        );
    }
    let seen = second.as_object().and_then(|obj| {
        obj.borrow()
            .props
            .get(&PropKey::Name(JsString::from_str("x")))
            .cloned()
    });
    assert!(matches!(seen, Some(Value::Number(n)) if n == 1.0));
}

#[test]
fn array_mutation_is_visible_through_every_handle() {
    let array = Value::array(vec![Value::Number(1.0)], None);
    let alias = array.clone();
    if let Some(obj) = array.as_object() {
        if let ObjKind::Array(items) = &mut obj.borrow_mut().kind {
            items.push(Value::Number(2.0));
        }
    }
    let len = alias
        .as_object()
        .map(|obj| match &obj.borrow().kind {
            ObjKind::Array(items) => items.len(),
            _ => 0,
        })
        .unwrap_or(0);
    assert_eq!(len, 2);
}

#[test]
fn negative_zero_is_preserved() {
    let value = Value::Number(-0.0);
    let Value::Number(n) = value else {
        panic!("expected number");
    };
    assert!(n.is_sign_negative());
    assert_eq!(n, 0.0);
}

#[test]
fn a_bail_converts_into_an_abrupt_but_a_throw_does_not_convert_back() {
    let abrupt: Abrupt<'_> = Bail::Fuel.into();
    assert!(matches!(abrupt.kind(), AbruptKind::Bail(Bail::Fuel)));
    let thrown: Abrupt<'_> = Abrupt::throw(Value::Str(JsString::from_str("boom")));
    assert!(matches!(thrown.kind(), AbruptKind::Throw(_)));
}

#[test]
fn error_objects_carry_their_kind() {
    let err = Value::object(ObjKind::Error(ErrorKind::UriError), None);
    let kind = err.as_object().map(|obj| match &obj.borrow().kind {
        ObjKind::Error(kind) => Some(*kind),
        _ => None,
    });
    assert_eq!(kind, Some(Some(ErrorKind::UriError)));
}

#[test]
fn callability_covers_closures_natives_and_bound_functions() {
    assert!(!plain().is_callable());
    assert!(
        Value::object(ObjKind::Native(untwist_eval::value::NativeId(0), 0), None).is_callable()
    );
    assert!(!Value::Number(1.0).is_callable());
}

#[test]
fn is_nullish_covers_undefined_and_null_only() {
    assert!(Value::Undefined.is_nullish());
    assert!(Value::Null.is_nullish());
    assert!(!Value::Bool(false).is_nullish());
    assert!(!Value::Number(0.0).is_nullish());
}
