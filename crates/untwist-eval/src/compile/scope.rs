use std::rc::Rc;

use oxc_ast::ast::{BindingPattern, FormalParameters, Function, Statement};
use rustc_hash::FxHashMap;

use crate::bail::{Bail, ErrorKind};
use crate::env::{
    hoist_function_declarations, hoist_var_names, ScopeChainTooDeep, SCOPE_CHAIN_CAP,
};
use crate::frame::{Frame, FrameLayout, FrameRef, SlotKind, Stored};
use crate::host::Host;
use crate::interp::Interp;
use crate::names::{Name, Names};
use crate::value::{Eval, Value};

pub(crate) enum Provable {
    All,
    Slots(Box<[u32]>),
    Root,
    Pending { order: Rc<[u32]>, declared: u32 },
    Before { order: Rc<[u32]>, below: u32 },
}

pub(crate) struct ScopeChain {
    pub(crate) layout: Rc<FrameLayout>,
    provable: Provable,
    parent: Option<Rc<ScopeChain>>,
}

impl ScopeChain {
    pub(crate) fn new(
        layout: Rc<FrameLayout>,
        provable: Provable,
        parent: Option<Rc<ScopeChain>>,
    ) -> Rc<ScopeChain> {
        Rc::new(ScopeChain {
            layout,
            provable,
            parent,
        })
    }

    pub(crate) fn root() -> Option<Rc<ScopeChain>> {
        let layout = Rc::new(FrameLayout::new(Vec::new())?);
        Some(ScopeChain::new(layout, Provable::Root, None))
    }

    pub(crate) fn is_root(&self) -> bool {
        matches!(self.provable, Provable::Root)
    }

    fn proves(&self, slot: u32) -> bool {
        match &self.provable {
            Provable::All => true,
            Provable::Slots(slots) => slots.contains(&slot),
            Provable::Root => false,
            Provable::Pending { order, declared } => {
                order.get(slot as usize).is_some_and(|at| at < declared)
            }
            Provable::Before { order, below } => {
                order.get(slot as usize).is_some_and(|at| at < below)
            }
        }
    }

    fn maybe_here(&self) -> bool {
        matches!(self.provable, Provable::Pending { .. })
    }

    pub(crate) fn pending_order(&self) -> Option<&Rc<[u32]>> {
        match &self.provable {
            Provable::Pending { order, .. } => Some(order),
            _ => None,
        }
    }

    pub(crate) fn sibling(self: &Rc<Self>, provable: Provable) -> Rc<ScopeChain> {
        ScopeChain::new(self.layout.clone(), provable, self.parent.clone())
    }

    pub(crate) fn resolve(self: &Rc<Self>, name: Name) -> Resolved {
        resolve_from(Some(self), name, 0)
    }
}

impl Drop for ScopeChain {
    fn drop(&mut self) {
        let mut next = self.parent.take();
        while let Some(chain) = next {
            next = match Rc::try_unwrap(chain) {
                Ok(mut chain) => chain.parent.take(),
                Err(_) => None,
            };
        }
    }
}

pub(crate) enum Resolved {
    Local(u32),
    Outer {
        hops: u16,
        slot: u32,
    },
    Global {
        hops: u16,
        name: Name,
    },
    Maybe {
        slot: Box<Resolved>,
        otherwise: Box<Resolved>,
    },
    TooDeep,
}

fn at(hops: usize, slot: u32) -> Resolved {
    match u16::try_from(hops) {
        Ok(0) => Resolved::Local(slot),
        Ok(hops) => Resolved::Outer { hops, slot },
        Err(_) => Resolved::TooDeep,
    }
}

fn resolve_from(mut scope: Option<&Rc<ScopeChain>>, name: Name, mut hops: usize) -> Resolved {
    while let Some(current) = scope {
        if current.is_root() {
            break;
        }
        if hops >= SCOPE_CHAIN_CAP {
            return Resolved::TooDeep;
        }
        if let Some(slot) = current
            .layout
            .slot(name)
            .and_then(|s| u32::try_from(s).ok())
        {
            if current.proves(slot) {
                return at(hops, slot);
            }
            if hops > 0 || current.maybe_here() {
                let otherwise = resolve_from(current.parent.as_ref(), name, hops + 1);
                return Resolved::Maybe {
                    slot: Box::new(at(hops, slot)),
                    otherwise: Box::new(otherwise),
                };
            }
        }
        scope = current.parent.as_ref();
        hops += 1;
    }
    match u16::try_from(hops) {
        Ok(hops) if usize::from(hops) < SCOPE_CHAIN_CAP => Resolved::Global { hops, name },
        _ => Resolved::TooDeep,
    }
}

const UNLAID: &str = "a compiled binding in a frame the compiler did not lay out";

pub(crate) fn unlaid<'a>() -> crate::value::Abrupt<'a> {
    Bail::Unsupported(&UNLAID).into()
}

fn too_deep<'a>() -> crate::value::Abrupt<'a> {
    Bail::from(ScopeChainTooDeep).into()
}

fn ancestor<'a>(frame: &FrameRef<'a>, hops: u16) -> Option<FrameRef<'a>> {
    let mut current = frame.borrow().parent.clone()?;
    for _ in 1..hops {
        let next = current.borrow().parent.clone()?;
        current = next;
    }
    Some(current)
}

fn reaches_root(frame: &FrameRef<'_>, hops: u16) -> bool {
    let target = match hops {
        0 => Some(frame.clone()),
        hops => ancestor(frame, hops),
    };
    target.is_some_and(|target| target.borrow().parent.is_none())
}

fn with_frame<'a, R>(
    frame: &FrameRef<'a>,
    hops: u16,
    body: impl FnOnce(&Frame<'a>) -> Option<R>,
) -> Option<R> {
    if hops == 0 {
        return body(&frame.borrow());
    }
    let target = ancestor(frame, hops)?;
    let borrowed = target.borrow();
    body(&borrowed)
}

fn with_frame_mut<'a, R>(
    frame: &FrameRef<'a>,
    hops: u16,
    body: impl FnOnce(&mut Frame<'a>) -> Option<R>,
) -> Option<R> {
    if hops == 0 {
        return body(&mut frame.borrow_mut());
    }
    let target = ancestor(frame, hops)?;
    let mut borrowed = target.borrow_mut();
    body(&mut borrowed)
}

fn slot_of(resolved: &Resolved) -> (u16, u32) {
    match resolved {
        Resolved::Local(slot) => (0, *slot),
        Resolved::Outer { hops, slot } => (*hops, *slot),
        _ => (0, u32::MAX),
    }
}

fn declared(frame: &FrameRef<'_>, resolved: &Resolved) -> bool {
    let (hops, slot) = slot_of(resolved);
    with_frame(frame, hops, |target| Some(target.body.slot_declared(slot))).unwrap_or(false)
}

impl Resolved {
    #[cfg(feature = "test-hooks")]
    pub(crate) fn is_maybe(&self) -> bool {
        matches!(self, Resolved::Maybe { .. })
    }

    pub(crate) fn read<'a>(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        text: &'a str,
    ) -> Eval<'a, Value<'a>> {
        match self {
            Resolved::Local(slot) => frame.borrow().body.read_slot(*slot).ok_or_else(unlaid),
            Resolved::Outer { hops: 1, slot } => {
                Frame::parent_slot(frame, *slot).ok_or_else(unlaid)
            }
            Resolved::Outer { hops, slot } => {
                with_frame(frame, *hops, |target| target.body.read_slot(*slot)).ok_or_else(unlaid)
            }
            Resolved::Global { name, hops } => {
                debug_assert!(reaches_root(frame, *hops));
                let root = interp.root_frame().borrow();
                match root.body.get(*name) {
                    Some(value) => {
                        crate::object::note_root_touch(value);
                        Ok(value.clone())
                    }
                    None => Err(Bail::unresolved_global(text).into()),
                }
            }
            Resolved::Maybe { slot, otherwise } => match declared(frame, slot) {
                true => slot.read(interp, frame, text),
                false => otherwise.read(interp, frame, text),
            },
            Resolved::TooDeep => Err(too_deep()),
        }
    }

    pub(crate) fn lookup<'a>(
        &self,
        interp: &Interp<'a>,
        frame: &FrameRef<'a>,
    ) -> Eval<'a, Option<Value<'a>>> {
        match self {
            Resolved::Local(_) | Resolved::Outer { .. } => {
                let (hops, slot) = slot_of(self);
                with_frame(frame, hops, |target| target.body.slot_value(slot).cloned())
                    .map(Some)
                    .ok_or_else(unlaid)
            }
            Resolved::Global { name, .. } => {
                Ok(interp.root_frame().borrow().body.get(*name).cloned())
            }
            Resolved::Maybe { slot, otherwise } => match declared(frame, slot) {
                true => slot.lookup(interp, frame),
                false => otherwise.lookup(interp, frame),
            },
            Resolved::TooDeep => Err(too_deep()),
        }
    }

    pub(crate) fn write<'a>(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        (name, text): (Name, &'a str),
        value: Value<'a>,
    ) -> Eval<'a, ()> {
        let written = match self {
            Resolved::Local(_) | Resolved::Outer { .. } => write_slot(frame, self, value)?,
            Resolved::Global { .. } => {
                let root = interp.root_frame().clone();
                return interp.assign_name(&root, name, text, value);
            }
            Resolved::Maybe { slot, otherwise } => match declared(frame, slot) {
                true => write_slot(frame, slot, value)?,
                false => return otherwise.write(interp, frame, (name, text), value),
            },
            Resolved::TooDeep => return Err(too_deep()),
        };
        finish_store(interp, written)
    }

    pub(crate) fn near(&self) -> Near {
        match self {
            Resolved::Local(slot) => Near::Local(*slot),
            Resolved::Outer { hops: 1, slot } => Near::Parent(*slot),
            _ => Near::Far,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Near {
    Local(u32),
    Parent(u32),
    Far,
}

pub(crate) const CONSTANT_ASSIGNMENT: &str = "Assignment to constant variable.";

pub(crate) fn finish_store<'a>(interp: &mut Interp<'a>, stored: Stored) -> Eval<'a, ()> {
    match stored {
        Stored::Done => Ok(()),
        Stored::Immutable if interp.strict => {
            Err(interp.throw(ErrorKind::TypeError, CONSTANT_ASSIGNMENT))
        }
        Stored::Immutable => Ok(()),
        Stored::Constant => Err(interp.throw(ErrorKind::TypeError, CONSTANT_ASSIGNMENT)),
    }
}

impl Resolved {
    pub(crate) fn assign_var<'a>(&self, frame: &FrameRef<'a>, value: Value<'a>) -> Eval<'a, ()> {
        match self {
            Resolved::Local(_) | Resolved::Outer { .. } => {
                write_slot(frame, self, value)?;
                Ok(())
            }
            Resolved::TooDeep => Err(too_deep()),
            Resolved::Global { .. } | Resolved::Maybe { .. } => Err(unlaid()),
        }
    }

    pub(crate) fn has_var<'a>(&self) -> Eval<'a, ()> {
        match self {
            Resolved::Local(_) | Resolved::Outer { .. } => Ok(()),
            Resolved::TooDeep => Err(too_deep()),
            Resolved::Global { .. } | Resolved::Maybe { .. } => Err(unlaid()),
        }
    }
}

#[inline(always)]
fn write_slot<'a>(frame: &FrameRef<'a>, resolved: &Resolved, value: Value<'a>) -> Eval<'a, Stored> {
    let (hops, slot) = slot_of(resolved);
    let written = match hops {
        0 => frame.borrow_mut().body.store_slot(slot, value),
        1 => Frame::store_in_parent(frame, slot, value),
        _ => with_frame_mut(frame, hops, |target| target.body.store_slot(slot, value)),
    };
    written.ok_or_else(unlaid)
}

pub(crate) struct LayoutBuilder {
    slots: Vec<(Name, SlotKind)>,
    index: FxHashMap<Name, u32>,
}

impl LayoutBuilder {
    pub(crate) fn new() -> Self {
        LayoutBuilder {
            slots: Vec::new(),
            index: FxHashMap::default(),
        }
    }

    pub(crate) fn slot_for(&mut self, name: Name) -> Option<u32> {
        self.slot_of_kind(name, SlotKind::Mutable)
    }

    pub(crate) fn slot_of_kind(&mut self, name: Name, kind: SlotKind) -> Option<u32> {
        if let Some(slot) = self.index.get(&name) {
            return Some(*slot);
        }
        let slot = u32::try_from(self.slots.len()).ok()?;
        self.slots.push((name, kind));
        self.index.insert(name, slot);
        Some(slot)
    }

    pub(crate) fn finish(self) -> Option<Rc<FrameLayout>> {
        block_layout(self.slots)
    }
}

pub(crate) fn block_layout(slots: Vec<(Name, SlotKind)>) -> Option<Rc<FrameLayout>> {
    let layout = Rc::new(FrameLayout::new(slots)?);
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::note_frame_width(|| layout.len());
    Some(layout)
}

pub(crate) struct Hoisted<'a> {
    pub(crate) vars: std::ops::Range<u32>,
    pub(crate) functions: Box<[(u32, &'a Function<'a>)]>,
    pub(crate) count: usize,
    pub(crate) refused: Option<Bail>,
    pub(crate) shadows: bool,
}

pub(crate) struct FunctionLayout<'a> {
    pub(crate) layout: Rc<FrameLayout>,
    pub(crate) params: Vec<Option<u32>>,
    pub(crate) hoisted: Hoisted<'a>,
    pub(crate) lexicals: Box<[(usize, u32)]>,
}

pub(crate) fn function_layout<'a>(
    names: &mut Names<'a>,
    params: &FormalParameters<'a>,
    arguments: Option<Name>,
    body: Option<&'a [Statement<'a>]>,
) -> Option<FunctionLayout<'a>> {
    let mut builder = LayoutBuilder::new();
    let mut param_slots = Vec::with_capacity(params.items.len());
    for param in &params.items {
        param_slots.push(match &param.pattern {
            BindingPattern::BindingIdentifier(id) => {
                Some(builder.slot_for(names.intern(id.name.as_str()))?)
            }
            _ => None,
        });
    }
    if let Some(arguments) = arguments {
        builder.slot_for(arguments)?;
    }
    let body = body.unwrap_or_default();
    let hoisted = hoist_layout(names, &mut builder, body)?;
    let lexicals = super::lexical::scan(names, &mut builder, body)?;
    Some(FunctionLayout {
        layout: builder.finish()?,
        params: param_slots,
        hoisted,
        lexicals,
    })
}

fn hoist_layout<'a>(
    names: &mut Names<'a>,
    builder: &mut LayoutBuilder,
    body: &'a [Statement<'a>],
) -> Option<Hoisted<'a>> {
    let mut texts = Vec::new();
    let first = u32::try_from(builder.slots.len()).ok()?;
    if hoist_var_names(body, &mut texts).is_err() {
        return Some(Hoisted {
            vars: first..first,
            functions: Box::default(),
            count: 0,
            refused: Some(Bail::NestingDepth),
            shadows: false,
        });
    }
    let mut count = texts.len();
    let mut shadows = false;
    for text in texts {
        shadows |= builder.slot_for(names.intern(text))? < first;
    }
    let vars = first..u32::try_from(builder.slots.len()).ok()?;
    let mut functions = Vec::new();
    for (text, function) in hoist_function_declarations(body) {
        count += 1;
        let slot = builder.slot_for(names.intern(text))?;
        shadows |= slot < first;
        functions.push((slot, function));
    }
    Some(Hoisted {
        vars,
        functions: functions.into_boxed_slice(),
        count,
        refused: None,
        shadows,
    })
}

pub(crate) fn self_name_layout(name: Name) -> Option<Rc<FrameLayout>> {
    block_layout(vec![(name, SlotKind::Immutable)])
}

pub(crate) struct PreludeHoist<'a> {
    pub(crate) vars: Box<[u32]>,
    pub(crate) functions: Box<[(u32, &'a Function<'a>)]>,
    pub(crate) count: usize,
}

pub(crate) struct ChecksumLayout<'a> {
    pub(crate) layout: Rc<FrameLayout>,
    pub(crate) array: u32,
    pub(crate) expected: u32,
    pub(crate) hoists: Box<[PreludeHoist<'a>]>,
    pub(crate) lexicals: Box<[(usize, u32)]>,
}

pub(crate) fn checksum_layout<'a>(
    names: &mut Names<'a>,
    (array_param, expected_param): (&'a str, &'a str),
    prelude: &'a [Statement<'a>],
) -> Option<ChecksumLayout<'a>> {
    let mut builder = LayoutBuilder::new();
    let array = builder.slot_for(names.intern(array_param))?;
    let expected = builder.slot_for(names.intern(expected_param))?;
    let hoists = prelude
        .iter()
        .map(|statement| prelude_hoist(names, &mut builder, statement))
        .collect::<Option<Box<[PreludeHoist<'a>]>>>()?;
    let lexicals = super::lexical::scan(names, &mut builder, prelude)?;
    Some(ChecksumLayout {
        layout: builder.finish()?,
        array,
        expected,
        hoists,
        lexicals,
    })
}

fn prelude_hoist<'a>(
    names: &mut Names<'a>,
    builder: &mut LayoutBuilder,
    statement: &'a Statement<'a>,
) -> Option<PreludeHoist<'a>> {
    let statements = std::slice::from_ref(statement);
    let mut texts = Vec::new();
    hoist_var_names(statements, &mut texts).ok()?;
    let mut count = texts.len();
    let vars = texts
        .into_iter()
        .map(|text| builder.slot_for(names.intern(text)))
        .collect::<Option<Box<[u32]>>>()?;
    let mut functions = Vec::new();
    for (text, function) in hoist_function_declarations(statements) {
        count += 1;
        functions.push((builder.slot_for(names.intern(text))?, function));
    }
    Some(PreludeHoist {
        vars,
        functions: functions.into_boxed_slice(),
        count,
    })
}
