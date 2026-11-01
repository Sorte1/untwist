use std::mem;
use std::rc::Rc;

use crate::frame::{Frame, FrameRef};
use crate::value::{JsObject, ObjKind, ObjRef, Value};

enum Pending<'a> {
    Object(ObjRef<'a>),
    Scope(FrameRef<'a>),
}

pub(crate) fn release_object(object: &mut JsObject<'_>) {
    if !object_owns_a_child(object) {
        return;
    }
    let mut pending = Vec::new();
    collect_object(object, &mut pending);
    drain(pending);
}

pub(crate) fn release_frame(scope: &mut Frame<'_>) {
    if !scope_owns_a_child(scope) {
        return;
    }
    let mut pending = Vec::new();
    collect_scope(scope, &mut pending);
    drain(pending);
}

fn object_owns_a_child(object: &JsObject<'_>) -> bool {
    object.props.values().any(is_sole_owner)
        || object
            .proto
            .as_ref()
            .is_some_and(|proto| Rc::strong_count(proto) == 1)
        || kind_owns_a_child(&object.kind)
}

fn kind_owns_a_child(kind: &ObjKind<'_>) -> bool {
    match kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => items.iter().any(is_sole_owner),
        ObjKind::MappedArguments(mapped) => {
            mapped.items.iter().any(is_sole_owner) || Rc::strong_count(&mapped.scope) == 1
        }
        ObjKind::Function(closure) => Rc::strong_count(closure) == 1,
        ObjKind::Bound(bound) => Rc::strong_count(bound) == 1,
        ObjKind::Plain(_) | ObjKind::Native(_, _) | ObjKind::Error(_) | ObjKind::RegExp(_) => false,
    }
}

fn scope_owns_a_child(scope: &Frame<'_>) -> bool {
    scope.body.any_value(is_sole_owner)
        || scope
            .parent
            .as_ref()
            .is_some_and(|parent| Rc::strong_count(parent) == 1)
}

fn is_sole_owner(value: &Value<'_>) -> bool {
    matches!(value, Value::Object(obj) if Rc::strong_count(obj) == 1)
}

fn drain(mut pending: Vec<Pending<'_>>) {
    while let Some(next) = pending.pop() {
        match next {
            Pending::Object(obj) => {
                if let Ok(cell) = Rc::try_unwrap(obj) {
                    collect_object(&mut cell.into_inner(), &mut pending);
                }
            }
            Pending::Scope(env) => {
                if let Ok(cell) = Rc::try_unwrap(env) {
                    collect_scope(&mut cell.into_inner(), &mut pending);
                }
            }
        }
    }
}

fn collect_object<'a>(object: &mut JsObject<'a>, pending: &mut Vec<Pending<'a>>) {
    for (_, value) in mem::take(&mut object.props) {
        keep_value(value, pending);
    }
    if let Some(proto) = object.proto.take() {
        keep_object(proto, pending);
    }
    match mem::replace(&mut object.kind, ObjKind::Plain(0)) {
        ObjKind::Array(items) | ObjKind::Arguments(items) => {
            for value in items {
                keep_value(value, pending);
            }
        }
        ObjKind::MappedArguments(mapped) => {
            let mapped = *mapped;
            for value in mapped.items {
                keep_value(value, pending);
            }
            keep_scope(mapped.scope, pending);
        }
        ObjKind::Function(closure) => {
            if let Ok(closure) = Rc::try_unwrap(closure) {
                keep_scope(closure.env, pending);
                if let Some(this) = closure.lexical_this {
                    keep_value(*this, pending);
                }
            }
        }
        ObjKind::Bound(bound) => {
            if let Ok(bound) = Rc::try_unwrap(bound) {
                keep_value(bound.target, pending);
                keep_value(bound.this, pending);
                for value in bound.args {
                    keep_value(value, pending);
                }
            }
        }
        ObjKind::Plain(_) | ObjKind::Native(_, _) | ObjKind::Error(_) | ObjKind::RegExp(_) => {}
    }
}

fn collect_scope<'a>(scope: &mut Frame<'a>, pending: &mut Vec<Pending<'a>>) {
    mem::take(&mut scope.body).consume_values(|value| keep_value(value, pending));
    if let Some(parent) = scope.parent.take() {
        keep_scope(parent, pending);
    }
}

fn keep_value<'a>(value: Value<'a>, pending: &mut Vec<Pending<'a>>) {
    if let Value::Object(obj) = value {
        keep_object(obj, pending);
    }
}

fn keep_object<'a>(obj: ObjRef<'a>, pending: &mut Vec<Pending<'a>>) {
    if Rc::strong_count(&obj) == 1 {
        pending.push(Pending::Object(obj));
    }
}

fn keep_scope<'a>(env: FrameRef<'a>, pending: &mut Vec<Pending<'a>>) {
    if Rc::strong_count(&env) == 1 {
        pending.push(Pending::Scope(env));
    }
}
