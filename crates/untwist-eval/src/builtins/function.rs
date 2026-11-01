use std::rc::Rc;

use crate::bail::{Bail, ErrorKind};
use crate::budget::BOUND_CHAIN_CAP;
use crate::builtins::args::{argument, charge_for_length, grow_object, slot_bytes};
use crate::builtins::{NativeFn, NativeTable, Realm};
use crate::host::Host;
use crate::object::array_items;
use crate::value::{BoundFunction, Eval, ObjKind, Value};

const BOUND_BYTES: u64 = 64;

pub fn install<'a>(realm: &mut Realm<'a>, table: &mut NativeTable<'a>) {
    let members: [(&str, NativeFn<'a>); 4] = [
        ("call", native_call),
        ("apply", native_apply),
        ("bind", native_bind),
        ("toString", native_to_string),
    ];
    for (name, function) in members {
        let value = realm.native_value(table, ("Function.prototype", name), function);
        realm.define_function_member(name, value);
    }
}

fn native_call<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if !this.is_callable() {
        return Err(host.throw(
            ErrorKind::TypeError,
            "Function.prototype.call on a non-function",
        ));
    }
    let receiver = argument(args, 0);
    host.call_value(&this, receiver, &args[args.len().min(1)..])
}

fn native_apply<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if !this.is_callable() {
        return Err(host.throw(
            ErrorKind::TypeError,
            "Function.prototype.apply on a non-function",
        ));
    }
    let receiver = argument(args, 0);
    let list = match args.get(1) {
        None | Some(Value::Undefined) | Some(Value::Null) => Vec::new(),
        Some(value) => match value.as_object().and_then(array_items) {
            Some(items) => items,
            None if value.as_object().is_some() => {
                return Err(Bail::Unsupported(&"apply with an array-like object").into());
            }
            None => {
                return Err(host.throw(ErrorKind::TypeError, "apply arguments are not an array"))
            }
        },
    };
    charge_for_length(host, list.len())?;
    host.call_value(&this, receiver, &list)
}

fn bound_depth(value: &Value<'_>) -> usize {
    let mut depth = 0;
    let mut current = value.clone();
    while let Some(inner) = unwrap_bound(&current) {
        depth += 1;
        if depth >= BOUND_CHAIN_CAP {
            break;
        }
        current = inner;
    }
    depth
}

fn native_bind<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if !this.is_callable() {
        return Err(host.throw(
            ErrorKind::TypeError,
            "Function.prototype.bind on a non-function",
        ));
    }
    if bound_depth(&this) >= BOUND_CHAIN_CAP {
        return Err(Bail::ChainTooDeep(&"bound function").into());
    }
    let receiver = argument(args, 0);
    let partial: Vec<Value<'a>> = args.iter().skip(1).cloned().collect();
    charge_for_length(host, partial.len())?;
    let slots = slot_bytes(partial.capacity()).saturating_add(BOUND_BYTES);
    let bound = host.new_object(
        ObjKind::Bound(Rc::new(BoundFunction {
            target: this,
            this: receiver,
            args: partial,
        })),
        None,
    );
    if let Some(obj) = bound.as_object() {
        grow_object(host, obj, slots)?;
    }
    Ok(bound)
}

fn native_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Str(host.function_source(&this)?))
}

pub fn unwrap_bound<'a>(value: &Value<'a>) -> Option<Value<'a>> {
    let obj = value.as_object()?;
    let ObjKind::Bound(bound) = &obj.borrow().kind else {
        return None;
    };
    Some(bound.target.clone())
}
