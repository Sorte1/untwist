use std::cell::RefCell;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::budget::BINDING_BYTES;
use crate::env::{Bindings, ScopeChainTooDeep, SCOPE_CHAIN_CAP};
use crate::heap::Charge;
use crate::names::Name;
use crate::value::Value;

const INLINE_BITS: usize = 64;
const TAIL_BITS: usize = 32;
const INDEXED_LAYOUT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotKind {
    Mutable,
    Immutable,
    Constant,
}

#[derive(Debug)]
pub struct FrameLayout {
    names: Box<[Name]>,
    kinds: Box<[SlotKind]>,
    index: Option<FxHashMap<Name, u32>>,
}

impl FrameLayout {
    pub fn new(slots: Vec<(Name, SlotKind)>) -> Option<FrameLayout> {
        u32::try_from(slots.len()).ok()?;
        let (names, kinds): (Vec<Name>, Vec<SlotKind>) = slots.into_iter().unzip();
        let index = (names.len() > INDEXED_LAYOUT).then(|| {
            names
                .iter()
                .enumerate()
                .rev()
                .filter_map(|(slot, name)| Some((*name, u32::try_from(slot).ok()?)))
                .collect()
        });
        Some(FrameLayout {
            names: names.into_boxed_slice(),
            kinds: kinds.into_boxed_slice(),
            index,
        })
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub(crate) fn slot(&self, name: Name) -> Option<usize> {
        match &self.index {
            Some(index) => index.get(&name).map(|slot| *slot as usize),
            None => self.names.iter().position(|bound| *bound == name),
        }
    }

    pub(crate) fn is_mutable(&self, slot: usize) -> bool {
        self.kinds.get(slot) == Some(&SlotKind::Mutable)
    }

    fn refused(&self, slot: usize) -> Option<Stored> {
        match self.kinds.get(slot)? {
            SlotKind::Mutable => None,
            SlotKind::Immutable => Some(Stored::Immutable),
            SlotKind::Constant => Some(Stored::Constant),
        }
    }
}

fn tail_words(slots: usize) -> usize {
    slots.saturating_sub(INLINE_BITS).div_ceil(TAIL_BITS)
}

fn tail_word(value: &Value<'_>) -> u32 {
    match value {
        Value::Number(bits) => *bits as u32,
        _ => 0,
    }
}

pub(crate) enum Stored {
    Done,
    Immutable,
    Constant,
}

pub enum WriteSlot<'s, 'a> {
    Slot(&'s mut Value<'a>),
    Immutable,
    Constant,
    Absent,
}

#[derive(Debug)]
pub enum FrameBody<'a> {
    Dynamic(Bindings<'a>),
    Fixed {
        layout: Rc<FrameLayout>,
        slots: Box<[Value<'a>]>,
        declared: u64,
    },
}

impl Default for FrameBody<'_> {
    fn default() -> Self {
        FrameBody::Dynamic(Bindings::default())
    }
}

#[cold]
fn fixed_is_declared(slots: &[Value<'_>], declared: u64, width: usize, slot: usize) -> bool {
    if slot < INLINE_BITS {
        return declared >> slot & 1 == 1;
    }
    let tail = slot - INLINE_BITS;
    slots
        .get(width + tail / TAIL_BITS)
        .is_some_and(|word| tail_word(word) >> (tail % TAIL_BITS) & 1 == 1)
}

#[cold]
fn fixed_mark_declared(slots: &mut [Value<'_>], declared: &mut u64, width: usize, slot: usize) {
    if slot < INLINE_BITS {
        *declared |= 1 << slot;
        return;
    }
    let tail = slot - INLINE_BITS;
    if let Some(word) = slots.get_mut(width + tail / TAIL_BITS) {
        let bits = tail_word(word) | 1 << (tail % TAIL_BITS);
        *word = Value::Number(f64::from(bits));
    }
}

#[cold]
fn fixed_declared_count(slots: &[Value<'_>], declared: u64, width: usize) -> usize {
    let tail: u32 = slots
        .get(width..)
        .unwrap_or_default()
        .iter()
        .map(|word| tail_word(word).count_ones())
        .sum();
    (declared.count_ones() + tail) as usize
}

#[cold]
fn fixed_any<'a>(slots: &[Value<'a>], width: usize, test: fn(&Value<'a>) -> bool) -> bool {
    slots.iter().take(width).any(test)
}

impl<'a> FrameBody<'a> {
    pub fn fixed(layout: &Rc<FrameLayout>) -> FrameBody<'a> {
        let width = layout.len();
        let mut slots = Vec::with_capacity(width + tail_words(width));
        slots.resize(width, Value::Undefined);
        slots.resize(width + tail_words(width), Value::Number(0.0));
        FrameBody::Fixed {
            layout: layout.clone(),
            slots: slots.into_boxed_slice(),
            declared: 0,
        }
    }

    #[inline]
    pub fn get(&self, name: Name) -> Option<&Value<'a>> {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings.get(name);
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings.get(name);
        }
        match self {
            FrameBody::Dynamic(bindings) => bindings.get(name),
            FrameBody::Fixed { .. } => self.fixed_get(name),
        }
    }

    #[cold]
    fn fixed_get(&self, name: Name) -> Option<&Value<'a>> {
        let FrameBody::Fixed {
            layout,
            slots,
            declared,
        } = self
        else {
            return None;
        };
        let slot = layout.slot(name)?;
        fixed_is_declared(slots, *declared, layout.len(), slot)
            .then(|| slots.get(slot))
            .flatten()
    }

    #[inline]
    pub fn get_mut(&mut self, name: Name) -> Option<&mut Value<'a>> {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings.get_mut(name);
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings.get_mut(name);
        }
        match self {
            FrameBody::Dynamic(bindings) => bindings.get_mut(name),
            FrameBody::Fixed { .. } => self.fixed_get_mut(name),
        }
    }

    #[inline]
    pub fn slot_for_write(&mut self, name: Name) -> WriteSlot<'_, 'a> {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings
                .get_mut(name)
                .map_or(WriteSlot::Absent, WriteSlot::Slot);
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings
                .get_mut(name)
                .map_or(WriteSlot::Absent, WriteSlot::Slot);
        }
        self.rare_slot_for_write(name)
    }

    fn rare_slot_for_write(&mut self, name: Name) -> WriteSlot<'_, 'a> {
        if self.is_immutable(name) {
            return WriteSlot::Immutable;
        }
        if self.fixed_kind(name) == Some(SlotKind::Constant) {
            return WriteSlot::Constant;
        }
        self.get_mut(name)
            .map_or(WriteSlot::Absent, WriteSlot::Slot)
    }

    #[cold]
    fn fixed_get_mut(&mut self, name: Name) -> Option<&mut Value<'a>> {
        let FrameBody::Fixed {
            layout,
            slots,
            declared,
        } = self
        else {
            return None;
        };
        let slot = layout.slot(name)?;
        let writable =
            layout.is_mutable(slot) && fixed_is_declared(slots, *declared, layout.len(), slot);
        writable.then(|| slots.get_mut(slot)).flatten()
    }

    #[inline]
    pub fn is_immutable(&self, name: Name) -> bool {
        match self {
            FrameBody::Dynamic(bindings) => bindings.is_immutable(name),
            FrameBody::Fixed { .. } => self.fixed_is_immutable(name),
        }
    }

    #[cold]
    fn fixed_is_immutable(&self, name: Name) -> bool {
        self.fixed_kind(name) == Some(SlotKind::Immutable)
    }

    #[cold]
    fn fixed_kind(&self, name: Name) -> Option<SlotKind> {
        let FrameBody::Fixed { layout, .. } = self else {
            return None;
        };
        let kind = *layout.kinds.get(layout.slot(name)?)?;
        self.fixed_get(name).map(|_| kind)
    }

    #[inline]
    pub fn contains_key(&self, name: Name) -> bool {
        self.get(name).is_some()
    }

    #[inline(never)]
    pub fn insert(&mut self, name: Name, value: Value<'a>) -> bool {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings.insert(name, value);
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings.insert(name, value);
        }
        match self {
            FrameBody::Dynamic(bindings) => bindings.insert(name, value),
            FrameBody::Fixed { .. } => self.fixed_write(name, value, true),
        }
    }

    #[inline(never)]
    pub fn insert_if_absent(&mut self, name: Name, value: Value<'a>) -> bool {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings.insert_if_absent(name, value);
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings.insert_if_absent(name, value);
        }
        match self {
            FrameBody::Dynamic(bindings) => bindings.insert_if_absent(name, value),
            FrameBody::Fixed { .. } => self.fixed_write(name, value, false),
        }
    }

    pub fn set_immutable(&mut self, name: Name, value: Value<'a>) -> bool {
        match self {
            FrameBody::Dynamic(bindings) => {
                let added = bindings.is_empty();
                *bindings = Bindings::Immutable(name, value);
                added
            }
            FrameBody::Fixed { .. } => self.fixed_write(name, value, true),
        }
    }

    #[cold]
    fn fixed_write(&mut self, name: Name, value: Value<'a>, overwrite: bool) -> bool {
        let FrameBody::Fixed {
            layout,
            slots,
            declared,
        } = self
        else {
            return false;
        };
        let Some(slot) = layout.slot(name) else {
            debug_assert!(false, "a fixed frame never gains a name outside its layout");
            return false;
        };
        let width = layout.len();
        let added = !fixed_is_declared(slots, *declared, width, slot);
        if added || overwrite {
            if let Some(target) = slots.get_mut(slot) {
                *target = value;
            }
        }
        fixed_mark_declared(slots, declared, width, slot);
        added
    }

    #[inline]
    pub(crate) fn slot_value(&self, slot: u32) -> Option<&Value<'a>> {
        match self {
            FrameBody::Fixed { slots, .. } => slots.get(slot as usize),
            FrameBody::Dynamic(_) => None,
        }
    }

    #[inline]
    pub(crate) fn slot_value_mut(&mut self, slot: u32) -> Option<&mut Value<'a>> {
        match self {
            FrameBody::Fixed { slots, .. } => slots.get_mut(slot as usize),
            FrameBody::Dynamic(_) => None,
        }
    }

    #[inline(always)]
    pub(crate) fn read_slot(&self, slot: u32) -> Option<Value<'a>> {
        match self.slot_value(slot)? {
            Value::Number(number) => Some(Value::Number(*number)),
            value => Some(value.clone()),
        }
    }

    #[inline(always)]
    pub(crate) fn store_slot(&mut self, slot: u32, value: Value<'a>) -> Option<Stored> {
        let FrameBody::Fixed { layout, .. } = self else {
            return None;
        };
        if !layout.is_mutable(slot as usize) {
            return layout.refused(slot as usize);
        }
        let place = self.slot_value_mut(slot)?;
        *place = value;
        Some(Stored::Done)
    }

    pub(crate) fn slot_declared(&self, slot: u32) -> bool {
        match self {
            FrameBody::Fixed {
                layout,
                slots,
                declared,
            } => fixed_is_declared(slots, *declared, layout.len(), slot as usize),
            FrameBody::Dynamic(_) => false,
        }
    }

    fn write_slot(&mut self, slot: u32, value: Value<'a>) -> bool {
        let FrameBody::Fixed {
            layout,
            slots,
            declared,
        } = self
        else {
            return false;
        };
        let (width, slot) = (layout.len(), slot as usize);
        if slot >= width {
            return false;
        }
        let added = !fixed_is_declared(slots, *declared, width, slot);
        if let Some(target) = slots.get_mut(slot) {
            *target = value;
        }
        fixed_mark_declared(slots, declared, width, slot);
        added
    }

    fn write_slot_if_absent(&mut self, slot: u32, value: Value<'a>) -> bool {
        let declared = match self {
            FrameBody::Fixed { .. } => self.slot_declared(slot),
            FrameBody::Dynamic(_) => return false,
        };
        !declared && self.write_slot(slot, value)
    }

    pub fn clear(&mut self) {
        match self {
            FrameBody::Dynamic(bindings) => bindings.clear(),
            FrameBody::Fixed {
                layout,
                slots,
                declared,
            } => {
                let width = layout.len();
                for (at, slot) in slots.iter_mut().enumerate() {
                    *slot = if at < width {
                        Value::Undefined
                    } else {
                        Value::Number(0.0)
                    };
                }
                *declared = 0;
            }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if let FrameBody::Dynamic(bindings @ Bindings::Small(_)) = self {
            return bindings.len();
        }
        if let FrameBody::Dynamic(bindings @ Bindings::Large(_)) = self {
            return bindings.len();
        }
        match self {
            FrameBody::Dynamic(bindings) => bindings.len(),
            FrameBody::Fixed {
                layout,
                slots,
                declared,
            } => fixed_declared_count(slots, *declared, layout.len()),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn any_value(&self, test: fn(&Value<'a>) -> bool) -> bool {
        match self {
            FrameBody::Dynamic(Bindings::Small(entries)) => {
                entries.iter().any(|(_, value)| test(value))
            }
            FrameBody::Dynamic(Bindings::Large(map)) => map.values().any(test),
            FrameBody::Dynamic(Bindings::Immutable(_, value)) => test(value),
            FrameBody::Fixed { layout, slots, .. } => fixed_any(slots, layout.len(), test),
        }
    }

    pub fn consume_values(self, mut keep: impl FnMut(Value<'a>)) {
        match self {
            FrameBody::Dynamic(Bindings::Small(entries)) => {
                for (_, value) in entries {
                    keep(value);
                }
            }
            FrameBody::Dynamic(Bindings::Large(map)) => {
                for value in map.into_values() {
                    keep(value);
                }
            }
            FrameBody::Dynamic(Bindings::Immutable(_, value)) => keep(value),
            FrameBody::Fixed { layout, slots, .. } => {
                for value in slots.into_vec().into_iter().take(layout.len()) {
                    keep(value);
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct Frame<'a> {
    pub body: FrameBody<'a>,
    pub parent: Option<FrameRef<'a>>,
    pub charge: Option<Charge>,
}

pub type FrameRef<'a> = Rc<RefCell<Frame<'a>>>;

impl<'a> Frame<'a> {
    pub fn dynamic(parent: Option<&FrameRef<'a>>, charge: Option<Charge>) -> FrameRef<'a> {
        Rc::new(RefCell::new(Frame {
            body: FrameBody::default(),
            parent: parent.cloned(),
            charge,
        }))
    }

    pub fn fixed(
        layout: &Rc<FrameLayout>,
        parent: Option<&FrameRef<'a>>,
        charge: Option<Charge>,
    ) -> FrameRef<'a> {
        Rc::new(RefCell::new(Frame {
            body: FrameBody::fixed(layout),
            parent: parent.cloned(),
            charge,
        }))
    }

    #[inline]
    fn charge_binding(&self, added: bool) {
        if let (true, Some(charge)) = (added, &self.charge) {
            charge.grow_lenient(BINDING_BYTES);
        }
        #[cfg(feature = "test-hooks")]
        if added && self.parent.is_some() {
            crate::test_hooks::note_frame_width(|| self.body.len());
        }
    }

    #[inline]
    pub fn declare(frame: &FrameRef<'a>, name: Name, value: Value<'a>) {
        let mut this = frame.borrow_mut();
        let added = this.body.insert(name, value);
        this.charge_binding(added);
    }

    #[inline]
    pub fn declare_immutable(frame: &FrameRef<'a>, name: Name, value: Value<'a>) {
        let mut this = frame.borrow_mut();
        let added = this.body.set_immutable(name, value);
        this.charge_binding(added);
    }

    #[inline]
    pub fn declare_if_absent(frame: &FrameRef<'a>, name: Name, value: Value<'a>) {
        let mut this = frame.borrow_mut();
        let added = this.body.insert_if_absent(name, value);
        this.charge_binding(added);
    }

    #[inline]
    pub(crate) fn declare_slot(frame: &FrameRef<'a>, slot: u32, value: Value<'a>) {
        let mut this = frame.borrow_mut();
        let added = this.body.write_slot(slot, value);
        this.charge_binding(added);
    }

    #[inline(always)]
    pub(crate) fn declare_slot_if_absent(frame: &FrameRef<'a>, slot: u32, value: Value<'a>) {
        let mut this = frame.borrow_mut();
        let added = this.body.write_slot_if_absent(slot, value);
        this.charge_binding(added);
    }

    #[inline(always)]
    pub(crate) fn parent_slot(frame: &FrameRef<'a>, slot: u32) -> Option<Value<'a>> {
        let this = frame.borrow();
        let parent = this.parent.as_ref()?.borrow();
        parent.body.read_slot(slot)
    }

    #[inline(always)]
    pub(crate) fn store_in_parent(
        frame: &FrameRef<'a>,
        slot: u32,
        value: Value<'a>,
    ) -> Option<Stored> {
        let this = frame.borrow();
        let mut parent = this.parent.as_ref()?.borrow_mut();
        parent.body.store_slot(slot, value)
    }

    pub fn slot_by_name(
        frame: &FrameRef<'a>,
        name: Name,
    ) -> Result<Option<Value<'a>>, ScopeChainTooDeep> {
        slot_from(frame, name, 0)
    }
}

fn slot_from<'a>(
    frame: &FrameRef<'a>,
    name: Name,
    depth: usize,
) -> Result<Option<Value<'a>>, ScopeChainTooDeep> {
    if depth >= SCOPE_CHAIN_CAP {
        return Err(ScopeChainTooDeep);
    }
    let this = frame.borrow();
    if let Some(value) = this.body.get(name) {
        return Ok(Some(value.clone()));
    }
    match &this.parent {
        Some(parent) => slot_from(parent, name, depth + 1),
        None => Ok(None),
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        crate::release::release_frame(self);
    }
}
