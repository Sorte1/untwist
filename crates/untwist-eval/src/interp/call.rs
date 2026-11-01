use std::rc::Rc;

use oxc_ast::ast::{BindingIdentifier, BindingPattern, FormalParameters, FunctionType};

use crate::bail::{Bail, ErrorKind};
use crate::budget::{ARRAY_LEN_CAP, CALL_STACK_GROWTH};
use crate::builtins::args::{adopt_charge, slot_bytes};
use crate::env::{declare, declare_if_absent};
use crate::frame::FrameRef;
use crate::host::Host;
use crate::interp::{Interp, NativeCoercion};
use crate::names::Name;
use crate::stack;
use crate::value::{
    Abrupt, BoundFunction, Closure, Eval, FnCode, MappedArguments, NativeId, ObjKind, ObjRef, Value,
};

enum Callable<'a> {
    Closure(Rc<Closure<'a>>),
    Native(NativeId),
    Bound(Rc<BoundFunction<'a>>),
}

pub(super) const PROTOTYPE_KEY: &str = "prototype";

impl<'a> Interp<'a> {
    pub fn call(
        &mut self,
        callee: &Value<'a>,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        let Some(obj) = callee.as_object() else {
            return Err(self.throw(ErrorKind::TypeError, "value is not a function"));
        };
        let kind = match &obj.borrow().kind {
            ObjKind::Function(closure) => Callable::Closure(closure.clone()),
            ObjKind::Native(id, _) => Callable::Native(*id),
            ObjKind::Bound(bound) => Callable::Bound(bound.clone()),
            _ => return Err(self.throw(ErrorKind::TypeError, "value is not a function")),
        };
        self.budget.enter_call()?;
        if stack::has_room() {
            let result = self.dispatch(kind, callee, obj, this, args);
            self.budget.leave_call();
            return result;
        }
        let result = stack::grow(CALL_STACK_GROWTH, || {
            self.dispatch(kind, callee, obj, this, args)
        });
        self.budget.leave_call();
        result
    }

    fn dispatch(
        &mut self,
        kind: Callable<'a>,
        callee: &Value<'a>,
        obj: &ObjRef<'a>,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        match kind {
            Callable::Closure(closure) => self.invoke_closure(callee.clone(), &closure, this, args),
            Callable::Native(id) => match self.hostile_global_bail(obj, id) {
                Some(abrupt) => Err(abrupt),
                None => self.call_native(id, this, args),
            },
            Callable::Bound(bound) => {
                self.budget
                    .spend(1 + (bound.args.len() + args.len()) as u64 / 64)?;
                let mut combined = bound.args.clone();
                combined.extend_from_slice(args);
                self.call(&bound.target, bound.this.clone(), &combined)
            }
        }
    }

    fn hostile_global_bail(&self, obj: &ObjRef<'a>, id: NativeId) -> Option<Abrupt<'a>> {
        if !self.realm.is_bailing_native(id) {
            return None;
        }
        Some(Bail::hostile_global(crate::builtins::native_hostile_name(obj)).into())
    }

    pub fn call_native(
        &mut self,
        id: NativeId,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        let table = self.realm.natives();
        let Some((function, shape)) = table.get(id) else {
            return Err(Bail::Unsupported(&"unknown native function").into());
        };
        if !shape.admits(&this, args) {
            return Err(Bail::Unsupported(&crate::builtins::OUTSIDE_SHAPE).into());
        }
        let inside = if shape.coerces_in_spec_order {
            NativeCoercion::InSpecOrder
        } else {
            NativeCoercion::Unordered
        };
        let outer = std::mem::replace(&mut self.coercion, inside);
        let result = function(self, this, args);
        self.coercion = outer;
        result
    }

    pub fn invoke_closure(
        &mut self,
        callee: Value<'a>,
        closure: &Rc<Closure<'a>>,
        this: Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        if closure.is_generator_or_async() {
            return Err(Bail::Unsupported(&"a call of a generator or an async function").into());
        }
        self.check_heap()?;
        let compiled = self.compiled(closure)?;
        let env = self.compiled_frames(&compiled, &callee, closure);
        self.invocation(&callee, closure, this, args, |interp| {
            interp.run_compiled(&compiled, closure, &env, args)
        })
    }

    #[inline(always)]
    fn invocation(
        &mut self,
        callee: &Value<'a>,
        closure: &Closure<'a>,
        this: Value<'a>,
        args: &[Value<'a>],
        body: impl FnOnce(&mut Self) -> Eval<'a, Value<'a>>,
    ) -> Eval<'a, Value<'a>> {
        let previous_this = self.this_value.clone();
        let previous_strict = self.strict;
        self.this_value = match &closure.lexical_this {
            Some(bound) => (**bound).clone(),
            None => this,
        };
        self.strict = closure.strict;
        let pushed = crate::state::enter_invocation(callee.as_object(), args);
        let outcome = body(self);
        crate::state::leave_invocation(pushed, outcome.as_ref().ok());
        self.this_value = previous_this;
        self.strict = previous_strict;
        outcome
    }

    pub(crate) fn install_arguments(
        &mut self,
        (closure, uses_arguments): (&Closure<'a>, bool),
        params: &'a FormalParameters<'a>,
        args: &[Value<'a>],
        env: &FrameRef<'a>,
    ) -> Eval<'a, Option<ObjRef<'a>>> {
        self.charge_arguments(args)?;
        if !uses_arguments {
            declare_if_absent(env, self.arguments_name, Value::Undefined);
            return Ok(None);
        }
        self.install_arguments_object(closure.strict, params, args, env)
    }

    #[inline(always)]
    pub(crate) fn charge_arguments(&mut self, args: &[Value<'a>]) -> Eval<'a, ()> {
        if args.len() > ARRAY_LEN_CAP {
            return Err(Bail::ArrayLength.into());
        }
        self.budget.spend(1 + args.len() as u64 / 64)?;
        Ok(())
    }

    #[inline(never)]
    pub(crate) fn install_arguments_object(
        &mut self,
        strict: bool,
        params: &'a FormalParameters<'a>,
        args: &[Value<'a>],
        env: &FrameRef<'a>,
    ) -> Eval<'a, Option<ObjRef<'a>>> {
        let map = match strict {
            true => Vec::new(),
            false => self.parameter_map(params, args.len()),
        };
        let slots = slot_bytes(args.len()).saturating_add(slot_bytes(map.len()) / 2);
        let reserved = self.reserve(slots)?;
        if map.is_empty() {
            let arguments = self
                .realm
                .new_object(ObjKind::Arguments(args.to_vec()), None);
            adopt_charge(&arguments, reserved);
            declare(env, self.arguments_name, arguments);
            return Ok(None);
        }
        self.realm.capture(env);
        let kind = ObjKind::MappedArguments(Box::new(MappedArguments {
            items: args.to_vec(),
            scope: env.clone(),
            map,
        }));
        let arguments = self.realm.new_object(kind, None);
        adopt_charge(&arguments, reserved);
        let object = arguments.as_object().cloned();
        declare(env, self.arguments_name, arguments);
        Ok(object)
    }

    fn parameter_map(
        &mut self,
        params: &'a FormalParameters<'a>,
        passed: usize,
    ) -> Vec<Option<Name>> {
        let simple = params.rest.is_none()
            && params.items.iter().all(|param| {
                param.initializer.is_none()
                    && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
            });
        if !simple {
            return Vec::new();
        }
        let count = params.items.len().min(passed);
        let mut map: Vec<Option<Name>> = Vec::with_capacity(count);
        for index in 0..count {
            let BindingPattern::BindingIdentifier(id) = &params.items[index].pattern else {
                return Vec::new();
            };
            let repeated = params.items[index + 1..].iter().any(|later| {
                matches!(&later.pattern, BindingPattern::BindingIdentifier(other) if other.name == id.name)
            });
            map.push((!repeated).then(|| self.names.intern(id.name.as_str())));
        }
        if map.iter().all(Option::is_none) {
            return Vec::new();
        }
        map
    }

    pub fn ensure_prototype(&mut self, callee: &Value<'a>) -> Eval<'a, Value<'a>> {
        let key = self.prototype_key.clone();
        let existing = self.get_property(callee, &key)?;
        if matches!(existing, Value::Object(_)) {
            return Ok(existing);
        }
        Ok(self
            .realm
            .object_prototype()
            .map_or(Value::Null, Value::Object))
    }

    pub fn construct(&mut self, callee: &Value<'a>, args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        if let Some(obj) = callee.as_object() {
            if let Some(name) = self.realm.hostile_name(obj) {
                return Err(Bail::hostile_global(name).into());
            }
        }
        if !callee.is_callable() || !is_constructible(callee) {
            return Err(self.throw(ErrorKind::TypeError, "value is not a constructor"));
        }
        if bound_function(callee).is_some() {
            return self.construct_bound(callee, args);
        }
        if let Some(built) = self.construct_intrinsic(callee, args)? {
            return Ok(built);
        }
        if crate::object::decoding() {
            if let Some(obj) = callee.as_object() {
                if matches!(obj.borrow().kind, ObjKind::Native(_, _)) {
                    return Err(Bail::Unsupported(
                        &"constructing a native the evaluator does not model",
                    )
                    .into());
                }
            }
        }
        let prototype = self.ensure_prototype(callee)?;
        if let Some(proto) = prototype.as_object() {
            self.make_pending_prototype(proto)?;
            crate::unmodelled::refuse_unmodelled_prototype(proto)?;
            crate::state::note_prototype(proto);
            crate::object::note_prototype_attributes(proto);
        }
        let instance = self
            .realm
            .new_object(ObjKind::Plain(0), prototype.as_object().cloned());
        let result = self.call(callee, instance.clone(), args)?;
        if matches!(result, Value::Object(_)) {
            return Ok(result);
        }
        Ok(instance)
    }

    #[cold]
    #[inline(never)]
    fn construct_bound(&mut self, callee: &Value<'a>, args: &[Value<'a>]) -> Eval<'a, Value<'a>> {
        let mut target = callee.clone();
        let mut combined = args.to_vec();
        while let Some(bound) = bound_function(&target) {
            self.budget
                .spend(1 + (bound.args.len() + combined.len()) as u64 / 64)?;
            let mut outer = bound.args.clone();
            outer.append(&mut combined);
            combined = outer;
            target = bound.target.clone();
        }
        self.construct(&target, &combined)
    }

    fn construct_intrinsic(
        &mut self,
        callee: &Value<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Option<Value<'a>>> {
        let Some(id) = self.realm.intrinsic_constructor(callee) else {
            return Ok(None);
        };
        let built = self.call_native(id, Value::Undefined, args)?;
        Ok(Some(built))
    }
}

#[cold]
#[inline(never)]
pub(crate) fn detach_when_scope_ends<'a>(arguments: ObjRef<'a>, env: &FrameRef<'a>) {
    const OWN_AND_ARGUMENTS: usize = 2;
    if Rc::strong_count(env) > OWN_AND_ARGUMENTS {
        return;
    }
    let mut borrowed = arguments.borrow_mut();
    let detached = match std::mem::replace(&mut borrowed.kind, ObjKind::Plain(0)) {
        ObjKind::MappedArguments(mapped) => ObjKind::Arguments(mapped.into_unmapped()),
        other => other,
    };
    borrowed.kind = detached;
}

pub(crate) fn function_expression_name<'a>(
    closure: &Closure<'a>,
) -> Option<&'a BindingIdentifier<'a>> {
    let FnCode::Function(function) = closure.code else {
        return None;
    };
    if function.r#type != FunctionType::FunctionExpression {
        return None;
    }
    function.id.as_ref()
}

fn bound_function<'a>(callee: &Value<'a>) -> Option<Rc<BoundFunction<'a>>> {
    match &callee.as_object()?.borrow().kind {
        ObjKind::Bound(bound) => Some(bound.clone()),
        _ => None,
    }
}

fn is_constructible(callee: &Value<'_>) -> bool {
    callee
        .as_object()
        .is_some_and(|obj| match &obj.borrow().kind {
            ObjKind::Function(closure) => closure.plain,
            _ => true,
        })
}
