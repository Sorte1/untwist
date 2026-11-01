use std::rc::Rc;

use oxc_ast::ast::{BindingPattern, VariableDeclaration, VariableDeclarationKind};

use crate::bail::Bail;
use crate::compile::depth::Reach;
use crate::compile::expr::compile_expression;
use crate::compile::lexical::is_lexical;
use crate::compile::scope::{Resolved, ScopeChain};
use crate::compile::{Code, Compiler};
use crate::env::{assign, declare, has_binding, Assigned};
use crate::frame::{Frame, FrameRef};
use crate::interp::Interp;
use crate::names::Name;
use crate::value::{Eval, Value};

const DESTRUCTURING: &str = "destructuring declaration";
const USING: &str = "using declaration";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Var,
    Lexical,
    Using,
}

enum Target {
    Var(Resolved),
    VarNamed(Name),
    Slot(u32),
    Named(Name),
    Destructuring,
}

struct Declarator<'a> {
    target: Target,
    init: Option<Code<'a>>,
}

pub(crate) struct Declaration<'a> {
    kind: Kind,
    declarators: Box<[Declarator<'a>]>,
}

fn kind_of(kind: VariableDeclarationKind) -> Kind {
    match kind {
        VariableDeclarationKind::Var => Kind::Var,
        other if is_lexical(other) => Kind::Lexical,
        _ => Kind::Using,
    }
}

fn var_target(scope: &Rc<ScopeChain>, name: Name) -> Target {
    match scope.resolve(name) {
        resolved @ (Resolved::Local(_) | Resolved::Outer { .. } | Resolved::TooDeep) => {
            Target::Var(resolved)
        }
        _ => Target::VarNamed(name),
    }
}

fn lexical_target(scope: &Rc<ScopeChain>, name: Name) -> Target {
    if scope.is_root() {
        return Target::Named(name);
    }
    match scope
        .layout
        .slot(name)
        .and_then(|slot| u32::try_from(slot).ok())
    {
        Some(slot) => Target::Slot(slot),
        None => Target::Named(name),
    }
}

pub(crate) fn compile_declaration<'a>(
    declaration: &'a VariableDeclaration<'a>,
    scope: &Rc<ScopeChain>,
    cx: &mut Compiler<'_, 'a>,
) -> Declaration<'a> {
    let kind = kind_of(declaration.kind);
    let mut declarators = Vec::with_capacity(declaration.declarations.len());
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            declarators.push(Declarator {
                target: Target::Destructuring,
                init: None,
            });
            break;
        };
        let init = declarator
            .init
            .as_ref()
            .map(|init| compile_expression(init, scope, cx));
        let name = cx.names.intern(id.name.as_str());
        let target = match kind {
            Kind::Var => var_target(scope, name),
            Kind::Lexical | Kind::Using => lexical_target(scope, name),
        };
        declarators.push(Declarator { target, init });
    }
    Declaration {
        kind,
        declarators: declarators.into_boxed_slice(),
    }
}

impl<'a> Declaration<'a> {
    pub(crate) fn run(
        &self,
        interp: &mut Interp<'a>,
        frame: &FrameRef<'a>,
        reach: Reach,
    ) -> Eval<'a, ()> {
        for declarator in self.declarators.iter() {
            declarator.run(interp, (frame, reach), self.kind)?;
        }
        if self.kind != Kind::Var {
            interp.realm.charge_wide(frame);
        }
        Ok(())
    }
}

impl<'a> Declarator<'a> {
    fn run(
        &self,
        interp: &mut Interp<'a>,
        (frame, reach): (&FrameRef<'a>, Reach),
        kind: Kind,
    ) -> Eval<'a, ()> {
        if matches!(self.target, Target::Destructuring) {
            return Err(Bail::Unsupported(&DESTRUCTURING).into());
        }
        let value = match &self.init {
            Some(init) => Some(init.run(interp, frame, reach)?),
            None => None,
        };
        if kind == Kind::Using {
            return Err(Bail::Unsupported(&USING).into());
        }
        match (&self.target, value) {
            (Target::Var(resolved), Some(value)) => resolved.assign_var(frame, value),
            (Target::Var(resolved), None) => resolved.has_var(),
            (Target::VarNamed(name), value) => dynamic_var(frame, *name, value),
            (Target::Slot(slot), value) => {
                Frame::declare_slot(frame, *slot, value.unwrap_or(Value::Undefined));
                Ok(())
            }
            (Target::Named(name), value) => {
                declare(frame, *name, value.unwrap_or(Value::Undefined));
                Ok(())
            }
            (Target::Destructuring, _) => Err(Bail::Unsupported(&DESTRUCTURING).into()),
        }
    }
}

fn dynamic_var<'a>(frame: &FrameRef<'a>, name: Name, value: Option<Value<'a>>) -> Eval<'a, ()> {
    match value {
        Some(value) => {
            if assign(frame, name, value.clone())? == Assigned::Unbound {
                declare(frame, name, value);
            }
        }
        None => {
            if !has_binding(frame, name)? {
                declare(frame, name, Value::Undefined);
            }
        }
    }
    Ok(())
}
