use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustc_hash::FxHashSet;

use crate::value::{IntrinsicProto, JsString, ObjRef, PropKey, ReservedString, Value};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Consulted {
    Key(PropKey),
    AnyIndex,
    Any,
}

pub type ProtoRead = (IntrinsicProto, Consulted);

pub(super) const ROOT_FLAG: u8 = 0x80;

const CACHE_SLOTS: usize = 64;
const CACHE_SHIFT: u32 = usize::BITS - 6;
#[cfg(target_pointer_width = "64")]
const CACHE_GOLDEN: usize = 0x9E37_79B9_7F4A_7C15;
#[cfg(not(target_pointer_width = "64"))]
const CACHE_GOLDEN: usize = 0x9E37_79B9;

fn cache_slot(a: usize, b: usize) -> usize {
    (a ^ b).wrapping_mul(CACHE_GOLDEN) >> CACHE_SHIFT
}

fn key_identity(what: &Consulted) -> usize {
    match what {
        Consulted::Key(PropKey::Name(name)) => name.identity(),
        Consulted::Key(PropKey::Index(index)) => ((*index as usize) << 1) | 1,
        Consulted::AnyIndex => usize::MAX,
        Consulted::Any => usize::MAX - 1,
    }
}

fn units_seed(units: &[u16]) -> usize {
    let first = units.first().copied().unwrap_or(0) as usize;
    let middle = units.get(units.len() / 2).copied().unwrap_or(0) as usize;
    let last = units.last().copied().unwrap_or(0) as usize;
    units.len().rotate_left(16) ^ first ^ middle.rotate_left(5) ^ last.rotate_left(11)
}

fn key_seed(key: &PropKey) -> usize {
    match key {
        PropKey::Name(name) => units_seed(name.units()),
        PropKey::Index(_) => usize::MAX,
    }
}

fn marker_seed(what: &Consulted) -> usize {
    match what {
        Consulted::Key(PropKey::Name(name)) => units_seed(name.units()),
        other => key_identity(other),
    }
}

fn consulted_for(key: &PropKey) -> Consulted {
    match key {
        PropKey::Index(_) => Consulted::AnyIndex,
        PropKey::Name(name) => Consulted::Key(PropKey::Name(name.clone())),
    }
}

fn key_identity_of(key: &PropKey) -> usize {
    match key {
        PropKey::Name(name) => name.identity(),
        PropKey::Index(_) => usize::MAX,
    }
}

fn hash_units(units: &[u16]) -> u64 {
    use std::hash::Hasher;
    let mut hasher = rustc_hash::FxHasher::default();
    for unit in units {
        hasher.write_u16(*unit);
    }
    hasher.finish()
}

fn key_hash(key: &PropKey) -> u64 {
    match key {
        PropKey::Name(name) => hash_units(name.units()),
        PropKey::Index(_) => 0,
    }
}

#[derive(Clone)]
struct TraceEntry {
    tag: u8,
    ptr: usize,
    hash: u64,
    held: Option<ReservedString>,
}

fn reserved(key: &PropKey) -> Option<ReservedString> {
    match key {
        PropKey::Name(name) => Some(name.reserve()),
        PropKey::Index(_) => None,
    }
}

fn reserved_matches(held: Option<&ReservedString>, key: &PropKey) -> bool {
    match (held, key) {
        (Some(held), PropKey::Name(name)) => held.holds_units(name.units()),
        _ => false,
    }
}

#[derive(Clone)]
struct WalkedEntry {
    proto_addr: usize,
    ptr: usize,
    hash: u64,
    held: Option<ReservedString>,
}

const CONSTRUCTOR: &[u8] = b"constructor";

pub(crate) fn is_constructor_key(key: &PropKey) -> bool {
    match key {
        PropKey::Name(name) => {
            let units = name.units();
            units.len() == CONSTRUCTOR.len()
                && units.iter().zip(CONSTRUCTOR).all(|(u, b)| *u == *b as u16)
        }
        PropKey::Index(_) => false,
    }
}

fn intrinsic_slot(tag: IntrinsicProto) -> usize {
    match tag {
        IntrinsicProto::Object => 0,
        IntrinsicProto::Array => 1,
        IntrinsicProto::Function => 2,
        IntrinsicProto::String => 3,
        IntrinsicProto::Number => 4,
        IntrinsicProto::RegExp => 5,
    }
}

thread_local! {
    static DECODING: Cell<bool> = const { Cell::new(false) };
    static RECORDING: Cell<bool> = const { Cell::new(false) };
    static TRACE: RefCell<Vec<(u8, Consulted)>> = const { RefCell::new(Vec::new()) };
    static TRACE_SEEN: RefCell<FxHashSet<(u8, Consulted)>> = RefCell::new(FxHashSet::default());
    static TRACE_CACHE: RefCell<[Option<TraceEntry>; CACHE_SLOTS]> =
        RefCell::new(std::array::from_fn(|_| None));
    static ROOT_NAMES: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static ROOT_HITS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static ROOT_SEEN: RefCell<FxHashSet<&'static str>> = RefCell::new(FxHashSet::default());
    static ROOT_CACHE: RefCell<[usize; CACHE_SLOTS]> = const { RefCell::new([0usize; CACHE_SLOTS]) };
    static INTRINSIC_ROOTS: RefCell<[Option<u8>; 6]> = const { RefCell::new([None; 6]) };
    static WALKED_CACHE: RefCell<[Option<WalkedEntry>; CACHE_SLOTS]> =
        RefCell::new(std::array::from_fn(|_| None));
    static ROOT_TABLE: RefCell<[Option<(usize, u8)>; ROOT_TABLE_SLOTS]> =
        const { RefCell::new([None; ROOT_TABLE_SLOTS]) };
}

pub const ROOT_TABLE_SLOTS: usize = 256;
const ROOT_TABLE_MASK: usize = ROOT_TABLE_SLOTS - 1;
const ROOT_TABLE_SHIFT: u32 = usize::BITS - 8;

pub const MAX_ROOT_REGISTRATIONS: usize = 0x7f;

fn root_table_slot(address: usize) -> usize {
    (address.wrapping_mul(CACHE_GOLDEN) >> ROOT_TABLE_SHIFT) & ROOT_TABLE_MASK
}

pub fn reset_root_table() {
    ROOT_TABLE.with(|table| *table.borrow_mut() = [None; ROOT_TABLE_SLOTS]);
}

pub fn register_root_address(address: usize, root_index: u8) {
    ROOT_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let start = root_table_slot(address);
        for offset in 0..ROOT_TABLE_SLOTS {
            let slot = (start + offset) & ROOT_TABLE_MASK;
            if table[slot].is_none() {
                table[slot] = Some((address, root_index));
                return;
            }
        }
    });
}

pub fn lookup_root_address(address: usize) -> Option<u8> {
    ROOT_TABLE.with(|table| {
        let table = table.borrow();
        let start = root_table_slot(address);
        for offset in 0..ROOT_TABLE_SLOTS {
            let slot = (start + offset) & ROOT_TABLE_MASK;
            match table[slot] {
                Some((addr, index)) if addr == address => return Some(index),
                Some(_) => continue,
                None => return None,
            }
        }
        None
    })
}

#[inline]
pub fn note_root_touch(value: &Value<'_>) {
    if !recording() {
        return;
    }
    let Some(obj) = value.as_object() else {
        return;
    };
    let addr = Rc::as_ptr(obj) as usize;
    if let Some(index) = lookup_root_address(addr) {
        note_root_index(index);
    }
}

pub fn note_intrinsic_root(tag: IntrinsicProto, root_index: u8) {
    INTRINSIC_ROOTS.with(|roots| roots.borrow_mut()[intrinsic_slot(tag)] = Some(root_index));
}

pub fn root_name(obj: &ObjRef<'_>) -> Option<&'static str> {
    let index = lookup_root_address(Rc::as_ptr(obj) as usize)?;
    ROOT_NAMES.with(|names| names.borrow().get(index as usize).copied())
}

pub fn note_builtin_roots(names: Vec<&'static str>) {
    ROOT_NAMES.with(|table| *table.borrow_mut() = names);
}

pub fn decoding() -> bool {
    DECODING.with(Cell::get)
}

pub fn set_decoding(on: bool) {
    DECODING.with(|flag| flag.set(on));
}

pub fn recording() -> bool {
    RECORDING.with(Cell::get)
}

pub fn set_tracing(on: bool) {
    set_decoding(on);
    set_recording(on);
}

pub fn set_recording(on: bool) {
    RECORDING.with(|flag| flag.set(on));
    if on {
        TRACE.with(|trace| trace.borrow_mut().clear());
        TRACE_SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            seen.clear();
            seen.shrink_to(CACHE_SLOTS);
        });
        TRACE_CACHE.with(|cache| *cache.borrow_mut() = std::array::from_fn(|_| None));
        ROOT_HITS.with(|hits| hits.borrow_mut().clear());
        ROOT_SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            seen.clear();
            seen.shrink_to(CACHE_SLOTS);
        });
        ROOT_CACHE.with(|cache| *cache.borrow_mut() = [0usize; CACHE_SLOTS]);
        WALKED_CACHE.with(|cache| *cache.borrow_mut() = std::array::from_fn(|_| None));
    } else {
        release_records();
    }
}

fn release_records() {
    TRACE.with(|trace| drop(std::mem::take(&mut *trace.borrow_mut())));
    TRACE_SEEN.with(|seen| drop(std::mem::take(&mut *seen.borrow_mut())));
    TRACE_CACHE.with(|cache| *cache.borrow_mut() = std::array::from_fn(|_| None));
    ROOT_HITS.with(|hits| drop(std::mem::take(&mut *hits.borrow_mut())));
    ROOT_SEEN.with(|seen| drop(std::mem::take(&mut *seen.borrow_mut())));
    WALKED_CACHE.with(|cache| *cache.borrow_mut() = std::array::from_fn(|_| None));
}

pub(super) fn already_walked(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    let (tag, proto_addr) = {
        let object = obj.borrow();
        let Some(proto) = object.proto.as_ref() else {
            return false;
        };
        let tag = proto.borrow().trace_tag();
        (tag, Rc::as_ptr(proto) as usize)
    };
    if tag == 0 || tag >= ROOT_FLAG {
        return false;
    }
    let ptr = key_identity_of(key);
    let slot = cache_slot(tag as usize, key_seed(key));
    WALKED_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let entry = &mut cache[slot];
        if entry
            .as_ref()
            .is_some_and(|entry| entry.proto_addr == proto_addr && entry.ptr == ptr)
        {
            return true;
        }
        let hash = key_hash(key);
        let content_hit = hash != 0
            && entry.as_ref().is_some_and(|entry| {
                entry.proto_addr == proto_addr
                    && entry.hash == hash
                    && reserved_matches(entry.held.as_ref(), key)
            });
        if !content_hit {
            *entry = Some(WalkedEntry {
                proto_addr,
                ptr,
                hash,
                held: reserved(key),
            });
        }
        content_hit
    })
}

fn note_root_index(index: u8) {
    let name = ROOT_NAMES.with(|names| names.borrow().get(index as usize).copied());
    let Some(name) = name else {
        return;
    };
    let identity = name.as_ptr() as usize;
    let slot = cache_slot(index as usize, 0);
    let hit = ROOT_CACHE.with(|cache| cache.borrow()[slot] == identity);
    if hit {
        return;
    }
    ROOT_CACHE.with(|cache| cache.borrow_mut()[slot] = identity);
    let fresh = ROOT_SEEN.with(|seen| seen.borrow_mut().insert(name));
    if fresh {
        ROOT_HITS.with(|hits| hits.borrow_mut().push(name));
    }
}

pub(super) fn note_constructor_root(tag: u8) {
    let Some(intrinsic) = IntrinsicProto::from_bit(tag) else {
        return;
    };
    let root_index = INTRINSIC_ROOTS.with(|roots| roots.borrow()[intrinsic_slot(intrinsic)]);
    if let Some(root_index) = root_index {
        note_root_index(root_index);
    }
}

pub const PROTOTYPE_LINKS: &str = "@@prototype-links";

pub fn note_prototype_links() {
    if !recording() {
        return;
    }
    let fresh = ROOT_SEEN.with(|seen| seen.borrow_mut().insert(PROTOTYPE_LINKS));
    if fresh {
        ROOT_HITS.with(|hits| hits.borrow_mut().push(PROTOTYPE_LINKS));
    }
}

pub fn take_builtin_roots() -> Vec<&'static str> {
    ROOT_HITS.with(|hits| hits.borrow_mut().drain(..).collect())
}

pub fn take_trace() -> Vec<ProtoRead> {
    TRACE.with(|trace| {
        trace
            .borrow_mut()
            .drain(..)
            .filter_map(|(tag, what)| IntrinsicProto::from_bit(tag).map(|proto| (proto, what)))
            .collect()
    })
}

fn push_trace_if_new(
    tag: u8,
    ptr: usize,
    seed: usize,
    hash: impl FnOnce() -> u64,
    key: Option<&PropKey>,
    build: impl FnOnce() -> Consulted,
) {
    let slot = cache_slot(seed, tag as usize);
    let fresh_entry = TRACE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let entry = &mut cache[slot];
        if entry
            .as_ref()
            .is_some_and(|entry| entry.tag == tag && entry.ptr == ptr)
        {
            return false;
        }
        let hash = hash();
        let content_hit = hash != 0
            && key.is_some_and(|k| {
                entry.as_ref().is_some_and(|entry| {
                    entry.tag == tag
                        && entry.hash == hash
                        && reserved_matches(entry.held.as_ref(), k)
                })
            });
        let held = key.and_then(reserved);
        match entry.as_mut() {
            Some(entry) if content_hit => {
                entry.ptr = ptr;
                entry.held = held;
                false
            }
            _ => {
                *entry = Some(TraceEntry {
                    tag,
                    ptr,
                    hash,
                    held,
                });
                true
            }
        }
    });
    if !fresh_entry {
        return;
    }
    let what = build();
    let fresh = TRACE_SEEN.with(|seen| seen.borrow_mut().insert((tag, what.clone())));
    if fresh {
        TRACE.with(|trace| trace.borrow_mut().push((tag, what)));
    }
}

#[cfg(feature = "test-hooks")]
pub fn trace_cache_owns_its_keys() -> bool {
    TRACE_CACHE.with(|cache| {
        cache.borrow().iter().flatten().all(|entry| {
            entry
                .held
                .as_ref()
                .is_none_or(|held| entry.ptr == held.identity())
        })
    })
}

#[cfg(feature = "test-hooks")]
pub fn trace_slot(tag: IntrinsicProto, key: &PropKey) -> usize {
    cache_slot(key_seed(key), tag.bit() as usize)
}

pub(super) fn push_trace_key(tag: u8, key: &PropKey) {
    let ptr = key_identity_of(key);
    push_trace_if_new(
        tag,
        ptr,
        key_seed(key),
        || key_hash(key),
        Some(key),
        || consulted_for(key),
    );
}

fn push_trace_marker(tag: u8, what: Consulted) {
    let ptr = key_identity(&what);
    let seed = marker_seed(&what);
    push_trace_if_new(tag, ptr, seed, || 0, None, || what);
}

pub fn note_root(target: &ObjRef<'_>, key: &PropKey) {
    let tag = target.borrow().trace_tag();
    if tag != 0 && tag < ROOT_FLAG {
        push_trace_key(tag, key);
    }
}

pub fn is_intrinsic_root(target: &ObjRef<'_>) -> bool {
    recording() && {
        let tag = target.borrow().trace_tag();
        tag != 0 && tag < ROOT_FLAG
    }
}

pub fn note_intrinsic(tag: IntrinsicProto, key: &PropKey) {
    if !recording() {
        return;
    }
    push_trace_key(tag.bit(), key);
}

pub fn note_intrinsic_constructor(tag: IntrinsicProto) {
    if !recording() {
        return;
    }
    let key = PropKey::Name(JsString::from_str("constructor"));
    push_trace_key(tag.bit(), &key);
    note_constructor_root(tag.bit());
}

pub fn note_intrinsics(tags: &[IntrinsicProto], what: Consulted) {
    if !recording() {
        return;
    }
    for tag in tags {
        push_trace_marker(tag.bit(), what.clone());
    }
}

pub fn is_registered_intrinsic(obj: &ObjRef<'_>) -> bool {
    let tag = obj.borrow().trace_tag();
    tag != 0 && tag < ROOT_FLAG
}
