mod arguments;
pub(crate) use arguments::needs_arguments_object;
mod call;
pub(crate) use call::{detach_when_scope_ends, function_expression_name};
pub mod lexical;
mod shared;

use oxc_ast::ast::{Program, Statement};
use std::rc::Rc;

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::Budget;
use crate::budget::{props_bytes, CLOSURE_BYTES, OBJECT_BYTES};
use crate::builtins::args::text_bytes;
use crate::builtins::{Realm, WIDE_SCOPE_BINDINGS};
use crate::env::{declare, declare_if_absent, hoist_function_declarations, hoist_var_names};
use crate::frame::FrameRef;
use crate::heap::{reserve, Charge, HeapLedger};
use crate::host::Host;
use crate::names::{Name, Names};
use crate::object::{self, lookup_property, string_property, Consulted, MAX_ROOT_REGISTRATIONS};
use crate::source::function_source;
use crate::value::{
    js_string_from_literal, Abrupt, Closure, Eval, FnCode, IntrinsicProto, JsObject, JsString,
    ObjKind, ObjRef, PropKey, PropMap, Value,
};

pub fn refuse_arguments_miss(key: &PropKey, inherited: Option<&Value<'_>>) -> EvalResult<()> {
    if object::arguments_miss_differs(key, inherited) {
        return Err(Bail::Unsupported(
            &"a key of arguments that node answers differently",
        ));
    }
    Ok(())
}

fn refuse_unmodelled<'a>(
    target: &Value<'a>,
    key: &PropKey,
    found: Value<'a>,
) -> Eval<'a, Value<'a>> {
    if matches!(found, Value::Undefined) {
        if object::is_constructor_key(key) {
            return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR).into());
        }
        crate::unmodelled::refuse_unmodelled_miss(target, key)?;
    }
    Ok(found)
}

fn code_address(code: FnCode<'_>) -> usize {
    match code {
        FnCode::Function(function) => std::ptr::from_ref(function) as usize,
        FnCode::Arrow(arrow) => std::ptr::from_ref(arrow) as usize,
    }
}

pub struct Interp<'a> {
    pub realm: Realm<'a>,
    pub names: Names<'a>,
    root: FrameRef<'a>,
    pub budget: Budget,
    pub heap: Option<Rc<HeapLedger>>,
    pub source_text: &'a str,
    pub notes: Vec<String>,
    pub this_value: Value<'a>,
    pub strict: bool,
    pub loop_len_hint: usize,
    coercion: NativeCoercion,
    arguments_name: Name,
    pub(crate) prototype_key: PropKey,
    pub(crate) functions: crate::compile::FnTable<'a>,
    regexes: shared::SharedRegexes,
    sources: shared::SharedSources,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeCoercion {
    Outside,
    InSpecOrder,
    Unordered,
}

impl<'a> Interp<'a> {
    pub fn new(source_text: &'a str, budget: Budget) -> Self {
        Self::with_builtin_roots(source_text, budget, false)
    }

    pub fn new_for_decode(source_text: &'a str, budget: Budget) -> Self {
        Self::with_builtin_roots(source_text, budget, true)
    }

    fn with_builtin_roots(source_text: &'a str, budget: Budget, track_roots: bool) -> Self {
        let mut names = Names::for_source(source_text);
        let arguments_name = names.intern("arguments");
        let (realm, globals) = Realm::with_globals();
        if track_roots {
            crate::object::reset_root_table();
            let mut names = Vec::new();
            for (obj, name) in globals.objects() {
                if names.len() >= MAX_ROOT_REGISTRATIONS {
                    break;
                }
                let index = names.len() as u8;
                obj.borrow_mut().set_trace_tag(0x80 | index);
                if let Some(intrinsic) = intrinsic_for_root_name(name) {
                    crate::object::note_intrinsic_root(intrinsic, index);
                }
                crate::object::register_root_address(Rc::as_ptr(&obj) as usize, index);
                names.push(name);
            }
            crate::object::note_builtin_roots(names);
        }
        let root = realm.new_env(None);
        globals.install(&root, &mut names);
        Self {
            realm,
            names,
            root,
            budget,
            heap: None,
            source_text,
            notes: Vec::new(),
            this_value: Value::Undefined,
            strict: false,
            loop_len_hint: 0,
            coercion: NativeCoercion::Outside,
            arguments_name,
            prototype_key: PropKey::Name(JsString::from_str(call::PROTOTYPE_KEY)),
            functions: crate::compile::FnTable::default(),
            regexes: shared::SharedRegexes::default(),
            sources: shared::SharedSources::default(),
        }
    }

    pub fn name_of(&mut self, text: &'a str) -> Name {
        self.names.intern(text)
    }

    pub fn global_env(&self) -> FrameRef<'a> {
        self.root.clone()
    }

    pub(crate) fn root_frame(&self) -> &FrameRef<'a> {
        &self.root
    }

    pub(crate) fn arguments_name(&self) -> Name {
        self.arguments_name
    }

    pub fn hoist(&mut self, statements: &'a [Statement<'a>], env: &FrameRef<'a>) -> Eval<'a, ()> {
        let mut names = Vec::new();
        hoist_var_names(statements, &mut names)?;
        let mut hoisted = names.len();
        for text in names {
            let name = self.name_of(text);
            declare_if_absent(env, name, Value::Undefined);
        }
        for (text, function) in hoist_function_declarations(statements) {
            hoisted += 1;
            let closure = self.make_closure(FnCode::Function(function), env);
            let name = self.name_of(text);
            declare(env, name, closure);
        }
        if hoisted > WIDE_SCOPE_BINDINGS / 2 {
            self.realm.charge_wide(env);
        }
        Ok(())
    }

    pub fn make_closure(&mut self, code: FnCode<'a>, env: &FrameRef<'a>) -> Value<'a> {
        let plain =
            matches!(code, FnCode::Function(function) if !function.generator && !function.r#async);
        self.build_closure(code, env, plain)
    }

    pub fn make_method(&mut self, code: FnCode<'a>, env: &FrameRef<'a>) -> Value<'a> {
        self.build_closure(code, env, false)
    }

    fn build_closure(&mut self, code: FnCode<'a>, env: &FrameRef<'a>, plain: bool) -> Value<'a> {
        let strict = self.strict
            || match code {
                FnCode::Function(function) => function.has_use_strict_directive(),
                FnCode::Arrow(arrow) => arrow.body.has_use_strict_directive(),
            };
        self.realm.capture(env);
        let bytes = OBJECT_BYTES + CLOSURE_BYTES;
        let obj: ObjRef<'a> = std::rc::Rc::new(std::cell::RefCell::new(JsObject {
            kind: ObjKind::Function(std::rc::Rc::new(Closure {
                code,
                env: env.clone(),
                lexical_this: None,
                strict,
                plain,
            })),
            props: PropMap::default(),
            proto: None,
            charge: self.realm.heap_charge(bytes).map(Box::new),
        }));
        self.realm.track(&obj);
        Value::Object(obj)
    }

    #[cold]
    #[inline(never)]
    pub(crate) fn read_after_waking(
        &mut self,
        obj: &ObjRef<'a>,
        key: &PropKey,
    ) -> EvalResult<Option<Value<'a>>> {
        if self.make_pending_prototype(obj)? {
            return lookup_property(obj, key);
        }
        Ok(None)
    }

    pub(crate) fn make_pending_prototype(&mut self, obj: &ObjRef<'a>) -> EvalResult<bool> {
        if !object::has_pending_prototype(obj, &self.prototype_key) {
            return Ok(false);
        }
        let prototype = self
            .realm
            .new_object(ObjKind::Plain(0), self.realm.object_prototype());
        if let Some(created) = prototype.as_object() {
            let before = created.borrow().props.capacity();
            crate::builtins::define_builtin(created, "constructor", Value::Object(obj.clone()));
            let grown = props_bytes(before, created.borrow().props.capacity());
            crate::builtins::args::grow_object(self, created, grown)?;
        }
        let key = crate::value::StoredKey::new(
            self.prototype_key.clone(),
            crate::value::Attrs::PROTOTYPE,
        );
        let before = obj.borrow().props.capacity();
        crate::builtins::define_prop(obj, key, prototype);
        let grown = props_bytes(before, obj.borrow().props.capacity());
        crate::builtins::args::grow_object(self, obj, grown)?;
        Ok(true)
    }

    pub fn set_heap(&mut self, heap: Option<Rc<HeapLedger>>) {
        self.realm.set_heap(heap.clone());
        self.heap = heap;
    }

    #[inline]
    pub fn check_heap(&self) -> EvalResult<()> {
        match &self.heap {
            Some(ledger) => ledger.check(),
            None => Ok(()),
        }
    }

    fn own_function_source(&mut self, target: &Value<'a>) -> Eval<'a, JsString> {
        let Some(obj) = target.as_object() else {
            return Err(self.throw(ErrorKind::TypeError, "not a function"));
        };
        let code = match &obj.borrow().kind {
            ObjKind::Function(closure) => Some(closure.code),
            _ => None,
        };
        match code {
            Some(code) if crate::source::is_method_code(self.source_text, code) => Err(
                Bail::Unsupported(&"the source text of a method, which starts at its key").into(),
            ),
            Some(code) => {
                let node = code_address(code);
                if let Some(shared) = self.sources.get(node) {
                    return Ok(shared);
                }
                let units: Vec<u16> = function_source(self.source_text, code)
                    .encode_utf16()
                    .collect();
                let charge = self.reserve(text_bytes(units.len()))?;
                let text = JsString::charged(units, charge);
                self.sources.keep(node, &text);
                Ok(text)
            }
            None => Err(Bail::Unsupported(&"the source text of a built-in").into()),
        }
    }
}

#[cfg_attr(not(feature = "value-api"), allow(dead_code))]
pub fn eval_script_in<'a, R>(
    program: &'a Program<'a>,
    budget: Budget,
    inspect: impl FnOnce(Eval<'a, Value<'a>>, &[String]) -> R,
) -> R {
    let mut interp = Interp::new(program.source_text, budget);
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::charge_recorded_script(&mut interp);
    let result = run_script(&mut interp, program);
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::record_script(&interp, &result, &[], &[]);
    inspect(result, &interp.notes)
}

#[cfg_attr(not(feature = "value-api"), allow(dead_code))]
pub fn eval_script_decoding<'a, R>(
    program: &'a Program<'a>,
    budget: Budget,
    inspect: impl FnOnce(Eval<'a, Value<'a>>) -> R,
) -> R {
    let mut interp = Interp::new_for_decode(program.source_text, budget);
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::charge_recorded_script(&mut interp);
    object::set_tracing(true);
    let result = match lexical::refuse_program_dead_zones(program) {
        Ok(()) => run_script(&mut interp, program),
        Err(bail) => Err(bail.into()),
    };
    object::set_tracing(false);
    #[cfg(feature = "test-hooks")]
    crate::test_hooks::record_script(
        &interp,
        &result,
        &object::take_trace(),
        &object::take_builtin_roots(),
    );
    #[cfg(not(feature = "test-hooks"))]
    {
        object::take_trace();
        object::take_builtin_roots();
    }
    let answer = inspect(result);
    object::reset_root_table();
    answer
}

fn run_script<'a>(interp: &mut Interp<'a>, program: &'a Program<'a>) -> Eval<'a, Value<'a>> {
    interp.strict = untwist_core::semantic::program_is_strict(program);
    let env = interp.global_env();
    interp.hoist(&program.body, &env)?;
    let mut last = Value::Undefined;
    for directive in &program.directives {
        interp.budget.spend(1)?;
        last = Value::Str(js_string_from_literal(&directive.expression));
    }
    let Some(root) = crate::compile::program::root() else {
        return Err(Bail::Unsupported(&"a program root the compiler could not lay out").into());
    };
    crate::compile::program::run_top_level(interp, (&program.body, &root), &env, last)
}

impl Interp<'_> {
    pub fn reserve(&self, bytes: u64) -> EvalResult<Option<Charge>> {
        match &self.heap {
            Some(ledger) => reserve(ledger, bytes).map(Some),
            None => Ok(None),
        }
    }
}

impl<'a> Interp<'a> {
    fn string_member(
        &mut self,
        target: &Value<'a>,
        text: &JsString,
        key: &PropKey,
    ) -> Eval<'a, Value<'a>> {
        if let PropKey::Index(index) = key {
            if let Some(unit) = text.code_unit_at(*index as usize) {
                return Ok(Value::Str(self.realm.unit_string(unit)?));
            }
        }
        match string_property(text, key) {
            Some(value) => Ok(value),
            None => refuse_unmodelled(target, key, self.realm.string_prototype_member(key)?),
        }
    }

    #[cold]
    #[inline(never)]
    fn refuse_unmodelled_undefined(
        &self,
        target: &Value<'a>,
        obj: &ObjRef<'a>,
        key: &PropKey,
    ) -> EvalResult<()> {
        if object::decoding() {
            note_possible_hole(obj, key);
        } else if self.hole_under_an_indexed_prototype(target, obj, key)? {
            return Err(Bail::Unsupported(&HOLE_UNDER_AN_INDEXED_PROTOTYPE));
        }
        refuse_placeholder_constructor(obj, key)
    }

    fn hole_under_an_indexed_prototype(
        &self,
        target: &Value<'a>,
        obj: &ObjRef<'a>,
        key: &PropKey,
    ) -> EvalResult<bool> {
        if !object::is_dense_index(obj, key) {
            return Ok(false);
        }
        let proto = obj.borrow().proto.clone();
        if let Some(proto) = proto {
            if object::lookup_inherited(&proto, key)?.is_some() {
                return Ok(true);
            }
        }
        Ok(self.realm.intrinsic_member(target, key)?.is_some())
    }
}

impl<'a> Host<'a> for Interp<'a> {
    fn spend(&mut self, steps: u64) -> EvalResult<()> {
        self.budget.spend(steps)
    }

    fn shared_regex(&mut self, pattern: &str, flags: &str) -> Option<regress::Regex> {
        self.regexes.get(pattern, flags)
    }

    fn share_regex(&mut self, pattern: &str, flags: &str, regex: &regress::Regex) {
        self.regexes.offer(pattern, flags, regex);
    }

    fn refuse_global_object(&self, target: &Value<'a>) -> EvalResult<()> {
        match target.as_object() {
            Some(obj) => self.realm.refuse_global_object(obj),
            None => Ok(()),
        }
    }

    fn reserve_live(&mut self, bytes: u64) -> EvalResult<Option<Charge>> {
        self.reserve(bytes)
    }

    fn charges_heap(&self) -> bool {
        self.heap.is_some()
    }

    fn check_heap(&self) -> EvalResult<()> {
        Interp::check_heap(self)
    }

    fn unit_string(&mut self, unit: u16) -> EvalResult<JsString> {
        self.realm.unit_string(unit)
    }

    fn throw(&self, kind: ErrorKind, message: &str) -> Abrupt<'a> {
        Abrupt::throw(self.realm.error_value(kind, message))
    }

    fn note(&mut self, message: &str) {
        self.notes.push(message.to_string());
    }

    fn new_object(&mut self, kind: ObjKind<'a>, proto: Option<ObjRef<'a>>) -> Value<'a> {
        self.realm.new_object(kind, proto)
    }

    fn object_prototype(&self) -> Option<ObjRef<'a>> {
        self.realm.object_prototype()
    }

    fn new_array(&mut self, items: Vec<Value<'a>>) -> Value<'a> {
        self.realm.new_array(items, None)
    }

    fn call_value(
        &mut self,
        callee: &Value<'a>,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        let outer = std::mem::replace(&mut self.coercion, NativeCoercion::Outside);
        let result = self.call(callee, this, args);
        self.coercion = outer;
        result
    }

    fn refuse_namespace(&self, target: &Value<'a>) -> EvalResult<()> {
        match target.as_object() {
            Some(obj) => self.realm.refuse_namespace(obj),
            None => Ok(()),
        }
    }

    fn refuse_coercion_call(&mut self, method: &Value<'a>) -> EvalResult<()> {
        if self.coercion != NativeCoercion::Unordered || !object::decoding() {
            return Ok(());
        }
        let runs_user_code = method.as_object().is_some_and(|obj| {
            matches!(obj.borrow().kind, ObjKind::Function(_) | ObjKind::Bound(_))
        });
        if runs_user_code {
            return Err(Bail::Unsupported(
                &"a native's coercion that runs user code",
            ));
        }
        Ok(())
    }

    fn construct_value(&mut self, callee: &Value<'a>, args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        self.construct(callee, args)
    }

    fn get_property(&mut self, target: &Value<'a>, key: &PropKey) -> Eval<'a, Value<'a>> {
        self.spend(1)?;
        match target {
            Value::Undefined | Value::Null => Err(self.throw(
                ErrorKind::TypeError,
                "cannot read a property of null or undefined",
            )),
            Value::Str(text) => self.string_member(target, text, key),
            Value::Number(_) => {
                refuse_unmodelled(target, key, self.realm.number_prototype_member(key)?)
            }
            Value::Bool(_) => Err(Bail::Unsupported(&"a boolean has no modelled prototype").into()),
            Value::Object(obj) => {
                if let Some(name) = self.realm.hostile_name(obj) {
                    return Err(Bail::hostile_global(name).into());
                }
                let own = lookup_property(obj, key)?;
                if matches!(own, Some(Value::Undefined)) {
                    self.refuse_unmodelled_undefined(target, obj, key)?;
                }
                match own {
                    Some(value) => Ok(value),
                    None => {
                        if object::is_prototype_key(key) {
                            if let Some(woken) = self.read_after_waking(obj, key)? {
                                return Ok(woken);
                            }
                        }
                        let inherited = self.realm.intrinsic_member(target, key)?;
                        if object::decoding() && object::is_arguments(obj) {
                            refuse_arguments_miss(key, inherited.as_ref())?;
                        }
                        if object::is_constructor_key(key) {
                            refuse_unmodelled_constructor(obj, inherited.as_ref())?;
                        }
                        if inherited.is_none() {
                            self.realm.refuse_global_object(obj)?;
                            crate::unmodelled::refuse_unmodelled_miss(target, key)?;
                            if let Some(sentinel) = crate::state::probe_miss(obj, key) {
                                return Ok(sentinel);
                            }
                        }
                        Ok(inherited.unwrap_or(Value::Undefined))
                    }
                }
            }
        }
    }

    fn note_protocol_member(&mut self, target: &Value<'a>, key: &PropKey) -> EvalResult<()> {
        if target.as_object().is_some() {
            self.realm.intrinsic_member(target, key)?;
        }
        Ok(())
    }

    fn function_source(&mut self, target: &Value<'a>) -> Eval<'a, JsString> {
        if crate::builtins::function::unwrap_bound(target).is_some() {
            return Err(Bail::Unsupported(&"the source text of a bound function").into());
        }
        self.own_function_source(target)
    }

    fn enter_nesting(&mut self) -> EvalResult<()> {
        self.budget.enter_nesting()
    }

    fn leave_nesting(&mut self) {
        self.budget.leave_nesting();
    }

    fn get_prototype_of(&mut self, value: &Value<'a>) -> Eval<'a, Value<'a>> {
        let proto = match value {
            Value::Undefined | Value::Null => {
                return Err(self.throw(
                    ErrorKind::TypeError,
                    "Object.getPrototypeOf called on null or undefined",
                ))
            }
            Value::Object(obj) => self.realm.prototype_of(obj)?,
            Value::Str(_) => self.realm.string_prototype(),
            Value::Number(_) => self.realm.number_prototype(),
            Value::Bool(_) => {
                crate::unmodelled::refuse_while_decoding(&"the prototype of a boolean")?;
                None
            }
        };
        Ok(proto.map(Value::Object).unwrap_or(Value::Null))
    }
}

const CONSTRUCTOR_ROOTS: [(&str, IntrinsicProto); 6] = [
    ("Object", IntrinsicProto::Object),
    ("Array", IntrinsicProto::Array),
    ("Function", IntrinsicProto::Function),
    ("String", IntrinsicProto::String),
    ("Number", IntrinsicProto::Number),
    ("RegExp", IntrinsicProto::RegExp),
];

fn intrinsic_for_root_name(name: &str) -> Option<IntrinsicProto> {
    CONSTRUCTOR_ROOTS
        .iter()
        .find(|(root_name, _)| *root_name == name)
        .map(|(_, intrinsic)| *intrinsic)
}

const UNMODELLED_CONSTRUCTOR: &str =
    "a constructor the evaluator does not model, read through a built-in prototype";

const HOLE_UNDER_AN_INDEXED_PROTOTYPE: &str =
    "an array hole or undefined element whose index a prototype also holds";

fn refuse_placeholder_constructor(obj: &ObjRef<'_>, key: &PropKey) -> EvalResult<()> {
    if !object::is_constructor_key(key) {
        return Ok(());
    }
    if !obj.borrow().props.contains_key(key) || crate::unmodelled::is_intrinsic_object(obj) {
        return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR));
    }
    Ok(())
}

fn refuse_unmodelled_constructor(obj: &ObjRef<'_>, found: Option<&Value<'_>>) -> EvalResult<()> {
    let unmodelled = matches!(found, Some(Value::Undefined))
        || match &obj.borrow().kind {
            ObjKind::Function(closure) => closure.is_generator_or_async(),
            ObjKind::Bound(_) => true,
            _ => false,
        };
    if unmodelled {
        return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR));
    }
    Ok(())
}

fn note_possible_hole(obj: &ObjRef<'_>, key: &PropKey) {
    if !matches!(key, PropKey::Index(_)) {
        return;
    }
    let is_dense =
        matches!(obj.borrow().kind, ObjKind::Array(_)) || obj.borrow().kind.is_arguments();
    if is_dense {
        object::note_intrinsics(
            &[IntrinsicProto::Array, IntrinsicProto::Object],
            Consulted::AnyIndex,
        );
    }
}
