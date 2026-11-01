use std::cell::Cell;

use indexmap::map::MutableKeys;

use crate::bail::{Bail, EvalResult};
use crate::budget::props_bytes;
use crate::value::{Attrs, JsObject, ObjKind, ObjRef, PropKey, Value};

use super::{arguments_or_dense_len, is_length, own_property_of, Written, PROTO_CHAIN_CAP};

pub const NON_EXTENSIBLE: u8 = 0x40;

const ARRAY_LENGTH: Attrs = Attrs::new(true, false, false);
const ARGUMENTS_LENGTH: Attrs = Attrs::new(true, false, true);

thread_local! {
    static READ_ONLY_REACHABLE: Cell<bool> = const { Cell::new(false) };
}

fn mark_read_only_reachable() {
    READ_ONLY_REACHABLE.with(|reachable| reachable.set(true));
}

#[derive(Default)]
pub struct Descriptor<'a> {
    pub value: Option<Value<'a>>,
    pub writable: Option<bool>,
    pub enumerable: Option<bool>,
    pub configurable: Option<bool>,
}

impl<'a> Descriptor<'a> {
    fn merged(&self, current: Attrs) -> Attrs {
        Attrs::new(
            self.writable.unwrap_or(current.writable()),
            self.enumerable.unwrap_or(current.enumerable()),
            self.configurable.unwrap_or(current.configurable()),
        )
    }

    fn admits(&self, current: Attrs, value: &Value<'a>) -> bool {
        if current.configurable() {
            return true;
        }
        if self.configurable == Some(true) {
            return false;
        }
        if self
            .enumerable
            .is_some_and(|enumerable| enumerable != current.enumerable())
        {
            return false;
        }
        if current.writable() {
            return true;
        }
        self.writable != Some(true)
            && self
                .value
                .as_ref()
                .is_none_or(|next| same_value(next, value))
    }
}

#[inline(never)]
pub(super) fn set_honouring_attributes<'a>(
    obj: &ObjRef<'a>,
    key: PropKey,
    value: Value<'a>,
) -> EvalResult<Written> {
    let mut borrowed = obj.borrow_mut();
    let extensible = !matches!(borrowed.kind, ObjKind::Plain(NON_EXTENSIBLE));
    let JsObject {
        props, proto, kind, ..
    } = &mut *borrowed;
    match props.get_full_mut2(&key) {
        Some((_, stored, slot)) if stored.attrs().writable() => {
            *slot = value;
            return Ok(Written::slots(0));
        }
        Some(_) => return Err(Bail::Refused),
        None => {}
    }
    if !extensible || inherits_read_only(proto, &key)? {
        return Err(Bail::Refused);
    }
    let before = props.capacity();
    super::insert_new(props, kind, key, value);
    let after = props.capacity();
    Ok(Written {
        resized: 0,
        prop_bytes: props_bytes(before, after),
    })
}

fn inherits_read_only(proto: &Option<ObjRef<'_>>, key: &PropKey) -> EvalResult<bool> {
    if !READ_ONLY_REACHABLE.with(Cell::get) {
        return Ok(false);
    }
    inherited_read_only(proto.clone(), key)
}

#[cold]
#[inline(never)]
fn inherited_read_only(mut link: Option<ObjRef<'_>>, key: &PropKey) -> EvalResult<bool> {
    for _ in 0..PROTO_CHAIN_CAP {
        let Some(current) = link else {
            return Ok(false);
        };
        let borrowed = current.borrow();
        if let Some((_, stored, _)) = borrowed.props.get_full(key) {
            return Ok(!stored.attrs().writable());
        }
        if own_property_of(&borrowed, key).is_some() {
            return Ok(false);
        }
        link = borrowed.proto.clone();
    }
    Err(Bail::ChainTooDeep(&"prototype"))
}

pub fn note_prototype_attributes(proto: &ObjRef<'_>) {
    if READ_ONLY_REACHABLE.with(Cell::get) {
        return;
    }
    let read_only = proto
        .borrow()
        .props
        .keys()
        .any(|key| !key.attrs().writable());
    if read_only {
        mark_read_only_reachable();
    }
}

pub fn own_attributes(obj: &ObjRef<'_>, key: &PropKey) -> Option<Attrs> {
    let borrowed = obj.borrow();
    if let Some(len) = arguments_or_dense_len(&borrowed.kind) {
        if is_length(key) {
            return Some(match borrowed.kind {
                ObjKind::Array(_) => ARRAY_LENGTH,
                _ => ARGUMENTS_LENGTH,
            });
        }
        if matches!(key, PropKey::Index(index) if (*index as usize) < len) {
            return Some(Attrs::DATA);
        }
    }
    borrowed
        .props
        .get_full(key)
        .map(|(_, stored, _)| stored.attrs())
        .or_else(|| super::has_pending_prototype(obj, key).then_some(Attrs::PROTOTYPE))
}

pub fn define_own_property<'a>(
    obj: &ObjRef<'a>,
    key: PropKey,
    descriptor: Descriptor<'a>,
) -> EvalResult<Written> {
    let mut borrowed = obj.borrow_mut();
    let extensible = !matches!(borrowed.kind, ObjKind::Plain(NON_EXTENSIBLE));
    let props = &mut borrowed.props;
    let before = props.capacity();
    let (index, fresh) = match props.get_index_of(&key) {
        Some(index) => (index, false),
        None if extensible => (props.insert_full(key.into(), Value::Undefined).0, true),
        None => return Err(Bail::Refused),
    };
    let after = props.capacity();
    let Some((stored, value)) = props.get_index_mut2(index) else {
        return Err(Bail::Refused);
    };
    let current = if fresh {
        Attrs::new(false, false, false)
    } else {
        stored.attrs()
    };
    if !fresh && !descriptor.admits(current, value) {
        return Err(Bail::Refused);
    }
    let next = descriptor.merged(current);
    stored.set_attrs(next);
    if let Some(fresh_value) = descriptor.value {
        *value = fresh_value;
    }
    if !next.writable() {
        mark_read_only_reachable();
    }
    Ok(Written {
        resized: 0,
        prop_bytes: props_bytes(before, after),
    })
}

pub fn freeze(obj: &ObjRef<'_>) -> EvalResult<()> {
    let mut borrowed = obj.borrow_mut();
    match borrowed.kind {
        ObjKind::Plain(0) => {}
        ObjKind::Plain(NON_EXTENSIBLE) => return Ok(()),
        _ => {
            return Err(Bail::Unsupported(
                &"Object.freeze of an object other than an ordinary plain one",
            ))
        }
    }
    let mut any = false;
    for (key, _) in borrowed.props.iter_mut2() {
        key.set_attrs(key.attrs().frozen());
        any = true;
    }
    borrowed.kind = ObjKind::Plain(NON_EXTENSIBLE);
    drop(borrowed);
    if any {
        mark_read_only_reachable();
    }
    Ok(())
}

pub fn is_frozen(value: Option<&Value<'_>>) -> bool {
    match value {
        Some(Value::Object(obj)) => matches!(obj.borrow().kind, ObjKind::Plain(NON_EXTENSIBLE)),
        _ => true,
    }
}

fn same_value<'a>(next: &Value<'a>, current: &Value<'a>) -> bool {
    match (next, current) {
        (Value::Number(a), Value::Number(b)) => {
            a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
        }
        _ => crate::ops::strict_equals(next, current),
    }
}
