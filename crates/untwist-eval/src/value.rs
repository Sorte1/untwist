use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::{Equivalent, IndexMap};
use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrowFunctionExpression, Function, StringLiteral};
use oxc_str::{JSStr, JSStrBuilder};
use rustc_hash::FxBuildHasher;

use crate::bail::{Bail, EvalResult};
use crate::budget::{STRING_LEN_CAP, STRING_UNIT_BYTES};
use crate::frame::FrameRef;
use crate::heap::Charge;
use crate::names::Name;

pub use crate::bail::{Abrupt, AbruptKind, ErrorKind, Eval};

struct StrInner {
    units: Box<[u16]>,
    charge: Option<Box<Charge>>,
}

#[derive(Clone)]
pub struct JsString(Rc<StrInner>);

#[derive(Clone)]
pub(crate) struct ReservedString(std::rc::Weak<StrInner>);

impl ReservedString {
    pub(crate) fn upgrade(&self) -> Option<JsString> {
        self.0.upgrade().map(JsString)
    }

    #[cfg(feature = "test-hooks")]
    pub(crate) fn identity(&self) -> usize {
        std::rc::Weak::as_ptr(&self.0) as *const () as usize
    }

    pub(crate) fn holds_units(&self, units: &[u16]) -> bool {
        self.0.upgrade().is_some_and(|inner| *inner.units == *units)
    }
}

impl PartialEq for JsString {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.units() == other.units()
    }
}

impl Eq for JsString {}

impl Hash for JsString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.units().hash(state);
    }
}

impl JsString {
    pub fn empty() -> Self {
        Self::from_units(Vec::new())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        let units = match text.is_ascii() {
            true => text.bytes().map(u16::from).collect::<Vec<u16>>(),
            false => text.encode_utf16().collect::<Vec<u16>>(),
        };
        Self::from_units(units)
    }

    pub fn from_units(units: Vec<u16>) -> Self {
        Self::charged(units, None)
    }

    pub fn charged(units: Vec<u16>, charge: Option<Charge>) -> Self {
        Self(Rc::new(StrInner {
            units: units.into_boxed_slice(),
            charge: charge.map(Box::new),
        }))
    }

    pub fn is_unshared(&self) -> bool {
        Rc::strong_count(&self.0) == 1 && self.0.charge.is_none()
    }

    pub fn charged_bytes(&self) -> u64 {
        self.0.charge.as_ref().map_or(0, |charge| charge.bytes())
    }

    pub(crate) fn is_charged_on(&self, ledger: &Rc<crate::heap::HeapLedger>) -> bool {
        self.0
            .charge
            .as_ref()
            .is_some_and(|charge| charge.is_on(ledger))
    }

    pub fn units(&self) -> &[u16] {
        &self.0.units
    }

    pub fn len(&self) -> usize {
        self.0.units.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.units.is_empty()
    }

    pub fn code_unit_at(&self, index: usize) -> Option<u16> {
        self.0.units.get(index).copied()
    }

    pub fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }

    pub(crate) fn reserve(&self) -> ReservedString {
        ReservedString(Rc::downgrade(&self.0))
    }

    pub fn to_lossy_string(&self) -> String {
        String::from_utf16_lossy(self.units())
    }

    pub fn as_string(&self) -> Option<String> {
        let units = self.units();
        if units.iter().all(|unit| *unit < 0x80) {
            return String::from_utf8(units.iter().map(|unit| *unit as u8).collect()).ok();
        }
        String::from_utf16(units).ok()
    }

    pub fn is_well_formed(&self) -> bool {
        char::decode_utf16(self.units().iter().copied()).all(|unit| unit.is_ok())
    }

    pub fn eq_str(&self, other: &str) -> bool {
        char::decode_utf16(self.units().iter().copied())
            .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
            .eq(other.chars())
    }

    pub fn concat(&self, other: &Self) -> EvalResult<Self> {
        self.concat_charged(other, None)
    }

    pub fn concat_charged(&self, other: &Self, charge: Option<Charge>) -> EvalResult<Self> {
        let total = self.len() + other.len();
        if total > STRING_LEN_CAP {
            return Err(Bail::StringLength);
        }
        let mut units = Vec::with_capacity(total);
        units.extend_from_slice(self.units());
        units.extend_from_slice(other.units());
        Ok(Self::charged(units, charge))
    }
}

pub fn error_value<'a>(kind: ErrorKind, message: &str) -> Value<'a> {
    let error = Value::object(ObjKind::Error(kind), None);
    if let Some(obj) = error.as_object() {
        obj.borrow_mut().props.insert(
            StoredKey::named("message", Attrs::BUILTIN),
            Value::Str(JsString::from_str(message)),
        );
    }
    error
}

#[derive(Default)]
pub struct Utf16Builder {
    units: Vec<u16>,
    charge: Option<Charge>,
}

const BUILDER_MIN_UNITS: usize = 8;

impl Utf16Builder {
    pub fn charged(charge: Option<Charge>) -> Self {
        Self {
            units: Vec::new(),
            charge,
        }
    }

    pub fn push(&mut self, text: &JsString) -> EvalResult<()> {
        let needed = self.units.len() + text.len();
        if needed > STRING_LEN_CAP {
            return Err(Bail::StringLength);
        }
        self.reserve_charged(needed)?;
        self.units.extend_from_slice(text.units());
        Ok(())
    }

    fn reserve_charged(&mut self, needed: usize) -> EvalResult<()> {
        let capacity = self.units.capacity();
        let Some(charge) = &self.charge else {
            return Ok(());
        };
        if needed <= capacity {
            return Ok(());
        }
        let target = needed
            .max(capacity.saturating_mul(2))
            .max(BUILDER_MIN_UNITS);
        charge.grow(((target - capacity) as u64).saturating_mul(STRING_UNIT_BYTES))?;
        self.units.reserve_exact(target - self.units.len());
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.units.len()
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    pub fn finish(self) -> JsString {
        if let Some(charge) = &self.charge {
            let held = (self.units.capacity() as u64).saturating_mul(STRING_UNIT_BYTES);
            let kept = (self.units.len() as u64).saturating_mul(STRING_UNIT_BYTES);
            charge.shrink(held - kept);
        }
        JsString::charged(self.units, self.charge)
    }
}

impl fmt::Debug for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JsString({:?})", self.to_lossy_string())
    }
}

impl From<&str> for JsString {
    fn from(text: &str) -> Self {
        Self::from_str(text)
    }
}

pub type ObjRef<'a> = Rc<RefCell<JsObject<'a>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IntrinsicProto {
    Object,
    Array,
    Function,
    String,
    Number,
    RegExp,
}

impl IntrinsicProto {
    pub fn bit(self) -> u8 {
        match self {
            Self::Object => 1 << 0,
            Self::Array => 1 << 1,
            Self::Function => 1 << 2,
            Self::String => 1 << 3,
            Self::Number => 1 << 4,
            Self::RegExp => 1 << 5,
        }
    }

    pub fn from_bit(bit: u8) -> Option<Self> {
        match bit {
            1 => Some(Self::Object),
            2 => Some(Self::Array),
            4 => Some(Self::Function),
            8 => Some(Self::String),
            16 => Some(Self::Number),
            32 => Some(Self::RegExp),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeId(pub u16);

#[derive(Clone, Debug)]
#[repr(C, u8)]
pub enum Value<'a> {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    Str(JsString),
    Object(ObjRef<'a>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attrs(u8);

impl Attrs {
    const READONLY: u8 = 1 << 0;
    const HIDDEN: u8 = 1 << 1;
    const FIXED: u8 = 1 << 2;

    pub const DATA: Self = Self(0);
    pub const BUILTIN: Self = Self(Self::HIDDEN);
    pub const FUNCTION_NAME: Self = Self(Self::READONLY | Self::HIDDEN);
    pub const CONSTANT: Self = Self(Self::READONLY | Self::HIDDEN | Self::FIXED);
    pub const PROTOTYPE: Self = Self(Self::HIDDEN | Self::FIXED);

    pub const fn new(writable: bool, enumerable: bool, configurable: bool) -> Self {
        let mut bits = 0;
        if !writable {
            bits |= Self::READONLY;
        }
        if !enumerable {
            bits |= Self::HIDDEN;
        }
        if !configurable {
            bits |= Self::FIXED;
        }
        Self(bits)
    }

    pub const fn writable(self) -> bool {
        self.0 & Self::READONLY == 0
    }

    pub const fn enumerable(self) -> bool {
        self.0 & Self::HIDDEN == 0
    }

    pub const fn configurable(self) -> bool {
        self.0 & Self::FIXED == 0
    }

    pub const fn frozen(self) -> Self {
        Self(self.0 | Self::READONLY | Self::FIXED)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropKey {
    Index(u32),
    Name(JsString),
}

#[derive(Clone, Debug)]
pub enum StoredKey {
    Index(u32, Attrs),
    Name(JsString, Attrs),
}

impl StoredKey {
    pub fn new(key: PropKey, attrs: Attrs) -> Self {
        match key {
            PropKey::Index(index) => Self::Index(index, attrs),
            PropKey::Name(name) => Self::Name(name, attrs),
        }
    }

    pub fn named(name: &str, attrs: Attrs) -> Self {
        Self::Name(JsString::from_str(name), attrs)
    }

    pub fn key(&self) -> PropKey {
        match self {
            Self::Index(index, _) => PropKey::Index(*index),
            Self::Name(name, _) => PropKey::Name(name.clone()),
        }
    }

    pub fn attrs(&self) -> Attrs {
        match self {
            Self::Index(_, attrs) | Self::Name(_, attrs) => *attrs,
        }
    }

    pub fn set_attrs(&mut self, value: Attrs) {
        match self {
            Self::Index(_, attrs) | Self::Name(_, attrs) => *attrs = value,
        }
    }
}

impl PartialEq for StoredKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Index(left, _), Self::Index(right, _)) => left == right,
            (Self::Name(left, _), Self::Name(right, _)) => left == right,
            _ => false,
        }
    }
}

impl Eq for StoredKey {}

impl Hash for StoredKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Index(index, _) => {
                0isize.hash(state);
                index.hash(state);
            }
            Self::Name(name, _) => {
                1isize.hash(state);
                name.hash(state);
            }
        }
    }
}

impl From<PropKey> for StoredKey {
    #[inline(never)]
    fn from(key: PropKey) -> Self {
        Self::new(key, Attrs::DATA)
    }
}

impl Equivalent<StoredKey> for PropKey {
    fn equivalent(&self, stored: &StoredKey) -> bool {
        match (self, stored) {
            (Self::Index(left), StoredKey::Index(right, _)) => left == right,
            (Self::Name(left), StoredKey::Name(right, _)) => left == right,
            _ => false,
        }
    }
}

pub type PropMap<'a> = IndexMap<StoredKey, Value<'a>, FxBuildHasher>;

#[derive(Debug)]
pub struct JsObject<'a> {
    pub kind: ObjKind<'a>,
    pub props: PropMap<'a>,
    pub proto: Option<ObjRef<'a>>,
    pub charge: Option<Box<Charge>>,
}

impl<'a> JsObject<'a> {
    pub fn trace_tag(&self) -> u8 {
        match &self.kind {
            ObjKind::Plain(tag) => *tag,
            ObjKind::Native(_, tag) => *tag,
            _ => 0,
        }
    }

    pub fn set_trace_tag(&mut self, value: u8) {
        match &mut self.kind {
            ObjKind::Plain(tag) => *tag = value,
            ObjKind::Native(_, tag) => *tag = value,
            _ => {}
        }
    }
}

impl Drop for JsObject<'_> {
    fn drop(&mut self) {
        crate::state::note_object_drop(self);
        crate::release::release_object(self);
    }
}

#[derive(Debug)]
pub enum ObjKind<'a> {
    Plain(u8),
    Array(Vec<Value<'a>>),
    Arguments(Vec<Value<'a>>),
    MappedArguments(Box<MappedArguments<'a>>),
    Function(Rc<Closure<'a>>),
    Bound(Rc<BoundFunction<'a>>),
    Native(NativeId, u8),
    Error(ErrorKind),
    RegExp(Rc<RegExpData>),
}

impl ObjKind<'_> {
    pub fn is_arguments(&self) -> bool {
        matches!(self, ObjKind::Arguments(_) | ObjKind::MappedArguments(_))
    }
}

#[derive(Debug)]
pub struct MappedArguments<'a> {
    pub items: Vec<Value<'a>>,
    pub scope: FrameRef<'a>,
    pub map: Vec<Option<Name>>,
}

impl<'a> MappedArguments<'a> {
    pub fn mapped_name(&self, index: usize) -> Option<Name> {
        self.map.get(index).copied().flatten()
    }

    pub fn value_at(&self, index: usize) -> Option<Value<'a>> {
        let item = self.items.get(index)?;
        let bound = self
            .mapped_name(index)
            .and_then(|name| self.scope.borrow().body.get(name).cloned());
        Some(bound.unwrap_or_else(|| item.clone()))
    }

    pub fn current_items(&self) -> Vec<Value<'a>> {
        (0..self.items.len())
            .filter_map(|index| self.value_at(index))
            .collect()
    }

    pub fn write_at(&mut self, index: usize, value: Value<'a>) {
        if let Some(name) = self.mapped_name(index) {
            let previous = self
                .scope
                .borrow_mut()
                .body
                .get_mut(name)
                .map(|slot| std::mem::replace(slot, value.clone()));
            drop(previous);
        }
        if let Some(slot) = self.items.get_mut(index) {
            *slot = value;
        }
    }

    pub fn unmap(&mut self, index: usize) {
        if let Some(slot) = self.map.get_mut(index) {
            *slot = None;
        }
    }

    pub fn into_unmapped(mut self) -> Vec<Value<'a>> {
        let scope = self.scope.borrow();
        for (index, slot) in self.items.iter_mut().enumerate() {
            let bound = self
                .map
                .get(index)
                .copied()
                .flatten()
                .and_then(|name| scope.body.get(name));
            if let Some(value) = bound {
                *slot = value.clone();
            }
        }
        drop(scope);
        self.items
    }
}

#[derive(Debug)]
pub struct Closure<'a> {
    pub code: FnCode<'a>,
    pub env: FrameRef<'a>,
    pub lexical_this: Option<Box<Value<'a>>>,
    pub strict: bool,
    pub plain: bool,
}

impl Closure<'_> {
    pub fn class_tag(&self) -> &'static str {
        let (generator, asynchronous) = match self.code {
            FnCode::Function(function) => (function.generator, function.r#async),
            FnCode::Arrow(arrow) => (false, arrow.r#async),
        };
        match (generator, asynchronous) {
            (true, true) => "AsyncGeneratorFunction",
            (true, false) => "GeneratorFunction",
            (false, true) => "AsyncFunction",
            (false, false) => "Function",
        }
    }

    pub fn is_generator_or_async(&self) -> bool {
        match self.code {
            FnCode::Function(function) => function.generator || function.r#async,
            FnCode::Arrow(arrow) => arrow.r#async,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FnCode<'a> {
    Function(&'a Function<'a>),
    Arrow(&'a ArrowFunctionExpression<'a>),
}

#[derive(Debug)]
pub struct BoundFunction<'a> {
    pub target: Value<'a>,
    pub this: Value<'a>,
    pub args: Vec<Value<'a>>,
}

#[derive(Debug)]
pub struct RegExpData {
    pub source: JsString,
    pub flags: JsString,
    pub regex: regress::Regex,
    pub global: bool,
    pub ascii_subject_only: bool,
    pub last_index: Cell<usize>,
}

#[derive(Clone, Debug)]
pub enum Completion<'a> {
    Normal,
    Return(Value<'a>),
    Break(Option<&'a str>),
    Continue(Option<&'a str>),
}

impl<'a> Value<'a> {
    pub fn object(kind: ObjKind<'a>, proto: Option<ObjRef<'a>>) -> Self {
        Self::charged_object(kind, proto, None)
    }

    pub fn charged_object(
        kind: ObjKind<'a>,
        proto: Option<ObjRef<'a>>,
        charge: Option<Charge>,
    ) -> Self {
        Self::Object(Rc::new(RefCell::new(JsObject {
            kind,
            props: PropMap::default(),
            proto,
            charge: charge.map(Box::new),
        })))
    }

    pub fn array(items: Vec<Value<'a>>, proto: Option<ObjRef<'a>>) -> Self {
        Self::object(ObjKind::Array(items), proto)
    }

    pub fn string(text: &str) -> Self {
        Self::Str(JsString::from_str(text))
    }

    pub fn as_object(&self) -> Option<&ObjRef<'a>> {
        match self {
            Self::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn is_callable(&self) -> bool {
        let Some(obj) = self.as_object() else {
            return false;
        };
        matches!(
            obj.borrow().kind,
            ObjKind::Function(_) | ObjKind::Native(_, _) | ObjKind::Bound(_)
        )
    }

    pub fn is_nullish(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }
}

pub fn js_str_in<'a>(units: &[u16], allocator: &'a Allocator) -> JSStr<'a> {
    match String::from_utf16(units) {
        Ok(text) => JSStr::from_str_in(&text, &allocator),
        Err(_) => {
            let mut builder = JSStrBuilder::with_capacity_in(units.len() * 3, &allocator);
            builder.push_utf16(units);
            builder.into_js_str()
        }
    }
}

pub fn js_string_from_js_str(value: JSStr<'_>) -> JsString {
    match value.as_str() {
        Some(text) => JsString::from_str(text),
        None => JsString::from_units(value.encode_utf16().collect()),
    }
}

pub fn js_string_from_literal(literal: &StringLiteral<'_>) -> JsString {
    js_string_from_js_str(literal.value)
}

#[cfg(test)]
mod size_tests {
    use super::*;

    #[test]
    fn js_object_size_is_unchanged_by_the_state_monitor_mark() {
        assert_eq!(std::mem::size_of::<JsObject<'static>>(), 104);
    }
}
