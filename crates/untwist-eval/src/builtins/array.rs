use crate::bail::EvalResult;
use crate::bail::{Bail, ErrorKind};
use crate::builtins::args::{
    adopt_array, argument, charge_for_length, check_array_growth, grow_array, grow_object,
    live_array, reserve_array, reserve_text, settle_slots, slot_bytes, slot_capacity,
};
use crate::builtins::string::clamp_index;
use crate::builtins::{NativeFn, NativeTable, Realm};
use crate::coerce::to_integer_or_infinity;
use crate::host::Host;
use crate::object::{array_element, array_items, array_length, array_pop, array_push, array_shift};
use crate::ops::{same_value_zero, strict_equals, to_boolean, to_js_string, to_number_value};
use crate::unmodelled::throw_or_refuse;
use crate::value::{Eval, IntrinsicProto, JsString, ObjKind, ObjRef, PropKey, Utf16Builder, Value};

pub fn array_ref<'a>(host: &mut dyn Host<'a>, this: &Value<'a>) -> Eval<'a, ObjRef<'a>> {
    let Some(obj) = this.as_object() else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::TypeError,
            "not an array",
            &"a generic receiver the evaluator does not model",
        ));
    };
    let is_dense = matches!(obj.borrow().kind, ObjKind::Array(_) | ObjKind::Arguments(_));
    if !is_dense {
        return Err(throw_or_refuse(
            host,
            ErrorKind::TypeError,
            "not an array",
            &"a generic receiver the evaluator does not model",
        ));
    }
    Ok(obj.clone())
}

fn items<'a>(host: &mut dyn Host<'a>, this: &Value<'a>) -> Eval<'a, Vec<Value<'a>>> {
    let obj = array_ref(host, this)?;
    Ok(array_items(&obj).unwrap_or_default())
}

fn precharge_slots<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    projected: usize,
) -> EvalResult<usize> {
    let held = slot_capacity(obj);
    grow_object(host, obj, slot_bytes(projected.saturating_sub(held)))?;
    Ok(held.max(projected))
}

fn replace_items<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    next: Vec<Value<'a>>,
    charged: usize,
) -> EvalResult<()> {
    let after = next.capacity();
    let previous = {
        let mut borrowed = obj.borrow_mut();
        match &mut borrowed.kind {
            ObjKind::Array(slot) | ObjKind::Arguments(slot) => Some(std::mem::replace(slot, next)),
            _ => None,
        }
    };
    drop(previous);
    settle_slots(host, obj, charged, after)
}

pub fn install<'a>(realm: &mut Realm<'a>, table: &mut NativeTable<'a>) {
    let members: [(&str, NativeFn<'a>); 15] = [
        ("push", native_push),
        ("pop", native_pop),
        ("shift", native_shift),
        ("unshift", native_unshift),
        ("slice", native_slice),
        ("splice", native_splice),
        ("join", native_join),
        ("concat", native_concat),
        ("indexOf", native_index_of),
        ("includes", native_includes),
        ("reverse", native_reverse),
        ("map", native_map),
        ("filter", native_filter),
        ("forEach", native_for_each),
        ("reduce", native_reduce),
    ];
    let pure_members = ["slice", "concat", "join", "indexOf", "includes"];
    let locally_pure_members = ["reverse"];
    for (name, function) in members {
        let value = realm.native_value(table, ("Array.prototype", name), function);
        if pure_members.contains(&name) {
            realm.mark_pure_value(&value);
        }
        if locally_pure_members.contains(&name) {
            realm.mark_locally_pure_value(&value);
        }
        realm.define_array_member(name, value);
    }
    let to_string = realm.native_value(
        table,
        ("Array.prototype", "toString"),
        native_array_to_string,
    );
    realm.define_array_member("toString", to_string);
}

fn native_array_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let key = PropKey::Name(JsString::from_str("join"));
    let join = host.get_property(&this, &key)?;
    if join.is_callable() {
        return host.call_value(&join, this, &[]);
    }
    native_join(host, this, args)
}

fn native_push<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    grow_array(host, &obj, args.len())?;
    for value in args {
        array_push(&obj, value.clone())?;
    }
    let len = array_length(&obj).unwrap_or(0);
    Ok(Value::Number(len as f64))
}

fn native_pop<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    Ok(array_pop(&obj).unwrap_or(Value::Undefined))
}

fn native_shift<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    charge_for_length(host, array_length(&obj).unwrap_or(0))?;
    Ok(array_shift(&obj).unwrap_or(Value::Undefined))
}

fn native_unshift<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    let len = array_length(&obj).unwrap_or(0);
    let projected = len.saturating_add(args.len());
    check_array_growth(projected)?;
    host.spend(args.len() as u64)?;
    charge_for_length(host, len)?;
    let charged = precharge_slots(host, &obj, projected)?;
    let mut current = array_items(&obj).unwrap_or_default();
    let mut next: Vec<Value<'a>> = Vec::with_capacity(projected);
    next.extend(args.iter().cloned());
    next.append(&mut current);
    replace_items(host, &obj, next, charged)?;
    Ok(Value::Number(projected as f64))
}

fn bounds<'a>(host: &mut dyn Host<'a>, args: &[Value<'a>], len: usize) -> Eval<'a, (usize, usize)> {
    let raw_start = match args.first() {
        None | Some(Value::Undefined) => 0.0,
        Some(value) => to_number_value(value, host)?,
    };
    let raw_end = match args.get(1) {
        None | Some(Value::Undefined) => len as f64,
        Some(value) => to_number_value(value, host)?,
    };
    let start = clamp_index(raw_start, len);
    let end = clamp_index(raw_end, len);
    Ok((start, end.max(start)))
}

fn native_slice<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_species();
    let current = match this.as_object() {
        Some(obj) if obj.borrow().kind.is_arguments() => arguments_items(obj)?,
        _ => items(host, &this)?,
    };
    let (start, end) = bounds(host, args, current.len())?;
    charge_for_length(host, current.len())?;
    let slice = current.get(start..end).unwrap_or(&[]);
    check_array_growth(slice.len())?;
    let reserved = reserve_array(host, slice.len())?;
    Ok(adopt_array(
        host,
        slice.to_vec(),
        reserved,
        Some(slice.len()),
    )?)
}

const UNMODELLED_ARGUMENTS: &str =
    "a slice of an arguments object with own keys, a hole or an undefined element, or under state tracing";

fn arguments_items<'a>(obj: &ObjRef<'a>) -> EvalResult<Vec<Value<'a>>> {
    let current = array_items(obj).unwrap_or_default();
    let unmodelled = crate::state::state_tracing()
        || !obj.borrow().props.is_empty()
        || (!crate::object::decoding()
            && current
                .iter()
                .any(|value| matches!(value, Value::Undefined)));
    if unmodelled {
        return Err(Bail::Unsupported(&UNMODELLED_ARGUMENTS));
    }
    Ok(current)
}

fn native_splice<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_species();
    let obj = array_ref(host, &this)?;
    let start_raw = match args.first() {
        None | Some(Value::Undefined) => 0.0,
        Some(value) => to_number_value(value, host)?,
    };
    let count_raw = match args.get(1) {
        None => None,
        Some(value) => Some(to_integer_or_infinity(to_number_value(value, host)?)),
    };
    let mut current = array_items(&obj).unwrap_or_default();
    let start = clamp_index(start_raw, current.len());
    let remaining = current.len().saturating_sub(start);
    let count = match count_raw {
        None if args.is_empty() => 0,
        None => remaining,
        Some(raw) => raw.clamp(0.0, remaining as f64) as usize,
    };
    let inserted = args.len().saturating_sub(2);
    let projected = current.len().saturating_sub(count).saturating_add(inserted);
    check_array_growth(projected)?;
    host.spend(inserted as u64)?;
    charge_for_length(host, current.len())?;
    let charged = precharge_slots(host, &obj, projected.max(current.len()))?;
    let reserved = reserve_array(host, count)?;
    let end = start.saturating_add(count).min(current.len());
    let removed: Vec<Value<'a>> = current
        .splice(start..end, args.iter().skip(2).cloned())
        .collect();
    replace_items(host, &obj, current, charged)?;
    Ok(adopt_array(host, removed, reserved, Some(count))?)
}

fn native_join<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let current = items(host, &this)?;
    let separator = match args.first() {
        None | Some(Value::Undefined) => JsString::from_str(","),
        Some(value) => to_js_string(value, host)?,
    };
    charge_for_length(host, current.len())?;
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    for (index, value) in current.iter().enumerate() {
        if index > 0 {
            charge_for_length(host, separator.len())?;
            out.push(&separator)?;
        }
        if value.is_nullish() {
            continue;
        }
        let rendered = to_js_string(value, host)?;
        charge_for_length(host, rendered.len())?;
        out.push(&rendered)?;
    }
    Ok(Value::Str(out.finish()))
}

fn note_marker<'a>(host: &mut dyn Host<'a>, target: &Value<'a>, name: &str) -> Eval<'a, ()> {
    if crate::object::decoding() {
        host.get_property(target, &PropKey::Name(JsString::from_str(name)))?;
    }
    Ok(())
}

fn note_species() {
    for key in ["constructor", "@@species"] {
        crate::object::note_intrinsic(
            IntrinsicProto::Array,
            &PropKey::Name(JsString::from_str(key)),
        );
    }
}

fn spreadable<'v, 'a>(value: &'v Value<'a>) -> Option<&'v ObjRef<'a>> {
    value
        .as_object()
        .filter(|obj| matches!(obj.borrow().kind, ObjKind::Array(_)))
}

fn native_concat<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_marker(host, &this, "@@isConcatSpreadable")?;
    note_species();
    if this.as_object().is_some_and(crate::object::is_arguments) {
        crate::unmodelled::refuse_while_decoding(&"concat on an arguments receiver")?;
    }
    let current = items(host, &this)?;
    let mut projected = current.len();
    for value in args {
        let extra = spreadable(value).and_then(array_length).unwrap_or(1);
        projected = projected.saturating_add(extra);
    }
    check_array_growth(projected)?;
    charge_for_length(host, projected)?;
    let reserved = reserve_array(host, projected)?;
    let mut result = Vec::with_capacity(projected);
    result.extend(current);
    for value in args {
        match spreadable(value).and_then(array_items) {
            Some(more) => result.extend(more),
            None => result.push(value.clone()),
        }
    }
    Ok(adopt_array(host, result, reserved, Some(projected))?)
}

fn search_start<'a>(host: &mut dyn Host<'a>, args: &[Value<'a>], len: usize) -> Eval<'a, usize> {
    let raw = match args.get(1) {
        None => 0.0,
        Some(value) => to_integer_or_infinity(to_number_value(value, host)?),
    };
    if raw >= 0.0 {
        return Ok(raw.min(len as f64) as usize);
    }
    Ok((len as f64 + raw).max(0.0) as usize)
}

fn native_index_of<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let current = items(host, &this)?;
    if current.is_empty() {
        return Ok(Value::Number(-1.0));
    }
    let start = search_start(host, args, current.len())?;
    charge_for_length(host, current.len())?;
    let needle = argument(args, 0);
    if matches!(needle, Value::Undefined) {
        crate::object::refuse_undefined_array_items(&array_ref(host, &this)?)?;
    }
    let found = current
        .iter()
        .skip(start)
        .position(|value| strict_equals(value, &needle));
    Ok(Value::Number(
        found.map_or(-1.0, |index| (start + index) as f64),
    ))
}

fn native_includes<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let current = items(host, &this)?;
    if current.is_empty() {
        return Ok(Value::Bool(false));
    }
    let start = search_start(host, args, current.len())?;
    charge_for_length(host, current.len())?;
    let needle = argument(args, 0);
    Ok(Value::Bool(
        current
            .iter()
            .skip(start)
            .any(|value| same_value_zero(value, &needle)),
    ))
}

fn native_reverse<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    charge_for_length(host, array_length(&obj).unwrap_or(0))?;
    let charged = slot_capacity(&obj);
    let mut current = array_items(&obj).unwrap_or_default();
    current.reverse();
    replace_items(host, &obj, current, charged)?;
    Ok(this)
}

struct Visited<'a> {
    produced: Value<'a>,
    original: Value<'a>,
}

fn iterate<'a>(
    host: &mut dyn Host<'a>,
    this: &Value<'a>,
    args: &[Value<'a>],
    mut visit: impl FnMut(Option<Visited<'a>>) -> Eval<'a, ()>,
) -> Eval<'a, ()> {
    let obj = array_ref(host, this)?;
    let len = array_length(&obj).unwrap_or(0);
    let callback = argument(args, 0);
    if !callback.is_callable() {
        return Err(host.throw(ErrorKind::TypeError, "callback is not a function"));
    }
    let receiver = argument(args, 1);
    for index in 0..len {
        let Some(original) = array_element(&obj, index) else {
            visit(None)?;
            continue;
        };
        if matches!(original, Value::Undefined) {
            crate::object::refuse_possible_hole()?;
        }
        let produced = host.call_value(
            &callback,
            receiver.clone(),
            &[original.clone(), Value::Number(index as f64), this.clone()],
        )?;
        visit(Some(Visited { produced, original }))?;
    }
    Ok(())
}

fn native_map<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_marker(host, &this, "@@species")?;
    note_species();
    let obj = array_ref(host, &this)?;
    let len = array_length(&obj).unwrap_or(0);
    check_array_growth(len)?;
    let reserved = reserve_array(host, len)?;
    let mut out = Vec::with_capacity(len);
    iterate(host, &this, args, |visited| match visited {
        Some(visited) => {
            out.push(visited.produced);
            Ok(())
        }
        None => Err(Bail::Unsupported(&crate::object::HOLE).into()),
    })?;
    Ok(adopt_array(host, out, reserved, Some(len))?)
}

fn native_filter<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_species();
    let obj = array_ref(host, &this)?;
    check_array_growth(array_length(&obj).unwrap_or(0))?;
    let mut out = Vec::new();
    iterate(host, &this, args, |visited| {
        if let Some(visited) = visited {
            if to_boolean(&visited.produced) {
                out.push(visited.original);
            }
        }
        Ok(())
    })?;
    live_array(host, out)
}

fn native_for_each<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    iterate(host, &this, args, |_| Ok(()))?;
    Ok(Value::Undefined)
}

fn native_reduce<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let obj = array_ref(host, &this)?;
    let len = array_length(&obj).unwrap_or(0);
    let callback = argument(args, 0);
    if !callback.is_callable() {
        return Err(host.throw(ErrorKind::TypeError, "callback is not a function"));
    }
    crate::object::refuse_undefined_array_items(&obj)?;
    let mut index = 0;
    let mut accumulator = match args.get(1) {
        Some(seed) => seed.clone(),
        None => {
            let Some(first) = array_element(&obj, 0).filter(|_| len > 0) else {
                return Err(host.throw(ErrorKind::TypeError, "reduce of empty array"));
            };
            index = 1;
            first
        }
    };
    while index < len {
        if let Some(value) = array_element(&obj, index) {
            accumulator = host.call_value(
                &callback,
                Value::Undefined,
                &[
                    accumulator,
                    value,
                    Value::Number(index as f64),
                    this.clone(),
                ],
            )?;
        }
        index += 1;
    }
    Ok(accumulator)
}
