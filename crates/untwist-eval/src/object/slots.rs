use crate::budget::ARRAY_LEN_CAP;
use crate::coerce::to_uint32;
use crate::value::{ObjKind, ObjRef, PropKey, Value};

use super::is_length;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    pub resized: usize,
    pub prop_bytes: u64,
}

impl Written {
    pub(super) fn slots(resized: usize) -> Self {
        Self {
            resized,
            prop_bytes: 0,
        }
    }
}

pub fn slot_dims(obj: &ObjRef<'_>) -> Option<(usize, usize)> {
    match &obj.borrow().kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => Some((items.len(), items.capacity())),
        ObjKind::MappedArguments(mapped) => Some((mapped.items.len(), mapped.items.capacity())),
        _ => None,
    }
}

pub fn reserve_slots(obj: &ObjRef<'_>, additional: usize) {
    match &mut obj.borrow_mut().kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => items.reserve(additional),
        ObjKind::MappedArguments(mapped) => mapped.items.reserve(additional),
        _ => {}
    }
}

pub fn write_growth(obj: &ObjRef<'_>, key: &PropKey, value: &Value<'_>) -> usize {
    let target = match (key, value) {
        (PropKey::Index(index), _) => (*index as usize).saturating_add(1),
        (PropKey::Name(_), Value::Number(requested)) if is_length(key) => {
            to_uint32(*requested) as usize
        }
        _ => return 0,
    };
    if target > ARRAY_LEN_CAP {
        return 0;
    }
    slot_dims(obj).map_or(0, |(len, _)| target.saturating_sub(len))
}
