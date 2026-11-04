use std::cell::{Cell, RefCell};
use std::marker::PhantomData;

use oxc_span::Span;
use rustc_hash::FxHashMap;

use crate::coerce::to_number_units;
use crate::value::{JsObject, JsString, ObjKind, ObjRef, PropKey, Value};

use super::registry::{self_owner, sub_owner};
use super::{Shape, UNPROVEN_GENERIC};

pub(super) const MONITOR_MARK: u8 = 0x80;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjClass {
    Taggable,
    Function,
    Arguments,
    Other,
}

pub fn class_of(kind: &ObjKind<'_>) -> ObjClass {
    match kind {
        ObjKind::Plain(_) | ObjKind::Native(_, _) => ObjClass::Taggable,
        ObjKind::Function(_) | ObjKind::Bound(_) => ObjClass::Function,
        ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => ObjClass::Arguments,
        _ => ObjClass::Other,
    }
}

#[derive(Clone, PartialEq)]
enum Canon {
    Undefined,
    Null,
    Bool(bool),
    Num(u64),
    Str(JsString),
    Object(usize),
}

fn canon(value: &Value<'_>) -> Canon {
    match value {
        Value::Undefined => Canon::Undefined,
        Value::Null => Canon::Null,
        Value::Bool(flag) => Canon::Bool(*flag),
        Value::Number(number) => Canon::Num(number.to_bits()),
        Value::Str(text) => Canon::Str(text.clone()),
        Value::Object(obj) => Canon::Object(address_of(obj)),
    }
}

struct Frame {
    decoder: usize,
    awaiting_forward: bool,
    read_args: Vec<Canon>,
    epoch: u64,
    last_read: Option<Canon>,
}

#[derive(Clone)]
struct TableEntry {
    read_args: Vec<Canon>,
    epoch: u64,
    value: Canon,
}

thread_local! {
    static STATE_TRACING: Cell<bool> = const { Cell::new(false) };
    static VIOLATION: Cell<bool> = const { Cell::new(false) };
    static EPOCH: Cell<u64> = const { Cell::new(0) };
    static ACTIVE_SHAPE: RefCell<Shape> = RefCell::new(Shape::default());
    static TABLE: RefCell<FxHashMap<(usize, PropKey), TableEntry>> =
        RefCell::new(FxHashMap::default());
    static FRAME_STACK: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
    static GATE_DEPTH: Cell<usize> = const { Cell::new(0) };
    static GATE_FLAGS: RefCell<Vec<Option<PropKey>>> = const { RefCell::new(Vec::new()) };
    static GATE_RESETS: RefCell<Vec<Option<PropKey>>> = const { RefCell::new(Vec::new()) };
    static GATE_LEN_MASK: Cell<u64> = const { Cell::new(0) };
    pub(super) static ARGUMENTS_SUB: Cell<bool> = const { Cell::new(false) };
}

fn length_bit(len: usize) -> u32 {
    if len >= 63 {
        63
    } else {
        len as u32
    }
}

fn gate_len_mask(shape: &Shape) -> u64 {
    let mut mask = 0u64;
    for gate in &shape.gates {
        let PropKey::Name(name) = &gate.key else {
            continue;
        };
        mask |= 1u64 << length_bit(name.len());
    }
    mask
}

pub struct StateTracingGuard<'a>(PhantomData<&'a ()>);

impl Drop for StateTracingGuard<'_> {
    fn drop(&mut self) {
        STATE_TRACING.with(|flag| flag.set(false));
        release_state();
    }
}

fn release_state() {
    ACTIVE_SHAPE.with(|active| drop(std::mem::take(&mut *active.borrow_mut())));
    super::registry::release();
    TABLE.with(|table| drop(std::mem::take(&mut *table.borrow_mut())));
    FRAME_STACK.with(|stack| drop(std::mem::take(&mut *stack.borrow_mut())));
    GATE_FLAGS.with(|flags| drop(std::mem::take(&mut *flags.borrow_mut())));
    GATE_RESETS.with(|resets| drop(std::mem::take(&mut *resets.borrow_mut())));
    super::probe::reset();
}

pub fn set_state_tracing(shape: Option<&Shape>) -> StateTracingGuard<'_> {
    match shape {
        Some(shape) => {
            GATE_LEN_MASK.with(|mask| mask.set(gate_len_mask(shape)));
            ACTIVE_SHAPE.with(|active| *active.borrow_mut() = shape.clone());
            super::registry::release();
            TABLE.with(|table| table.borrow_mut().clear());
            FRAME_STACK.with(|stack| stack.borrow_mut().clear());
            GATE_DEPTH.with(|depth| depth.set(0));
            GATE_FLAGS.with(|flags| flags.borrow_mut().clear());
            GATE_RESETS.with(|resets| resets.borrow_mut().clear());
            EPOCH.with(|epoch| epoch.set(0));
            VIOLATION.with(|violation| violation.set(false));
            ARGUMENTS_SUB.with(|flag| flag.set(false));
            super::probe::reset();
            STATE_TRACING.with(|flag| flag.set(true));
        }
        None => STATE_TRACING.with(|flag| flag.set(false)),
    }
    StateTracingGuard(PhantomData)
}

#[inline(always)]
pub fn state_tracing() -> bool {
    STATE_TRACING.with(Cell::get)
}

pub fn take_violation() -> Option<&'static &'static str> {
    if VIOLATION.with(Cell::get) {
        VIOLATION.with(|violation| violation.set(false));
        Some(&UNPROVEN_GENERIC)
    } else {
        None
    }
}

pub(crate) fn violation_pending() -> bool {
    VIOLATION.with(Cell::get)
}

pub(crate) fn invocation_quiet() -> bool {
    gate_depth() == 0 && FRAME_STACK.with(|stack| stack.borrow().is_empty())
}

pub(crate) fn machinery_owner(obj: &ObjRef<'_>) -> Option<usize> {
    self_owner(address_of(obj))
}

pub(super) fn breach() {
    VIOLATION.with(|violation| violation.set(true));
}

pub fn address_of(obj: &ObjRef<'_>) -> usize {
    obj.as_ptr() as usize
}

#[inline(always)]
pub fn note_object_drop(object: &JsObject<'_>) {
    let registrable = match &object.kind {
        ObjKind::Function(_) | ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => true,
        ObjKind::Plain(tag) => tag & MONITOR_MARK != 0,
        _ => false,
    };
    if registrable && state_tracing() {
        forget_object(std::ptr::from_ref(object) as usize);
    }
}

#[cold]
#[inline(never)]
fn forget_object(addr: usize) {
    if super::registry::forget(addr) {
        TABLE.with(|table| table.borrow_mut().retain(|(entry, _), _| *entry != addr));
        super::probe::forget(addr);
    }
}

#[inline(always)]
#[allow(clippy::needless_return)]
pub fn note_rotation_step() {
    if !state_tracing() {
        return;
    }
    EPOCH.with(|epoch| epoch.set(epoch.get() + 1));
}

fn mark_container(obj: &ObjRef<'_>) {
    let mut borrowed = obj.borrow_mut();
    if borrowed.trace_tag() == 0 {
        borrowed.set_trace_tag(MONITOR_MARK);
    }
}

#[inline(always)]
#[allow(clippy::needless_return)]
pub fn register_machinery_self(container: &ObjRef<'_>, decoder: usize) {
    if !state_tracing() {
        return;
    }
    mark_container(container);
    super::registry::remember_self(address_of(container), decoder);
}

#[inline(always)]
pub fn note_rebind(span: Span, value: &Value<'_>) {
    if let Value::Object(obj) = value {
        if state_tracing() {
            register_rewrite(span, obj);
        }
    }
}

#[inline(never)]
fn register_rewrite(span: Span, obj: &ObjRef<'_>) {
    let decoder = ACTIVE_SHAPE.with(|shape| {
        shape
            .borrow()
            .rewrites
            .iter()
            .find(|(rewrite, _)| *rewrite == span)
            .map(|(_, decoder)| *decoder)
    });
    if let Some(decoder) = decoder {
        super::registry::note_rewritten(address_of(obj));
        register_machinery_self(obj, decoder);
    }
}

fn canon_argument(value: &Value<'_>, numeric: bool) -> Canon {
    match value {
        Value::Str(text) if numeric => Canon::Num(to_number_units(text.units()).to_bits()),
        other => canon(other),
    }
}

fn push_frame(decoder: usize, args: &[Value<'_>], awaiting_forward: bool) {
    let (used, numeric) = ACTIVE_SHAPE.with(|shape| {
        let shape = shape.borrow();
        let of = |params: &[Vec<bool>]| params.get(decoder).cloned().unwrap_or_default();
        (of(&shape.used_params), of(&shape.numeric_params))
    });
    let read_args = args
        .iter()
        .enumerate()
        .filter(|(index, _)| used.get(*index).copied().unwrap_or(true))
        .map(|(index, value)| canon_argument(value, numeric.get(index).copied().unwrap_or(false)))
        .collect();
    let epoch = EPOCH.with(Cell::get);
    FRAME_STACK.with(|stack| {
        stack.borrow_mut().push(Frame {
            decoder,
            awaiting_forward,
            read_args,
            epoch,
            last_read: None,
        })
    });
}

fn pop_frame(result: Option<&Value<'_>>) {
    let popped = FRAME_STACK.with(|stack| stack.borrow_mut().pop());
    let Some(frame) = popped else {
        return;
    };
    let Some(read) = frame.last_read else {
        return;
    };
    let Some(result) = result else {
        return;
    };
    if read != canon(result) {
        breach();
    }
}

fn gate_depth() -> usize {
    GATE_DEPTH.with(Cell::get)
}

pub fn enter_invocation(container: Option<&ObjRef<'_>>, args: &[Value<'_>]) -> bool {
    if !state_tracing() {
        return false;
    }
    let Some(container) = container else {
        return false;
    };
    let Some(decoder) = self_owner(address_of(container)) else {
        return false;
    };
    if gate_depth() > 0 {
        breach();
    }
    if takes_the_forward(decoder) {
        return false;
    }
    let awaiting_forward =
        is_template(decoder) && !super::registry::is_rewritten(address_of(container));
    push_frame(decoder, args, awaiting_forward);
    true
}

fn is_template(decoder: usize) -> bool {
    ACTIVE_SHAPE.with(|shape| {
        shape
            .borrow()
            .rewrites
            .iter()
            .any(|(_, owner)| *owner == decoder)
    })
}

fn takes_the_forward(decoder: usize) -> bool {
    FRAME_STACK.with(|stack| match stack.borrow_mut().last_mut() {
        Some(frame) if frame.decoder == decoder && frame.awaiting_forward => {
            frame.awaiting_forward = false;
            true
        }
        _ => false,
    })
}

pub fn leave_invocation(pushed: bool, result: Option<&Value<'_>>) {
    if !pushed {
        return;
    }
    pop_frame(result);
}

fn innermost_frame_for_owner(owner: usize) -> Option<(Vec<Canon>, u64)> {
    FRAME_STACK.with(|stack| {
        stack
            .borrow()
            .iter()
            .rev()
            .find(|frame| frame.decoder == owner)
            .map(|frame| (frame.read_args.clone(), frame.epoch))
    })
}

fn mark_innermost_frame_read(owner: usize, value: Canon) {
    FRAME_STACK.with(|stack| {
        if let Some(frame) = stack
            .borrow_mut()
            .iter_mut()
            .rev()
            .find(|frame| frame.decoder == owner)
        {
            frame.last_read = Some(value);
        }
    });
}

fn record_new_entry(owner: usize, addr: usize, key: PropKey, value: &Value<'_>) {
    let Some((read_args, epoch)) = innermost_frame_for_owner(owner) else {
        breach();
        return;
    };
    let identity = |canon: &Canon| matches!(canon, Canon::Object(_));
    if matches!(value, Value::Object(_)) || read_args.iter().any(identity) {
        breach();
        return;
    }
    TABLE.with(|table| {
        table.borrow_mut().insert(
            (addr, key),
            TableEntry {
                read_args,
                epoch,
                value: canon(value),
            },
        );
    });
}

fn check_and_record(owner: usize, addr: usize, key: PropKey, value: &Value<'_>, is_read: bool) {
    let probe = super::probe::probing();
    if super::probe::was_skipped(addr, &key) {
        if !(probe && is_read) {
            breach();
        }
        return;
    }
    if probe && !is_read {
        breach();
        return;
    }
    let found = TABLE.with(|table| table.borrow().get(&(addr, key.clone())).cloned());
    let Some(entry) = found else {
        if is_read {
            breach();
        } else {
            record_new_entry(owner, addr, key, value);
        }
        return;
    };
    match innermost_frame_for_owner(owner) {
        Some((read_args, epoch)) if entry.read_args == read_args && entry.epoch == epoch => {
            if is_read {
                mark_innermost_frame_read(owner, entry.value);
            } else if entry.value != canon(value) {
                breach();
            }
        }
        _ => breach(),
    }
}

fn is_a_gate_key(key: &PropKey) -> bool {
    let PropKey::Name(name) = key else {
        return false;
    };
    let bit = 1u64 << length_bit(name.len());
    if GATE_LEN_MASK.with(Cell::get) & bit == 0 {
        return false;
    }
    ACTIVE_SHAPE.with(|shape| shape.borrow().gates.iter().any(|gate| &gate.key == key))
}

fn is_a_memo_container_key(owner: usize, key: &PropKey) -> bool {
    let PropKey::Name(_) = key else {
        return false;
    };
    ACTIVE_SHAPE.with(|shape| {
        shape
            .borrow()
            .memos
            .iter()
            .any(|memo| memo.owner == owner && memo.container_key.as_ref() == Some(key))
    })
}

fn innermost_frame_decoder() -> Option<usize> {
    FRAME_STACK.with(|stack| stack.borrow().last().map(|frame| frame.decoder))
}

fn register_sub(container: &ObjRef<'_>, owner: usize) {
    mark_container(container);
    super::registry::remember_sub(address_of(container), owner);
}

fn register_arguments_sub(container: &ObjRef<'_>, owner: usize) {
    ARGUMENTS_SUB.with(|flag| flag.set(true));
    let passed = crate::object::array_length(container).unwrap_or(usize::MAX);
    super::registry::remember_passed(address_of(container), passed);
    super::registry::remember_sub(address_of(container), owner);
}

pub(super) fn in_mapped_range(addr: usize, key: &PropKey) -> bool {
    let PropKey::Index(index) = key else {
        return false;
    };
    super::registry::passed(addr).is_some_and(|passed| (*index as usize) < passed)
}

fn overwrites_sub(container: &ObjRef<'_>, key: &PropKey) -> bool {
    match container.borrow().props.get(key) {
        Some(Value::Object(previous)) => sub_owner(address_of(previous)).is_some(),
        _ => false,
    }
}

fn conditional_flag() -> Option<PropKey> {
    GATE_FLAGS.with(|flags| flags.borrow().iter().rev().find_map(Clone::clone))
}

fn in_conditional_gate() -> bool {
    gate_depth() > 0 && conditional_flag().is_some()
}

fn is_fresh_plain(object: &ObjRef<'_>) -> bool {
    let fresh = {
        let borrowed = object.borrow();
        matches!(borrowed.kind, ObjKind::Plain(_)) && borrowed.props.is_empty()
    };
    let addr = address_of(object);
    fresh && self_owner(addr).is_none() && sub_owner(addr).is_none()
}

fn resets_memo(key: &PropKey, value: &Value<'_>) -> bool {
    let resetting = GATE_RESETS.with(|resets| resets.borrow().last().cloned().flatten());
    resetting.as_ref() == Some(key) && value.as_object().is_some_and(is_fresh_plain)
}

fn finish_self_write(container: &ObjRef<'_>, owner: usize, key: &PropKey, value: &Value<'_>) {
    if conditional_flag().is_some_and(|flag| &flag != key) {
        breach();
        return;
    }
    if gate_depth() == 0 || !is_a_gate_key(key) {
        breach();
        return;
    }
    if overwrites_sub(container, key) && !resets_memo(key, value) {
        breach();
        return;
    }
    let Some(object) = value.as_object() else {
        if is_a_memo_container_key(owner, key) {
            breach();
        }
        return;
    };
    let class = class_of(&object.borrow().kind);
    match class {
        ObjClass::Taggable if matches!(object.borrow().kind, ObjKind::Plain(_)) => {
            register_sub(object, owner)
        }
        ObjClass::Arguments => register_arguments_sub(object, owner),
        ObjClass::Function if !is_a_memo_container_key(owner, key) => {}
        _ => breach(),
    }
}

#[inline(never)]
fn note_write_registered(container: &ObjRef<'_>, key: &PropKey, value: &Value<'_>) {
    let addr = address_of(container);
    if let Some(owner) = sub_owner(addr) {
        if in_conditional_gate() || in_mapped_range(addr, key) {
            breach();
            return;
        }
        check_and_record(owner, addr, key.clone(), value, false);
    }
}

#[inline(never)]
fn note_write_function(container: &ObjRef<'_>, key: &PropKey, value: &Value<'_>) {
    match self_owner(address_of(container)) {
        Some(owner) => finish_self_write(container, owner, key, value),
        None => breach(),
    }
}

#[inline(never)]
fn note_write_arguments(container: &ObjRef<'_>, key: &PropKey, value: &Value<'_>) {
    if !ARGUMENTS_SUB.with(Cell::get) {
        return;
    }
    let addr = address_of(container);
    if let Some(owner) = sub_owner(addr) {
        if in_conditional_gate() {
            breach();
            return;
        }
        check_and_record(owner, addr, key.clone(), value, false);
    }
}

#[inline(always)]
pub fn note_write(
    container: &ObjRef<'_>,
    tag: u8,
    class: ObjClass,
    key: &PropKey,
    value: &Value<'_>,
) {
    let watched_kind =
        tag & MONITOR_MARK != 0 || matches!(class, ObjClass::Function | ObjClass::Arguments);
    if watched_kind && state_tracing() {
        note_write_watched(container, tag, class, key, value);
    }
}

#[inline(never)]
fn note_write_watched(
    container: &ObjRef<'_>,
    tag: u8,
    class: ObjClass,
    key: &PropKey,
    value: &Value<'_>,
) {
    if tag & MONITOR_MARK != 0 {
        note_write_registered(container, key, value);
        return;
    }
    match class {
        ObjClass::Function => note_write_function(container, key, value),
        ObjClass::Arguments => note_write_arguments(container, key, value),
        ObjClass::Taggable | ObjClass::Other => {}
    }
}

#[inline(always)]
pub fn note_capture(value: &Value<'_>) {
    if let Value::Object(obj) = value {
        if state_tracing() {
            capture_arguments(obj);
        }
    }
}

#[inline(never)]
fn capture_arguments(obj: &ObjRef<'_>) {
    if gate_depth() == 0 || class_of(&obj.borrow().kind) != ObjClass::Arguments {
        return;
    }
    if in_conditional_gate() {
        breach();
        return;
    }
    match innermost_frame_decoder() {
        Some(decoder) => register_arguments_sub(obj, decoder),
        None => breach(),
    }
}

#[allow(clippy::needless_return)]
pub fn note_delete(container: &ObjRef<'_>, _key: &PropKey) {
    if !state_tracing() {
        return;
    }
    let addr = address_of(container);
    if self_owner(addr).is_some() || sub_owner(addr).is_some() {
        breach();
    }
}

#[inline(always)]
pub fn note_read(
    container: &ObjRef<'_>,
    tag: u8,
    is_arguments: bool,
    key: &PropKey,
    value: &Value<'_>,
) {
    if tag & MONITOR_MARK == 0 && !is_arguments {
        return;
    }
    note_read_watched(container, tag, key, value);
}

#[inline(never)]
fn note_read_watched(container: &ObjRef<'_>, tag: u8, key: &PropKey, value: &Value<'_>) {
    if !state_tracing() || (tag & MONITOR_MARK == 0 && !ARGUMENTS_SUB.with(Cell::get)) {
        return;
    }
    let addr = address_of(container);
    let Some(owner) = sub_owner(addr) else {
        return;
    };
    if in_mapped_range(addr, key) {
        breach();
        return;
    }
    check_and_record(owner, addr, key.clone(), value, true);
}

#[inline(always)]
pub fn note_key_observation(target: &Value<'_>) {
    if !state_tracing() {
        return;
    }
    if let Value::Object(obj) = target {
        observe_keys(obj);
    }
}

fn is_registered(obj: &ObjRef<'_>, marked: bool) -> bool {
    if !marked && !ARGUMENTS_SUB.with(Cell::get) {
        return false;
    }
    let addr = address_of(obj);
    self_owner(addr).is_some() || sub_owner(addr).is_some()
}

fn is_registered_object(obj: &ObjRef<'_>) -> bool {
    let marked = obj.borrow().trace_tag() & MONITOR_MARK != 0;
    is_registered(obj, marked)
}

fn inherits_registered(obj: &ObjRef<'_>) -> bool {
    let Some(start) = obj.borrow().proto.clone() else {
        return false;
    };
    let found =
        crate::object::walk_prototype_chain(start, |link| is_registered_object(link).then_some(()));
    !matches!(found, Ok(None))
}

#[inline(never)]
fn observe_keys(obj: &ObjRef<'_>) {
    if inherits_registered(obj) {
        breach();
        return;
    }
    if is_registered_object(obj) && (gate_depth() == 0 || in_conditional_gate()) {
        breach();
    }
}

#[inline(always)]
pub fn note_inherited_link(link: &ObjRef<'_>, tag: u8, is_arguments: bool) {
    if tag & MONITOR_MARK != 0 || is_arguments {
        inherited_link(link, tag);
    }
}

#[inline(never)]
fn inherited_link(link: &ObjRef<'_>, tag: u8) {
    if is_registered(link, tag & MONITOR_MARK != 0) {
        breach();
    }
}

#[inline(always)]
pub fn note_prototype(proto: &ObjRef<'_>) {
    if state_tracing() {
        prototype_use(proto);
    }
}

#[inline(never)]
fn prototype_use(proto: &ObjRef<'_>) {
    if is_registered_object(proto) {
        breach();
    }
}

pub(super) fn gates_of(owner: usize) -> Vec<Span> {
    ACTIVE_SHAPE.with(|shape| {
        let shape = shape.borrow();
        let owned = shape.gate_owners.iter().filter(|(_, gate)| *gate == owner);
        owned.map(|(span, _)| *span).collect()
    })
}

fn is_gate_span(span: Span) -> bool {
    ACTIVE_SHAPE.with(|shape| shape.borrow().gates.iter().any(|gate| gate.guard == span))
}

fn reset_memo_at(span: Span) -> Option<PropKey> {
    ACTIVE_SHAPE.with(|shape| {
        shape
            .borrow()
            .resets
            .iter()
            .find(|reset| reset.guard == span)
            .map(|reset| reset.memo.clone())
    })
}

fn conditional_flag_at(span: Span) -> Option<PropKey> {
    ACTIVE_SHAPE.with(|shape| {
        shape
            .borrow()
            .conditional
            .iter()
            .find(|(guard, _)| *guard == span)
            .map(|(_, flag)| flag.clone())
    })
}

pub fn enter_gate(span: Span) -> bool {
    if !state_tracing() {
        return false;
    }
    let flag = conditional_flag_at(span);
    if flag.is_none() && !is_gate_span(span) {
        return false;
    }
    super::probe::note_gate_entered(span);
    GATE_DEPTH.with(|depth| depth.set(depth.get() + 1));
    GATE_FLAGS.with(|flags| flags.borrow_mut().push(flag));
    let memo = reset_memo_at(span);
    GATE_RESETS.with(|resets| resets.borrow_mut().push(memo));
    true
}

pub fn leave_gate(entered: bool, normal: bool) {
    if !entered {
        return;
    }
    GATE_FLAGS.with(|flags| flags.borrow_mut().pop());
    GATE_RESETS.with(|resets| resets.borrow_mut().pop());
    GATE_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    if !normal {
        breach();
    }
}
