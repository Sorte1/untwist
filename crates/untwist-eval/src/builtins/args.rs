use crate::bail::{Bail, EvalResult};
use crate::budget::{
    ARRAY_HEADER_BYTES, ARRAY_LEN_CAP, ARRAY_SLOT_BYTES, STRING_HEADER_BYTES, STRING_LEN_CAP,
    STRING_UNIT_BYTES,
};
use crate::heap::Charge;
use crate::host::Host;
use crate::object::{reserve_slots, set_property, slot_dims, write_growth};
use crate::value::{Eval, JsString, ObjRef, PropKey, Value};

const MIN_SLOT_CAPACITY: usize = 4;

pub(crate) fn argument<'a>(args: &[Value<'a>], index: usize) -> Value<'a> {
    args.get(index).cloned().unwrap_or(Value::Undefined)
}

pub(crate) fn check_array_growth<'a>(projected: usize) -> Eval<'a, ()> {
    if projected > ARRAY_LEN_CAP {
        return Err(Bail::ArrayLength.into());
    }
    Ok(())
}

pub(crate) fn check_string_cap<'a>(projected: usize) -> Eval<'a, ()> {
    if projected > STRING_LEN_CAP {
        return Err(Bail::StringLength.into());
    }
    Ok(())
}

pub(crate) fn check_string_growth<'a>(
    host: &mut dyn Host<'a>,
    projected: usize,
) -> Eval<'a, Option<Charge>> {
    check_string_cap(projected)?;
    Ok(reserve_text(host, projected)?)
}

pub(crate) fn string_bytes(units: usize) -> u64 {
    (units as u64).saturating_mul(STRING_UNIT_BYTES)
}

pub(crate) fn text_bytes(units: usize) -> u64 {
    STRING_HEADER_BYTES.saturating_add(string_bytes(units))
}

pub(crate) fn slot_bytes(slots: usize) -> u64 {
    (slots as u64).saturating_mul(ARRAY_SLOT_BYTES)
}

pub(crate) fn reserve_text<'a>(
    host: &mut dyn Host<'a>,
    units: usize,
) -> EvalResult<Option<Charge>> {
    host.reserve_live(text_bytes(units))
}

pub(crate) fn fitted(units: Vec<u16>, charge: Option<Charge>) -> JsString {
    if let Some(charge) = &charge {
        charge.shrink_to(text_bytes(units.len()));
    }
    JsString::charged(units, charge)
}

pub(crate) fn live_text<'a>(host: &mut dyn Host<'a>, units: Vec<u16>) -> EvalResult<JsString> {
    if let [unit] = units.as_slice() {
        return host.unit_string(*unit);
    }
    let charge = reserve_text(host, units.len())?;
    Ok(JsString::charged(units, charge))
}

pub(crate) fn live_slice<'a>(
    host: &mut dyn Host<'a>,
    units: &[u16],
    start: usize,
    end: usize,
) -> EvalResult<JsString> {
    let piece = units.get(start..end.max(start)).unwrap_or(&[]);
    if let [unit] = piece {
        return host.unit_string(*unit);
    }
    live_text(host, piece.to_vec())
}

pub(crate) fn reserve_array<'a>(
    host: &mut dyn Host<'a>,
    slots: usize,
) -> EvalResult<Option<Charge>> {
    host.reserve_live(ARRAY_HEADER_BYTES.saturating_add(slot_bytes(slots)))
}

pub(crate) fn live_array<'a>(
    host: &mut dyn Host<'a>,
    items: Vec<Value<'a>>,
) -> Eval<'a, Value<'a>> {
    let reserved = reserve_array(host, items.capacity())?;
    Ok(adopt_array(host, items, reserved, None)?)
}

pub(crate) fn adopt_array<'a>(
    host: &mut dyn Host<'a>,
    items: Vec<Value<'a>>,
    reserved: Option<Charge>,
    reserved_slots: Option<usize>,
) -> EvalResult<Value<'a>> {
    let capacity = items.capacity();
    let array = host.new_array(items);
    let Some(obj) = array.as_object() else {
        return Ok(array);
    };
    if let Some(reserved) = reserved {
        absorb(obj, reserved);
    }
    if let Some(slots) = reserved_slots {
        settle_slots(host, obj, slots, capacity)?;
    }
    Ok(array)
}

fn absorb(obj: &ObjRef<'_>, reserved: Charge) {
    let mut borrowed = obj.borrow_mut();
    match borrowed.charge.as_ref() {
        Some(charge) => charge.absorb(reserved),
        None => borrowed.charge = Some(Box::new(reserved)),
    }
}

pub(crate) fn adopt_charge(value: &Value<'_>, charge: Option<Charge>) {
    if let (Some(obj), Some(charge)) = (value.as_object(), charge) {
        absorb(obj, charge);
    }
}

pub(crate) fn grow_object<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    bytes: u64,
) -> EvalResult<()> {
    if bytes == 0 {
        return Ok(());
    }
    if let Some(charge) = obj.borrow().charge.as_ref() {
        return charge.grow(bytes);
    }
    let charge = host.reserve_live(bytes)?;
    obj.borrow_mut().charge = charge.map(Box::new);
    Ok(())
}

pub(crate) fn shrink_object(obj: &ObjRef<'_>, bytes: u64) {
    if let Some(charge) = obj.borrow().charge.as_ref() {
        charge.shrink(bytes);
    }
}

pub(crate) fn settle_slots<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    before: usize,
    after: usize,
) -> EvalResult<()> {
    if after >= before {
        return grow_object(host, obj, slot_bytes(after - before));
    }
    shrink_object(obj, slot_bytes(before - after));
    Ok(())
}

pub(crate) fn slot_capacity(obj: &ObjRef<'_>) -> usize {
    slot_dims(obj).map_or(0, |(_, capacity)| capacity)
}

pub(crate) fn grow_array<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    extra: usize,
) -> EvalResult<()> {
    if extra == 0 || !host.charges_heap() {
        return Ok(());
    }
    let Some((len, capacity)) = slot_dims(obj) else {
        return Ok(());
    };
    let needed = len.saturating_add(extra);
    if needed <= capacity {
        return Ok(());
    }
    let target = needed
        .max(capacity.saturating_mul(2))
        .max(MIN_SLOT_CAPACITY);
    grow_object(host, obj, slot_bytes(target - capacity))?;
    reserve_slots(obj, target - len);
    let reached = slot_capacity(obj);
    settle_slots(host, obj, target, reached)
}

pub(crate) fn write_property<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    key: PropKey,
    value: Value<'a>,
) -> EvalResult<()> {
    grow_array(host, obj, write_growth(obj, &key, &value))?;
    let written = set_property(obj, key, value)?;
    host.spend(written.resized as u64 / LENGTH_CHARGE_DIVISOR)?;
    grow_object(host, obj, written.prop_bytes)
}

const LENGTH_CHARGE_DIVISOR: u64 = 64;
const HEAP_CHECK_EVERY: usize = 1024;

#[inline]
pub(crate) fn heap_checkpoint<'a>(host: &mut dyn Host<'a>, done: usize) -> EvalResult<()> {
    if done % HEAP_CHECK_EVERY == HEAP_CHECK_EVERY - 1 {
        return host.check_heap();
    }
    Ok(())
}

pub(crate) fn charge_for_length<'a>(host: &mut dyn Host<'a>, len: usize) -> EvalResult<()> {
    host.spend(1 + (len as u64) / LENGTH_CHARGE_DIVISOR)
}

pub(crate) fn charge_for_scan<'a>(
    host: &mut dyn Host<'a>,
    haystack: usize,
    needle: usize,
) -> EvalResult<()> {
    let work = (haystack as u64).saturating_mul(needle.max(1) as u64);
    host.spend(1 + work / LENGTH_CHARGE_DIVISOR)
}
