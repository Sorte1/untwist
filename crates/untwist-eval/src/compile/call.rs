use std::rc::Rc;

use oxc_ast::ast::{ArrowFunctionBody, BindingIdentifier, Expression, FormalParameters, Statement};

use crate::bail::Bail;
use crate::builtins::WIDE_SCOPE_BINDINGS;
use crate::compile::defaults::{self, Unbound, UNSOUND_DEFAULT};
use crate::compile::lexical::{pending, progressive};
use crate::compile::scope::{self, Hoisted, Provable, ScopeChain};
use crate::compile::{depth, Lazy, LazyExpr, Reach};
use crate::frame::{Frame, FrameLayout, FrameRef};
use crate::interp::Interp;
use crate::names::Name;
use crate::value::{Closure, Completion, Eval, FnCode, ObjRef, Value};

const SHADOWED_PARAMETER: &str =
    "a body var or function that shadows a parameter beside parameter defaults, whose separate var scope the evaluator does not model";

struct Fallback<'a> {
    value: LazyExpr<'a>,
    sound: bool,
}

struct Param<'a> {
    slot: Option<u32>,
    default: Option<Fallback<'a>>,
}

struct Shape<'s, 'a> {
    layout: &'s scope::FunctionLayout<'a>,
    arguments_name: Option<Name>,
    arguments_slot: Option<u32>,
}

enum Body<'a> {
    Statements(Box<[Lazy<'a>]>),
    Expression(LazyExpr<'a>),
    Absent,
}

pub(crate) struct CompiledFn<'a> {
    layout: Rc<FrameLayout>,
    named: Option<Rc<FrameLayout>>,
    rest: bool,
    params: Box<[Param<'a>]>,
    arguments: Option<&'a FormalParameters<'a>>,
    hoisted: Hoisted<'a>,
    body: Body<'a>,
    strict: bool,
    uses_arguments: bool,
    arguments_first: bool,
    native: Option<Box<crate::native_body::Plan<'a>>>,
}

struct Signature<'a> {
    params: &'a FormalParameters<'a>,
    self_name: Option<&'a BindingIdentifier<'a>>,
    arguments: Option<&'a FormalParameters<'a>>,
    statements: Option<&'a [Statement<'a>]>,
}

fn signature<'a>(closure: &Closure<'a>) -> Signature<'a> {
    match closure.code {
        FnCode::Function(function) => Signature {
            params: &function.params,
            self_name: crate::interp::function_expression_name(closure),
            arguments: Some(&function.params),
            statements: function.body.as_ref().map(|body| &body.statements[..]),
        },
        FnCode::Arrow(arrow) => Signature {
            params: &arrow.params,
            self_name: None,
            arguments: None,
            statements: match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(&body.statements[..]),
                _ => None,
            },
        },
    }
}

impl<'a> CompiledFn<'a> {
    fn plain_statements(&self) -> Option<&[Lazy<'a>]> {
        let plain = !self.rest
            && !self.uses_arguments
            && !self.arguments_first
            && self.named.is_none()
            && self.hoisted.refused.is_none()
            && self
                .params
                .iter()
                .all(|param| param.slot.is_some() && param.default.is_none());
        match &self.body {
            Body::Statements(statements) if plain => Some(statements),
            _ => None,
        }
    }
}

impl<'a> Interp<'a> {
    pub(crate) fn compile_function(
        &mut self,
        closure: &Closure<'a>,
        chain: Option<&Rc<ScopeChain>>,
    ) -> Option<CompiledFn<'a>> {
        let signature = signature(closure);
        let named = match signature.self_name {
            Some(id) => Some(scope::self_name_layout(
                self.names.intern(id.name.as_str()),
            )?),
            None => None,
        };
        let parent = named
            .as_ref()
            .map(|layout| ScopeChain::new(layout.clone(), Provable::All, chain.cloned()))
            .or_else(|| chain.cloned());
        let arguments_name = signature.arguments.map(|_| self.arguments_name());
        let mut shape = scope::function_layout(
            &mut self.names,
            signature.params,
            arguments_name,
            signature.statements,
        )?;
        let uses_arguments = match closure.code {
            FnCode::Function(function) => crate::interp::needs_arguments_object(function),
            FnCode::Arrow(_) => false,
        };
        let arguments_slot = arguments_name
            .filter(|_| uses_arguments)
            .and_then(|name| shape.layout.slot(name))
            .and_then(|slot| u32::try_from(slot).ok());
        let outline = Shape {
            layout: &shape,
            arguments_name,
            arguments_slot,
        };
        let params = self.compiled_params(signature.params, &outline, parent.as_ref())?;
        let has_defaults = signature
            .params
            .items
            .iter()
            .any(|param| param.initializer.is_some());
        if has_defaults && shape.hoisted.shadows {
            shape.hoisted.refused = Some(Bail::Unsupported(&SHADOWED_PARAMETER));
        }
        let provable = pending(&shape.layout, &shape.lexicals)?;
        let head = ScopeChain::new(shape.layout.clone(), provable, parent);
        for (_, function) in shape.hoisted.functions.iter() {
            self.functions.register(FnCode::Function(function), &head);
        }
        let compiled = CompiledFn {
            layout: shape.layout,
            named,
            rest: signature.params.rest.is_some(),
            params,
            arguments: signature.arguments,
            hoisted: shape.hoisted,
            body: function_body(closure.code, &head, &shape.lexicals),
            strict: closure.strict,
            uses_arguments,
            arguments_first: arguments_slot.is_some() && has_defaults,
            native: None,
        };
        Some(self.with_native_plan(compiled, closure.code))
    }

    fn with_native_plan(
        &mut self,
        mut compiled: CompiledFn<'a>,
        code: FnCode<'a>,
    ) -> CompiledFn<'a> {
        let native = compiled
            .plain_statements()
            .and_then(|lazies| self.plan_native_body(code, lazies));
        compiled.native = native;
        compiled
    }

    fn compiled_params(
        &mut self,
        params: &'a FormalParameters<'a>,
        outline: &Shape<'_, 'a>,
        parent: Option<&Rc<ScopeChain>>,
    ) -> Option<Box<[Param<'a>]>> {
        let shape = outline.layout;
        let mut order = None;
        let mut out = Vec::with_capacity(params.items.len());
        for (index, param) in params.items.iter().enumerate() {
            let slot = shape.params.get(index).copied().flatten();
            let default = match param.initializer.as_deref() {
                Some(initializer) => {
                    let provable =
                        earlier_parameters((shape, outline.arguments_slot), &mut order, index)?;
                    let scope = ScopeChain::new(shape.layout.clone(), provable, parent.cloned());
                    Some(Fallback {
                        value: LazyExpr::new(initializer, scope),
                        sound: self.is_sound_default(initializer, outline, index),
                    })
                }
                None => None,
            };
            out.push(Param { slot, default });
        }
        Some(out.into_boxed_slice())
    }

    fn is_sound_default(
        &mut self,
        initializer: &Expression<'a>,
        outline: &Shape<'_, 'a>,
        index: usize,
    ) -> bool {
        let shape = outline.layout;
        let mut unbound = Unbound {
            parameters: Vec::new(),
            body: Vec::new(),
        };
        for text in defaults::mentions(initializer) {
            let name = self.names.intern(text);
            if Some(name) == outline.arguments_name {
                continue;
            }
            let Some(slot) = shape.layout.slot(name) else {
                continue;
            };
            let slot = u32::try_from(slot).ok();
            let is_parameter = |bound: &Option<u32>| bound.is_some() && *bound == slot;
            let earlier = shape.params.iter().take(index).any(is_parameter);
            let later = shape.params.iter().skip(index).any(is_parameter);
            match (earlier, later) {
                (_, true) => unbound.parameters.push(text),
                (true, false) => {}
                (false, false) => unbound.body.push(text),
            }
        }
        !defaults::reads_unbound(initializer, &unbound)
    }
}

fn earlier_parameters(
    (shape, arguments_slot): (&scope::FunctionLayout<'_>, Option<u32>),
    order: &mut Option<Rc<[u32]>>,
    index: usize,
) -> Option<Provable> {
    if index == 0 {
        return Some(Provable::Slots(arguments_slot.into_iter().collect()));
    }
    if order.is_none() {
        *order = Some(parameter_order(
            shape.layout.len(),
            &shape.params,
            arguments_slot,
        )?);
    }
    Some(Provable::Before {
        order: order.clone()?,
        below: u32::try_from(index).ok()?.checked_add(1)?,
    })
}

fn parameter_order(
    width: usize,
    params: &[Option<u32>],
    arguments_slot: Option<u32>,
) -> Option<Rc<[u32]>> {
    let mut order: Rc<[u32]> = std::iter::repeat_n(u32::MAX, width).collect();
    let slots = Rc::get_mut(&mut order)?;
    if let Some(slot) = arguments_slot {
        *slots.get_mut(usize::try_from(slot).ok()?)? = 0;
    }
    for (index, slot) in params.iter().enumerate() {
        let Some(slot) = slot else {
            continue;
        };
        let first = slots.get_mut(usize::try_from(*slot).ok()?)?;
        if *first == u32::MAX {
            *first = u32::try_from(index).ok()?.checked_add(1)?;
        }
    }
    Some(order)
}

fn function_body<'a>(
    code: FnCode<'a>,
    head: &Rc<ScopeChain>,
    lexicals: &[(usize, u32)],
) -> Body<'a> {
    let statements = match code {
        FnCode::Function(function) => match &function.body {
            Some(body) => &body.statements,
            None => return Body::Absent,
        },
        FnCode::Arrow(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &body.statements,
            other => match other.as_expression() {
                Some(expression) => {
                    return Body::Expression(LazyExpr::new(expression, head.clone()))
                }
                None => return Body::Absent,
            },
        },
    };
    Body::Statements(progressive(statements, head, lexicals))
}

impl<'a> Interp<'a> {
    pub(crate) fn compiled_frames(
        &mut self,
        compiled: &CompiledFn<'a>,
        callee: &Value<'a>,
        closure: &Closure<'a>,
    ) -> FrameRef<'a> {
        debug_assert_eq!(compiled.strict, closure.strict);
        #[cfg(feature = "test-hooks")]
        crate::test_hooks::note_compiled_call(closure.code);
        match &compiled.named {
            Some(layout) => {
                let named = self.realm.new_fixed_env(layout, &closure.env);
                Frame::declare_slot(&named, 0, callee.clone());
                self.realm.new_fixed_env(&compiled.layout, &named)
            }
            None => self.realm.new_fixed_env(&compiled.layout, &closure.env),
        }
    }

    pub(crate) fn run_compiled(
        &mut self,
        compiled: &CompiledFn<'a>,
        closure: &Closure<'a>,
        env: &FrameRef<'a>,
        args: &[Value<'a>],
    ) -> Eval<'a, Value<'a>> {
        let reach = depth::frame_reach(self);
        let early = match compiled.arguments_first {
            true => self.arguments_before_parameters(compiled, closure, args, env)?,
            false => None,
        };
        self.bind_compiled_parameters(compiled, args, (env, reach))?;
        let mapped = match (compiled.arguments_first, compiled.arguments) {
            (true, _) => early,
            (false, Some(params)) => {
                self.install_arguments((closure, compiled.uses_arguments), params, args, env)?
            }
            (false, None) => None,
        };
        let outcome = self.run_compiled_body(compiled, (env, reach));
        if let Some(arguments) = mapped {
            crate::interp::detach_when_scope_ends(arguments, env);
        }
        outcome
    }

    #[cold]
    #[inline(never)]
    fn arguments_before_parameters(
        &mut self,
        compiled: &CompiledFn<'a>,
        closure: &Closure<'a>,
        args: &[Value<'a>],
        env: &FrameRef<'a>,
    ) -> Eval<'a, Option<ObjRef<'a>>> {
        let Some(params) = compiled.arguments else {
            return Ok(None);
        };
        self.charge_arguments(args)?;
        self.install_arguments_object(closure.strict, params, args, env)
    }

    fn bind_compiled_parameters(
        &mut self,
        compiled: &CompiledFn<'a>,
        args: &[Value<'a>],
        (env, reach): (&FrameRef<'a>, Reach),
    ) -> Eval<'a, ()> {
        if compiled.rest {
            return Err(Bail::Unsupported(&"rest parameter").into());
        }
        for (index, param) in compiled.params.iter().enumerate() {
            let Some(slot) = param.slot else {
                return Err(Bail::Unsupported(&"destructuring parameter").into());
            };
            let mut value = args.get(index).cloned().unwrap_or(Value::Undefined);
            if matches!(value, Value::Undefined) {
                if let Some(default) = &param.default {
                    if !default.sound {
                        return Err(Bail::Unsupported(&UNSOUND_DEFAULT).into());
                    }
                    value = default.value.eval(self, env, reach)?;
                }
            }
            Frame::declare_slot(env, slot, value);
        }
        if compiled.params.len() > WIDE_SCOPE_BINDINGS / 2 {
            self.realm.charge_wide(env);
        }
        Ok(())
    }

    fn hoist_compiled(&mut self, hoisted: &Hoisted<'a>, env: &FrameRef<'a>) {
        for slot in hoisted.vars.clone() {
            Frame::declare_slot_if_absent(env, slot, Value::Undefined);
        }
        for (slot, function) in hoisted.functions.iter() {
            let closure = self.make_closure(FnCode::Function(function), env);
            Frame::declare_slot(env, *slot, closure);
        }
        if hoisted.count > WIDE_SCOPE_BINDINGS / 2 {
            self.realm.charge_wide(env);
        }
    }

    #[inline(never)]
    fn run_native_body(
        &mut self,
        statements: &[Lazy<'a>],
        plan: &crate::native_body::Plan<'a>,
        (env, reach): (&FrameRef<'a>, Reach),
    ) -> Eval<'a, Value<'a>> {
        for (index, statement) in statements.iter().enumerate() {
            let native = match plan.step(index) {
                Some(step) => self.run_native_step(step, (env, reach))?,
                None => false,
            };
            if native {
                continue;
            }
            match statement.run(self, env, reach)? {
                Completion::Normal => {}
                Completion::Return(value) => return Ok(value),
                Completion::Break(_) | Completion::Continue(_) => {
                    return Err(Bail::Unsupported(&"break or continue outside a loop").into())
                }
            }
        }
        Ok(Value::Undefined)
    }

    fn run_compiled_body(
        &mut self,
        compiled: &CompiledFn<'a>,
        (env, reach): (&FrameRef<'a>, Reach),
    ) -> Eval<'a, Value<'a>> {
        let statements = match &compiled.body {
            Body::Statements(statements) => statements,
            Body::Expression(expression) => return expression.eval(self, env, reach),
            Body::Absent => return Ok(Value::Undefined),
        };
        if let Some(bail) = &compiled.hoisted.refused {
            return Err(bail.clone().into());
        }
        self.hoist_compiled(&compiled.hoisted, env);
        if let Some(plan) = &compiled.native {
            return self.run_native_body(statements, plan, (env, reach));
        }
        for statement in statements.iter() {
            match statement.run(self, env, reach)? {
                Completion::Normal => {}
                Completion::Return(value) => return Ok(value),
                Completion::Break(_) | Completion::Continue(_) => {
                    return Err(Bail::Unsupported(&"break or continue outside a loop").into())
                }
            }
        }
        Ok(Value::Undefined)
    }
}
