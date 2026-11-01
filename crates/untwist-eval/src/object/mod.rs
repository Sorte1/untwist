use crate::bail::{Bail, EvalResult};
use crate::budget::{props_bytes, ARRAY_LEN_CAP};
use crate::coerce::to_uint32;
use crate::value::{
    Attrs, Closure, JsObject, JsString, MappedArguments, ObjKind, ObjRef, PropKey, PropMap,
    StoredKey, Value,
};

mod attributes;
mod slots;
mod trace;

pub use attributes::{
    define_own_property, freeze, is_frozen, note_prototype_attributes, own_attributes, Descriptor,
    NON_EXTENSIBLE,
};
pub use slots::{reserve_slots, slot_dims, write_growth, Written};

pub(crate) use trace::is_constructor_key;
use trace::{already_walked, note_constructor_root, push_trace_key, ROOT_FLAG};
pub use trace::{
    decoding, is_intrinsic_root, is_registered_intrinsic, lookup_root_address, note_builtin_roots,
    note_intrinsic, note_intrinsic_root, note_intrinsics, note_prototype_links, note_root,
    note_root_touch, recording, register_root_address, reset_root_table, root_name, set_decoding,
    set_recording, set_tracing, take_builtin_roots, take_trace, Consulted, ProtoRead,
    MAX_ROOT_REGISTRATIONS, PROTOTYPE_LINKS, ROOT_TABLE_SLOTS,
};
#[cfg(feature = "test-hooks")]
pub use trace::{trace_cache_owns_its_keys, trace_slot};

pub const PROTO_CHAIN_CAP: usize = 64;

pub(crate) const HOLE: &str = "an array hole, which the evaluator does not model";

const LENGTH_UNITS: [u16; 6] = [0x6C, 0x65, 0x6E, 0x67, 0x74, 0x68];

fn is_last_index(key: &PropKey) -> bool {
    matches!(key, PropKey::Name(name) if name.eq_str("lastIndex"))
}

pub fn is_length_key(key: &PropKey) -> bool {
    is_length(key)
}

fn is_length(key: &PropKey) -> bool {
    match key {
        PropKey::Name(name) => name.units() == LENGTH_UNITS,
        PropKey::Index(_) => false,
    }
}

fn dense<'a, 'k>(kind: &'k ObjKind<'a>) -> Option<&'k [Value<'a>]> {
    match kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => Some(items),
        _ => None,
    }
}

fn dense_mut<'a, 'k>(kind: &'k mut ObjKind<'a>) -> Option<&'k mut Vec<Value<'a>>> {
    match kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => Some(items),
        _ => None,
    }
}

fn own_property_of<'a>(borrowed: &JsObject<'a>, key: &PropKey) -> Option<Value<'a>> {
    if let Some(items) = dense(&borrowed.kind) {
        if let PropKey::Index(index) = key {
            if let Some(value) = items.get(*index as usize) {
                return Some(value.clone());
            }
        }
        if is_length(key) {
            return Some(Value::Number(items.len() as f64));
        }
    }
    if let ObjKind::MappedArguments(mapped) = &borrowed.kind {
        return mapped_own_property(mapped, &borrowed.props, key);
    }
    borrowed.props.get(key).cloned()
}

#[cold]
#[inline(never)]
fn mapped_own_property<'a>(
    mapped: &MappedArguments<'a>,
    props: &PropMap<'a>,
    key: &PropKey,
) -> Option<Value<'a>> {
    if let PropKey::Index(index) = key {
        if let Some(value) = mapped.value_at(*index as usize) {
            return Some(value);
        }
    }
    if is_length(key) {
        return Some(Value::Number(mapped.items.len() as f64));
    }
    props.get(key).cloned()
}

fn arguments_or_dense_len(kind: &ObjKind<'_>) -> Option<usize> {
    match kind {
        ObjKind::MappedArguments(mapped) => Some(mapped.items.len()),
        other => dense(other).map(<[Value<'_>]>::len),
    }
}

pub(crate) const OBJECT_PROTOTYPE_KEYS: [&str; 12] = [
    "constructor",
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

const PROTOTYPE_UNITS: [u16; 9] = [0x70, 0x72, 0x6F, 0x74, 0x6F, 0x74, 0x79, 0x70, 0x65];

pub fn is_prototype_key(key: &PropKey) -> bool {
    matches!(key, PropKey::Name(name) if name.units() == PROTOTYPE_UNITS)
}

#[cold]
#[inline(never)]
pub(crate) fn has_pending_prototype(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    let borrowed = obj.borrow();
    matches!(&borrowed.kind, ObjKind::Function(closure) if closure.plain)
        && is_prototype_key(key)
        && !borrowed.props.contains_key(key)
}

pub fn is_proto_key(key: &PropKey) -> bool {
    matches!(key, PropKey::Name(name) if name.eq_str("__proto__"))
}

#[inline]
pub(crate) fn refuse_proto_write(key: &PropKey) -> EvalResult<()> {
    if is_proto_key(key) {
        return Err(Bail::Unsupported(&"an assignment to a __proto__ key"));
    }
    Ok(())
}

pub(crate) fn has_own_property(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    own_property_of(&obj.borrow(), key).is_some()
}

pub fn is_arguments(obj: &ObjRef<'_>) -> bool {
    obj.borrow().kind.is_arguments()
}

pub fn arguments_miss_differs(key: &PropKey, inherited: Option<&Value<'_>>) -> bool {
    if inherited.is_some_and(|value| !matches!(value, Value::Undefined)) {
        return true;
    }
    let PropKey::Name(name) = key else {
        return false;
    };
    name.units()
        .starts_with(&[u16::from(b'@'), u16::from(b'@')])
        || name.eq_str("callee")
        || OBJECT_PROTOTYPE_KEYS.iter().any(|known| name.eq_str(known))
}

#[cold]
#[inline(never)]
pub(crate) fn refuse_possible_hole() -> EvalResult<()> {
    if decoding() {
        return Ok(());
    }
    Err(Bail::Unsupported(&HOLE))
}

pub(crate) fn refuse_undefined_array_items(obj: &ObjRef<'_>) -> EvalResult<()> {
    let undefined = match &obj.borrow().kind {
        ObjKind::Array(items) | ObjKind::Arguments(items) => {
            items.iter().any(|value| matches!(value, Value::Undefined))
        }
        ObjKind::MappedArguments(mapped) => (0..mapped.items.len())
            .any(|index| matches!(mapped.value_at(index), Some(Value::Undefined))),
        _ => false,
    };
    if undefined {
        return refuse_possible_hole();
    }
    Ok(())
}

pub(crate) fn refuse_undefined_dense_index(
    obj: &ObjRef<'_>,
    key: &PropKey,
    own: Option<&Value<'_>>,
) -> EvalResult<()> {
    if matches!(own, Some(Value::Undefined)) && is_dense_index(obj, key) {
        return refuse_possible_hole();
    }
    Ok(())
}

pub fn is_dense_index(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    let PropKey::Index(index) = key else {
        return false;
    };
    arguments_or_dense_len(&obj.borrow().kind).is_some_and(|len| (*index as usize) < len)
}

pub fn has_own_keys(obj: &ObjRef<'_>) -> bool {
    let borrowed = obj.borrow();
    !borrowed.props.is_empty() || arguments_or_dense_len(&borrowed.kind).is_some_and(|len| len > 0)
}

pub fn has_enumerable_own_key(obj: &ObjRef<'_>) -> bool {
    obj.borrow()
        .props
        .keys()
        .any(|stored| stored.attrs().enumerable())
}

pub fn own_property<'a>(obj: &ObjRef<'a>, key: &PropKey) -> Option<Value<'a>> {
    own_property_of(&obj.borrow(), key)
}

pub fn walk_prototype_chain<'a, T>(
    start: ObjRef<'a>,
    mut visit: impl FnMut(&ObjRef<'a>) -> Option<T>,
) -> EvalResult<Option<T>> {
    let mut current = start;
    for _ in 0..PROTO_CHAIN_CAP {
        if let Some(found) = visit(&current) {
            return Ok(Some(found));
        }
        let parent = current.borrow().proto.clone();
        match parent {
            Some(next) => current = next,
            None => return Ok(None),
        }
    }
    Err(Bail::ChainTooDeep(&"prototype"))
}

fn chain_lookup<'a>(
    start: ObjRef<'a>,
    inherited: bool,
    key: &PropKey,
) -> EvalResult<Option<Value<'a>>> {
    let tracing = recording();
    let watched = crate::state::state_tracing();
    let mut first = true;
    walk_prototype_chain(start, |link| {
        let borrowed = link.borrow();
        let tag = borrowed.trace_tag();
        if tracing && (!first || inherited) && tag != 0 && tag < ROOT_FLAG {
            push_trace_key(tag, key);
            if is_constructor_key(key) {
                note_constructor_root(tag);
            }
        }
        if !watched || !first {
            if watched {
                let is_arguments = borrowed.kind.is_arguments();
                crate::state::note_inherited_link(link, tag, is_arguments);
            }
            first = false;
            return own_property_of(&borrowed, key);
        }
        first = false;
        let hit = own_property_of(&borrowed, key);
        if let Some(value) = &hit {
            let is_arguments = borrowed.kind.is_arguments();
            crate::state::note_read(link, tag, is_arguments, key, value);
        }
        hit
    })
}

pub fn lookup_property<'a>(obj: &ObjRef<'a>, key: &PropKey) -> EvalResult<Option<Value<'a>>> {
    chain_lookup(obj.clone(), false, key)
}

pub fn lookup_inherited<'a>(start: &ObjRef<'a>, key: &PropKey) -> EvalResult<Option<Value<'a>>> {
    chain_lookup(start.clone(), true, key)
}

pub fn with_array_items<'a, R>(
    obj: &ObjRef<'a>,
    body: impl FnOnce(&[Value<'a>]) -> R,
) -> Option<R> {
    let borrowed = obj.borrow();
    if let Some(items) = dense(&borrowed.kind) {
        return Some(body(items));
    }
    let current = mapped_items(&borrowed.kind)?;
    drop(borrowed);
    Some(body(&current))
}

#[cold]
#[inline(never)]
fn mapped_items<'a>(kind: &ObjKind<'a>) -> Option<Vec<Value<'a>>> {
    match kind {
        ObjKind::MappedArguments(mapped) => Some(mapped.current_items()),
        _ => None,
    }
}

#[cold]
#[inline(never)]
fn mapped_element<'a>(kind: &ObjKind<'a>, index: usize) -> Option<Value<'a>> {
    match kind {
        ObjKind::MappedArguments(mapped) => mapped.value_at(index),
        _ => None,
    }
}

#[cold]
#[inline(never)]
pub fn array_items<'a>(obj: &ObjRef<'a>) -> Option<Vec<Value<'a>>> {
    with_array_items(obj, <[Value<'a>]>::to_vec)
}

pub fn array_element<'a>(obj: &ObjRef<'a>, index: usize) -> Option<Value<'a>> {
    let borrowed = obj.borrow();
    match dense(&borrowed.kind) {
        Some(items) => items.get(index).cloned(),
        None => mapped_element(&borrowed.kind, index),
    }
}

pub fn array_length(obj: &ObjRef<'_>) -> Option<usize> {
    arguments_or_dense_len(&obj.borrow().kind)
}

pub fn string_property<'a>(text: &JsString, key: &PropKey) -> Option<Value<'a>> {
    if is_length(key) {
        return Some(Value::Number(text.len() as f64));
    }
    match key {
        PropKey::Index(index) => text
            .code_unit_at(*index as usize)
            .map(|unit| Value::Str(JsString::from_units(vec![unit]))),
        PropKey::Name(_) => None,
    }
}

fn set_array_index<'a>(
    obj: &ObjRef<'a>,
    tracing: bool,
    watch: (u8, crate::state::ObjClass),
    index: u32,
    value: Value<'a>,
) -> EvalResult<usize> {
    let index_usize = index as usize;
    if index_usize >= ARRAY_LEN_CAP {
        return Err(Bail::ArrayLength);
    }
    if tracing
        && arguments_or_dense_len(&obj.borrow().kind).is_some_and(|len| len <= index_usize)
        && !already_walked(obj, &PropKey::Index(index))
    {
        let _ = lookup_property(obj, &PropKey::Index(index));
    }
    crate::state::note_write(obj, watch.0, watch.1, &PropKey::Index(index), &value);
    let mut borrowed = obj.borrow_mut();
    let (items, arguments) = match &mut borrowed.kind {
        ObjKind::Array(items) => (items, false),
        ObjKind::Arguments(items) => (items, true),
        ObjKind::MappedArguments(mapped) => {
            if index_usize >= mapped.items.len() {
                return Err(Bail::Unsupported(growth_refusal(tracing)));
            }
            mapped.write_at(index_usize, value);
            return Ok(0);
        }
        _ => return Ok(0),
    };
    let len = items.len();
    if len > index_usize {
        items[index_usize] = value;
        return Ok(0);
    }
    if arguments || (tracing && index_usize > len) {
        return Err(Bail::Unsupported(growth_refusal(tracing)));
    }
    items.resize(index_usize + 1, Value::Undefined);
    items[index_usize] = value;
    Ok(index_usize + 1 - len)
}

const ARGUMENTS_GROWTH: &str =
    "an arguments index write past its length, which the evaluator does not model";

fn growth_refusal(tracing: bool) -> &'static &'static str {
    match tracing {
        true => &HOLE,
        false => &ARGUMENTS_GROWTH,
    }
}

fn refuse_length_write(obj: &ObjRef<'_>, value: &Value<'_>) -> EvalResult<()> {
    let modelled = match (&obj.borrow().kind, value) {
        (ObjKind::Array(items), Value::Number(requested)) => {
            f64::from(to_uint32(*requested)) == *requested && (*requested as usize) <= items.len()
        }
        _ => false,
    };
    if modelled {
        return Ok(());
    }
    Err(Bail::Unsupported(&HOLE))
}

fn set_array_length<'a>(obj: &ObjRef<'a>, value: Value<'a>) -> EvalResult<Written> {
    if decoding() || is_arguments(obj) {
        refuse_length_write(obj, &value)?;
    }
    if matches!(obj.borrow().kind, ObjKind::MappedArguments(_)) {
        return Err(Bail::Unsupported(
            &"a length write on an arguments object whose parameters it aliases",
        ));
    }
    let Value::Number(requested) = value else {
        return Ok(Written::slots(0));
    };
    let target = to_uint32(requested) as usize;
    if target > ARRAY_LEN_CAP {
        return Err(Bail::ArrayLength);
    }
    let mut resized = 0;
    let mut borrowed = obj.borrow_mut();
    if let Some(items) = dense_mut(&mut borrowed.kind) {
        resized = target.abs_diff(items.len());
        items.resize(target, Value::Undefined);
    }
    Ok(Written::slots(resized))
}

pub fn set_property<'a>(obj: &ObjRef<'a>, key: PropKey, value: Value<'a>) -> EvalResult<Written> {
    let tracing = decoding();
    let initial = obj.borrow();
    let is_dense = arguments_or_dense_len(&initial.kind).is_some();
    let tag = initial.trace_tag();
    let class = crate::state::class_of(&initial.kind);
    let regexp = matches!(initial.kind, ObjKind::RegExp(_));
    let ordinary = matches!(initial.kind, ObjKind::Plain(0) | ObjKind::Array(_));
    drop(initial);
    if tracing && regexp && is_last_index(&key) {
        return Err(Bail::Unsupported(
            &"a lastIndex write, which the evaluator keeps internal",
        ));
    }
    if is_dense {
        match key {
            PropKey::Index(index) => {
                return set_array_index(obj, tracing, (tag, class), index, value)
                    .map(Written::slots)
            }
            _ if is_length(&key) => {
                if tracing && !ordinary {
                    crate::unmodelled::refuse_unmodelled_write(obj, &key)?;
                }
                return set_array_length(obj, value);
            }
            _ => {}
        }
    }
    if !tracing {
        if !ordinary {
            crate::unmodelled::refuse_callable_name_or_length(obj, &key)?;
            crate::unmodelled::refuse_intrinsic_index_write(obj, &key)?;
        }
        crate::state::note_write(obj, tag, class, &key, &value);
        return attributes::set_honouring_attributes(obj, key, value);
    }
    if !ordinary {
        crate::unmodelled::refuse_unmodelled_write(obj, &key)?;
    }
    if !obj.borrow().props.contains_key(&key) && !already_walked(obj, &key) {
        let _ = lookup_property(obj, &key);
    }
    crate::state::note_write(obj, tag, class, &key, &value);
    let mut borrowed = obj.borrow_mut();
    let before = borrowed.props.capacity();
    let JsObject { props, kind, .. } = &mut *borrowed;
    match props.get_mut(&key) {
        Some(slot) => *slot = value,
        None => insert_new(props, kind, key, value),
    }
    let after = borrowed.props.capacity();
    drop(borrowed);
    Ok(Written {
        resized: 0,
        prop_bytes: props_bytes(before, after),
    })
}

#[inline(always)]
pub(super) fn insert_new<'a>(
    props: &mut PropMap<'a>,
    kind: &ObjKind<'a>,
    key: PropKey,
    value: Value<'a>,
) {
    let stored = match kind {
        ObjKind::Function(closure) => function_key(closure, key),
        _ => StoredKey::from(key),
    };
    props.insert(stored, value);
}

#[cold]
#[inline(never)]
fn function_key(closure: &Closure<'_>, key: PropKey) -> StoredKey {
    if closure.plain && is_prototype_key(&key) {
        return StoredKey::new(key, Attrs::PROTOTYPE);
    }
    StoredKey::from(key)
}

pub fn array_push<'a>(obj: &ObjRef<'a>, value: Value<'a>) -> EvalResult<Option<usize>> {
    let mut borrowed = obj.borrow_mut();
    let Some(items) = dense_mut(&mut borrowed.kind) else {
        return Ok(None);
    };
    if items.len() >= ARRAY_LEN_CAP {
        return Err(Bail::ArrayLength);
    }
    items.push(value);
    Ok(Some(items.len()))
}

pub fn array_pop<'a>(obj: &ObjRef<'a>) -> Option<Value<'a>> {
    let mut borrowed = obj.borrow_mut();
    let items = dense_mut(&mut borrowed.kind)?;
    Some(items.pop().unwrap_or(Value::Undefined))
}

pub fn array_shift<'a>(obj: &ObjRef<'a>) -> Option<Value<'a>> {
    crate::state::note_rotation_step();
    let mut borrowed = obj.borrow_mut();
    let items = dense_mut(&mut borrowed.kind)?;
    if items.is_empty() {
        return Some(Value::Undefined);
    }
    Some(items.remove(0))
}

pub fn array_rotate_left(obj: &ObjRef<'_>, steps: usize) -> Option<()> {
    let mut borrowed = obj.borrow_mut();
    let items = dense_mut(&mut borrowed.kind)?;
    let len = items.len();
    if len > 0 {
        items.rotate_left(steps % len);
    }
    Some(())
}
