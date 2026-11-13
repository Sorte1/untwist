pub(crate) mod args;
pub mod array;
mod charged;
mod implicit;

pub(crate) use charged::WIDE_SCOPE_BINDINGS;
pub mod function;
pub mod global;
pub mod json;
pub mod math;
pub mod number;
pub mod object;
mod pattern_hooks;
pub mod regexp;
pub mod regexp_syntax;
pub mod shape;
pub mod string;
pub mod string_match;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use rustc_hash::{FxHashMap, FxHashSet};

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::OBJECT_BYTES;
use crate::builtins::shape::Shape;
use crate::env::{charged_env, declare};
use crate::frame::{Frame, FrameLayout, FrameRef};
use crate::heap::{Charge, HeapLedger};
use crate::host::Host;
use crate::names::Names;
use crate::object::{lookup_inherited, lookup_property};
use crate::value::{
    Attrs, Eval, IntrinsicProto, JsObject, JsString, NativeId, ObjKind, ObjRef, PropKey, PropMap,
    StoredKey, Value,
};

const OBJECT_PROTO: usize = 0;
const ARRAY_PROTO: usize = 1;
const STRING_PROTO: usize = 2;
const NUMBER_PROTO: usize = 3;
const FUNCTION_PROTO: usize = 4;
const REGEXP_PROTO: usize = 5;

const REGISTRY_COMPACTION_FLOOR: usize = 1024;

pub type NativeFn<'a> = fn(&mut dyn Host<'a>, Value<'a>, &[Value<'a>]) -> Eval<'a, Value<'a>>;

#[derive(Default)]
pub struct NativeTable<'a> {
    entries: Vec<(NativeFn<'a>, &'static Shape)>,
    unlisted: usize,
}

impl<'a> NativeTable<'a> {
    pub fn register(&mut self, owner: &str, name: &str, function: NativeFn<'a>) -> NativeId {
        let shape = match shape::lookup(owner, name) {
            Some(shape) => shape,
            None => {
                self.unlisted += 1;
                &shape::REFUSED
            }
        };
        self.entries.push((function, shape));
        NativeId((self.entries.len() - 1) as u16)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn unlisted(&self) -> usize {
        self.unlisted
    }

    pub fn get(&self, id: NativeId) -> Option<(NativeFn<'a>, &'static Shape)> {
        self.entries.get(id.0 as usize).copied()
    }
}

pub const OUTSIDE_SHAPE: &str =
    "a native call outside the argument shape the evaluator models exactly";

pub const BAILING_FUNCTIONS: [&str; 10] = [
    "eval",
    "Function",
    "setTimeout",
    "setInterval",
    "setImmediate",
    "requestAnimationFrame",
    "Date",
    "fetch",
    "XMLHttpRequest",
    "require",
];

pub const HOSTILE_OBJECTS: [&str; 4] = ["process", "document", "window", "navigator"];

#[derive(Default)]
pub struct Registry<'a> {
    objects: Vec<Weak<RefCell<JsObject<'a>>>>,
    scopes: Vec<Weak<RefCell<Frame<'a>>>>,
    next_compaction: usize,
}

impl<'a> Registry<'a> {
    fn track_object(&mut self, obj: &ObjRef<'a>) {
        self.objects.push(Rc::downgrade(obj));
        self.compact_if_due();
    }

    fn track_scope(&mut self, env: &FrameRef<'a>) {
        self.scopes.push(Rc::downgrade(env));
        self.compact_if_due();
    }

    fn compact_if_due(&mut self) {
        let tracked = self.objects.len() + self.scopes.len();
        if tracked < self.next_compaction.max(REGISTRY_COMPACTION_FLOOR) {
            return;
        }
        self.objects.retain(|weak| weak.strong_count() > 0);
        self.scopes.retain(|weak| weak.strong_count() > 0);
        self.next_compaction = (self.objects.len() + self.scopes.len()) * 2;
    }

    fn teardown(&mut self) {
        let objects: Vec<ObjRef<'a>> = self
            .objects
            .drain(..)
            .filter_map(|weak| weak.upgrade())
            .collect();
        let scopes: Vec<FrameRef<'a>> = self
            .scopes
            .drain(..)
            .filter_map(|weak| weak.upgrade())
            .collect();
        for scope in &scopes {
            if let Ok(mut scope) = scope.try_borrow_mut() {
                scope.body.clear();
                scope.parent = None;
            }
        }
        for obj in &objects {
            if let Ok(mut obj) = obj.try_borrow_mut() {
                obj.props.clear();
                obj.proto = None;
                obj.kind = ObjKind::Plain(0);
            }
        }
    }
}

const NAMESPACE_SLOTS: usize = 8;

fn namespace_objects<'a>(globals: &Globals<'a>) -> [Option<ObjRef<'a>>; NAMESPACE_SLOTS] {
    let mut slots: [Option<ObjRef<'a>>; NAMESPACE_SLOTS] = Default::default();
    let plain = globals
        .values
        .values()
        .filter_map(Value::as_object)
        .filter(|obj| matches!(obj.borrow().kind, ObjKind::Plain(_)));
    let mut count = 0;
    for (slot, obj) in slots.iter_mut().zip(plain) {
        *slot = Some(obj.clone());
        count += 1;
    }
    debug_assert!(
        count < NAMESPACE_SLOTS,
        "every namespace global has a slot, with one to spare"
    );
    slots
}

fn tracked_object<'a>(
    registry: &RefCell<Registry<'a>>,
    kind: ObjKind<'a>,
    proto: Option<ObjRef<'a>>,
    charge: Option<Charge>,
) -> Value<'a> {
    let value = Value::charged_object(kind, proto, charge);
    if let Some(obj) = value.as_object() {
        registry.borrow_mut().track_object(obj);
    }
    value
}

fn tracked_plain<'a>(registry: &RefCell<Registry<'a>>, trace_tag: u8) -> ObjRef<'a> {
    let obj: ObjRef<'a> = Rc::new(RefCell::new(JsObject {
        kind: ObjKind::Plain(trace_tag),
        props: PropMap::default(),
        proto: None,
        charge: None,
    }));
    registry.borrow_mut().track_object(&obj);
    obj
}

#[derive(Default)]
pub struct Globals<'a> {
    values: FxHashMap<&'static str, Value<'a>>,
}

impl<'a> Globals<'a> {
    pub fn define(&mut self, name: &'static str, value: Value<'a>) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value<'a>> {
        self.values.get(name).cloned()
    }

    pub fn objects(&self) -> Vec<(ObjRef<'a>, &'static str)> {
        self.values
            .iter()
            .filter_map(|(name, value)| value.as_object().map(|obj| (obj.clone(), *name)))
            .collect()
    }

    pub fn install(self, env: &FrameRef<'a>, names: &mut Names<'a>) {
        for (text, value) in self.values {
            if untwist_core::semantic::is_foreign_global(text) {
                continue;
            }
            declare(env, names.intern(text), value);
        }
    }
}

pub struct Realm<'a> {
    registry: RefCell<Registry<'a>>,
    natives: Rc<NativeTable<'a>>,
    protos: [ObjRef<'a>; 6],
    constructible: Vec<NativeId>,
    hostile_objects: Vec<(ObjRef<'a>, &'static str)>,
    global_object: Option<ObjRef<'a>>,
    namespaces: [Option<ObjRef<'a>>; NAMESPACE_SLOTS],
    bail_id: NativeId,
    pure_natives: FxHashSet<NativeId>,
    locally_pure_natives: FxHashSet<NativeId>,
    heap: Option<Rc<HeapLedger>>,
    units: charged::UnitStrings,
}

#[inline(never)]
pub(crate) fn define_prop<'a>(obj: &ObjRef<'a>, key: StoredKey, value: Value<'a>) {
    obj.borrow_mut().props.insert(key, value);
}

#[inline(never)]
pub(crate) fn define_named<'a>(obj: &ObjRef<'a>, name: &str, attrs: Attrs, value: Value<'a>) {
    define_prop(obj, StoredKey::named(name, attrs), value);
}

pub(crate) fn define_builtin<'a>(obj: &ObjRef<'a>, name: &str, value: Value<'a>) {
    define_named(obj, name, Attrs::BUILTIN, value);
}

pub(crate) fn define_constant<'a>(obj: &ObjRef<'a>, name: &str, value: Value<'a>) {
    define_named(obj, name, Attrs::CONSTANT, value);
}

pub(crate) fn native_hostile_name(obj: &ObjRef<'_>) -> String {
    match lookup_property(obj, &PropKey::Name(JsString::from_str("name"))) {
        Ok(Some(Value::Str(text))) => text.to_lossy_string(),
        _ => "<unknown>".to_string(),
    }
}

fn native_bail<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Err(Bail::hostile_global("<unknown>").into())
}

static ALL_ROOT_NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();

pub fn all_root_names() -> &'static [&'static str] {
    ALL_ROOT_NAMES
        .get_or_init(|| {
            let (_realm, globals) = Realm::with_globals();
            globals
                .objects()
                .into_iter()
                .map(|(_, name)| name)
                .collect()
        })
        .as_slice()
}

impl<'a> Realm<'a> {
    pub fn with_globals() -> (Self, Globals<'a>) {
        let mut table = NativeTable::default();
        let bail_id = table.register("hostile", "", native_bail);
        let registry = RefCell::new(Registry::default());
        let object_proto = tracked_plain(&registry, IntrinsicProto::Object.bit());
        let array_proto = tracked_plain(&registry, IntrinsicProto::Array.bit());
        let string_proto = tracked_plain(&registry, IntrinsicProto::String.bit());
        let number_proto = tracked_plain(&registry, IntrinsicProto::Number.bit());
        let function_proto = tracked_plain(&registry, IntrinsicProto::Function.bit());
        let regexp_proto = tracked_plain(&registry, IntrinsicProto::RegExp.bit());
        for linked in [&array_proto, &function_proto, &regexp_proto] {
            linked.borrow_mut().proto = Some(object_proto.clone());
        }
        let mut realm = Self {
            registry,
            natives: Rc::new(NativeTable::default()),
            protos: [
                object_proto,
                array_proto,
                string_proto,
                number_proto,
                function_proto,
                regexp_proto,
            ],
            constructible: Vec::new(),
            hostile_objects: Vec::new(),
            global_object: None,
            namespaces: Default::default(),
            bail_id,
            pure_natives: FxHashSet::default(),
            locally_pure_natives: FxHashSet::default(),
            heap: None,
            units: charged::UnitStrings::default(),
        };
        let mut globals = Globals::default();
        realm.install_values(&mut globals);
        realm.install_bailing(bail_id, &mut globals);
        global::install(&mut realm, &mut globals, &mut table);
        string::install(&mut realm, &mut globals, &mut table);
        array::install(&mut realm, &mut table);
        number::install(&mut realm, &mut globals, &mut table);
        math::install(&mut realm, &mut globals, &mut table);
        object::install(&mut realm, &mut globals, &mut table);
        json::install(&mut realm, &mut globals, &mut table);
        function::install(&mut realm, &mut table);
        regexp::install(&mut realm, &mut globals, &mut table);
        realm.natives = Rc::new(table);
        realm.link_constructors(&globals);
        realm.namespaces = namespace_objects(&globals);
        (realm, globals)
    }

    fn link_constructors(&self, globals: &Globals<'a>) {
        if let Some(string_global) = globals.get("String") {
            define_builtin(&self.protos[STRING_PROTO], "constructor", string_global);
        }
        let constructor_prototypes: [(&str, &ObjRef<'a>); 4] = [
            ("Object", &self.protos[OBJECT_PROTO]),
            ("String", &self.protos[STRING_PROTO]),
            ("Number", &self.protos[NUMBER_PROTO]),
            ("RegExp", &self.protos[REGEXP_PROTO]),
        ];
        for (name, proto) in constructor_prototypes {
            let Some(constructor) = globals.get(name).and_then(|v| v.as_object().cloned()) else {
                continue;
            };
            define_constant(&constructor, "prototype", Value::Object(proto.clone()));
        }
    }

    pub fn new_object(&self, kind: ObjKind<'a>, proto: Option<ObjRef<'a>>) -> Value<'a> {
        tracked_object(&self.registry, kind, proto, self.heap_charge(OBJECT_BYTES))
    }

    pub fn new_array(&self, items: Vec<Value<'a>>, proto: Option<ObjRef<'a>>) -> Value<'a> {
        self.new_object(ObjKind::Array(items), proto)
    }

    pub fn new_env(&self, parent: Option<&FrameRef<'a>>) -> FrameRef<'a> {
        let env = charged_env(parent, None);
        self.registry.borrow_mut().track_scope(&env);
        env
    }

    pub(crate) fn new_fixed_env(
        &self,
        layout: &Rc<FrameLayout>,
        parent: &FrameRef<'a>,
    ) -> FrameRef<'a> {
        let env = Frame::fixed(layout, Some(parent), None);
        self.registry.borrow_mut().track_scope(&env);
        env
    }

    #[cfg(feature = "test-hooks")]
    pub fn tracked_frames(&self) -> usize {
        self.registry.borrow().scopes.len()
    }

    #[cfg(feature = "test-hooks")]
    pub fn live_tracked(&self) -> (usize, usize) {
        let registry = self.registry.borrow();
        let objects = registry
            .objects
            .iter()
            .filter(|weak| weak.strong_count() > 0);
        let frames = registry
            .scopes
            .iter()
            .filter(|weak| weak.strong_count() > 0);
        (objects.count(), frames.count())
    }

    pub fn track(&self, obj: &ObjRef<'a>) {
        self.registry.borrow_mut().track_object(obj);
    }

    pub fn error_value(&self, kind: ErrorKind, message: &str) -> Value<'a> {
        let _ = message;
        self.new_object(ObjKind::Error(kind), None)
    }

    fn install_values(&mut self, globals: &mut Globals<'a>) {
        globals.define("undefined", Value::Undefined);
        globals.define("NaN", Value::Number(f64::NAN));
        globals.define("Infinity", Value::Number(f64::INFINITY));
        let global_this = self.new_object(ObjKind::Plain(0), None);
        self.global_object = global_this.as_object().cloned();
        globals.define("globalThis", global_this);
    }

    fn install_bailing(&mut self, bail_id: NativeId, globals: &mut Globals<'a>) {
        let name_key = StoredKey::named("name", Attrs::FUNCTION_NAME);
        for name in BAILING_FUNCTIONS {
            let hostile = self.new_object(
                ObjKind::Native(bail_id, 0),
                Some(self.protos[FUNCTION_PROTO].clone()),
            );
            if let Some(obj) = hostile.as_object() {
                define_prop(obj, name_key.clone(), Value::Str(JsString::from_str(name)));
            }
            globals.define(name, hostile);
        }
        for name in HOSTILE_OBJECTS {
            let hostile = self.new_object(ObjKind::Plain(0), None);
            if let Some(obj) = hostile.as_object() {
                self.hostile_objects.push((obj.clone(), name));
            }
            globals.define(name, hostile);
        }
        let Some(function_global) = globals.get("Function") else {
            return;
        };
        define_builtin(&self.protos[FUNCTION_PROTO], "constructor", function_global);
        define_builtin(&self.protos[OBJECT_PROTO], "constructor", Value::Undefined);
    }

    #[inline(never)]
    pub fn native_value(
        &self,
        table: &mut NativeTable<'a>,
        (owner, name): (&str, &str),
        function: NativeFn<'a>,
    ) -> Value<'a> {
        let id = table.register(owner, name, function);
        self.new_object(
            ObjKind::Native(id, 0),
            Some(self.protos[FUNCTION_PROTO].clone()),
        )
    }

    #[inline(never)]
    pub fn native_pure_value(
        &mut self,
        table: &mut NativeTable<'a>,
        path: (&str, &str),
        function: NativeFn<'a>,
    ) -> Value<'a> {
        let value = self.native_value(table, path, function);
        self.mark_pure_value(&value);
        value
    }

    pub fn natives(&self) -> Rc<NativeTable<'a>> {
        self.natives.clone()
    }

    pub fn is_intrinsic_prototype(&self, obj: &ObjRef<'a>) -> bool {
        [
            &self.protos[OBJECT_PROTO],
            &self.protos[ARRAY_PROTO],
            &self.protos[FUNCTION_PROTO],
            &self.protos[REGEXP_PROTO],
            &self.protos[STRING_PROTO],
            &self.protos[NUMBER_PROTO],
        ]
        .into_iter()
        .any(|proto| Rc::ptr_eq(proto, obj))
    }

    pub fn compact(&self, globals: &Globals<'a>) {
        for obj in self
            .protos
            .iter()
            .cloned()
            .chain(globals.objects().into_iter().map(|(obj, _)| obj))
        {
            obj.borrow_mut().props.shrink_to_fit();
        }
    }

    pub fn object_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[OBJECT_PROTO].clone())
    }

    pub fn array_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[ARRAY_PROTO].clone())
    }

    pub fn function_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[FUNCTION_PROTO].clone())
    }

    pub fn regexp_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[REGEXP_PROTO].clone())
    }

    pub fn string_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[STRING_PROTO].clone())
    }

    pub fn number_prototype(&self) -> Option<ObjRef<'a>> {
        Some(self.protos[NUMBER_PROTO].clone())
    }

    pub fn string_prototype_member(&self, key: &PropKey) -> EvalResult<Value<'a>> {
        if let Some(found) = lookup_inherited(&self.protos[STRING_PROTO], key)? {
            return Ok(found);
        }
        Ok(lookup_inherited(&self.protos[OBJECT_PROTO], key)?.unwrap_or(Value::Undefined))
    }

    pub fn number_prototype_member(&self, key: &PropKey) -> EvalResult<Value<'a>> {
        if let Some(found) = lookup_inherited(&self.protos[NUMBER_PROTO], key)? {
            return Ok(found);
        }
        Ok(lookup_inherited(&self.protos[OBJECT_PROTO], key)?.unwrap_or(Value::Undefined))
    }

    pub fn intrinsic_member(
        &self,
        target: &Value<'a>,
        key: &PropKey,
    ) -> EvalResult<Option<Value<'a>>> {
        if target.is_callable() {
            if let Some(found) = lookup_inherited(&self.protos[FUNCTION_PROTO], key)? {
                return Ok(Some(found));
            }
        }
        let Some(obj) = target.as_object() else {
            return Ok(None);
        };
        let own_proto = match &obj.borrow().kind {
            ObjKind::Array(_) | ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => {
                Some(ARRAY_PROTO)
            }
            ObjKind::RegExp(_) => Some(REGEXP_PROTO),
            ObjKind::Error(_) => {
                crate::unmodelled::refuse_error_prototype_member(key)?;
                None
            }
            _ => None,
        };
        if let Some(index) = own_proto {
            if let Some(found) = lookup_inherited(&self.protos[index], key)? {
                return Ok(Some(found));
            }
        }
        lookup_inherited(&self.protos[OBJECT_PROTO], key)
    }

    pub fn supplied_prototypes_hold_enumerable_keys(&self, target: &Value<'a>) -> bool {
        let by_kind = match target {
            Value::Str(_) => Some(STRING_PROTO),
            Value::Number(_) => Some(NUMBER_PROTO),
            Value::Object(obj) => match &obj.borrow().kind {
                ObjKind::Array(_) | ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => {
                    Some(ARRAY_PROTO)
                }
                ObjKind::RegExp(_) => Some(REGEXP_PROTO),
                ObjKind::Function(_) | ObjKind::Bound(_) | ObjKind::Native(_, _) => {
                    Some(FUNCTION_PROTO)
                }
                _ => None,
            },
            _ => None,
        };
        by_kind
            .into_iter()
            .chain([OBJECT_PROTO])
            .any(|index| crate::object::has_enumerable_own_key(&self.protos[index]))
    }

    pub fn intrinsic_constructor(&self, callee: &Value<'a>) -> Option<NativeId> {
        let obj = callee.as_object()?;
        let borrowed = obj.borrow();
        let ObjKind::Native(id, _) = &borrowed.kind else {
            return None;
        };
        self.constructible.contains(id).then_some(*id)
    }

    pub fn mark_constructible(&mut self, id: NativeId) {
        self.constructible.push(id);
    }

    pub fn is_bailing_native(&self, id: NativeId) -> bool {
        self.bail_id == id
    }

    pub fn refuse_global_object(&self, obj: &ObjRef<'a>) -> EvalResult<()> {
        if self
            .global_object
            .as_ref()
            .is_some_and(|global| Rc::ptr_eq(global, obj))
        {
            return Err(Bail::Unsupported(
                &"a member of the global object, which the evaluator does not model",
            ));
        }
        Ok(())
    }

    pub fn refuse_namespace(&self, obj: &ObjRef<'a>) -> EvalResult<()> {
        if self
            .namespaces
            .iter()
            .flatten()
            .any(|namespace| Rc::ptr_eq(namespace, obj))
        {
            return Err(Bail::Unsupported(
                &"an own key or tag of a namespace object, which the evaluator does not model",
            ));
        }
        Ok(())
    }

    #[inline]
    pub(crate) fn refused_write_target(&self, obj: &ObjRef<'a>, key: &PropKey) -> Option<Bail> {
        if let Some(name) = self.hostile_name(obj) {
            return Some(Bail::hostile_global(name));
        }
        if crate::object::is_proto_key(key) {
            return crate::object::refuse_proto_write(key).err();
        }
        self.refuse_global_object(obj).err()
    }

    pub fn hostile_name(&self, obj: &ObjRef<'a>) -> Option<String> {
        match obj.borrow().kind {
            ObjKind::Native(id, _) if self.bail_id == id => return Some(native_hostile_name(obj)),
            ObjKind::Plain(_) => {}
            _ => return None,
        }
        self.hostile_objects
            .iter()
            .find(|(candidate, _)| Rc::ptr_eq(candidate, obj))
            .map(|(_, name)| (*name).to_string())
    }

    pub fn mark_pure(&mut self, id: NativeId) {
        self.pure_natives.insert(id);
    }

    pub fn is_pure_native(&self, id: NativeId) -> bool {
        self.pure_natives.contains(&id)
    }

    #[inline(never)]
    pub fn mark_pure_value(&mut self, value: &Value<'a>) {
        let Some(obj) = value.as_object() else {
            return;
        };
        if let ObjKind::Native(id, _) = obj.borrow().kind {
            self.mark_pure(id);
        }
    }

    #[inline(never)]
    pub fn mark_locally_pure_value(&mut self, value: &Value<'a>) {
        let Some(obj) = value.as_object() else {
            return;
        };
        if let ObjKind::Native(id, _) = obj.borrow().kind {
            self.locally_pure_natives.insert(id);
        }
    }

    pub fn is_locally_pure_native(&self, id: NativeId) -> bool {
        self.locally_pure_natives.contains(&id)
    }

    pub fn define_string_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[STRING_PROTO], name, value);
    }

    pub fn define_number_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[NUMBER_PROTO], name, value);
    }

    pub fn define_array_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[ARRAY_PROTO], name, value);
    }

    pub fn define_function_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[FUNCTION_PROTO], name, value);
    }

    pub fn define_object_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[OBJECT_PROTO], name, value);
    }

    pub fn define_regexp_member(&mut self, name: &str, value: Value<'a>) {
        define_builtin(&self.protos[REGEXP_PROTO], name, value);
    }
}

impl Drop for Realm<'_> {
    fn drop(&mut self) {
        self.registry.get_mut().teardown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::root_env;

    fn plain_object() -> ObjRef<'static> {
        Rc::new(RefCell::new(JsObject {
            kind: ObjKind::Plain(0),
            props: PropMap::default(),
            proto: None,
            charge: None,
        }))
    }

    #[test]
    fn compaction_drops_dead_entries_and_keeps_live_ones_once_the_floor_is_crossed() {
        let mut registry = Registry::default();
        let live: Vec<ObjRef<'static>> = (0..3).map(|_| plain_object()).collect();
        for obj in &live {
            registry.track_object(obj);
        }
        let scope = root_env();
        registry.track_scope(&scope);
        for _ in 0..REGISTRY_COMPACTION_FLOOR * 2 {
            registry.track_object(&plain_object());
        }
        assert!(registry.objects.len() < REGISTRY_COMPACTION_FLOOR);
        let survivors: Vec<ObjRef<'static>> =
            registry.objects.iter().filter_map(Weak::upgrade).collect();
        assert_eq!(survivors.len(), live.len());
        assert!(live
            .iter()
            .all(|obj| survivors.iter().any(|kept| Rc::ptr_eq(kept, obj))));
        assert_eq!(registry.scopes.len(), 1);
        assert!(registry.scopes[0].upgrade().is_some());
        let scopes_alive = 1;
        let transient_alive_while_tracked = 1;
        let alive_when_compacted = live.len() + scopes_alive + transient_alive_while_tracked;
        assert_eq!(registry.next_compaction, alive_when_compacted * 2);
    }
}
