use oxc_ast::ast::{
    BindingPattern, ForStatementInit, ForStatementLeft, Function, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use rustc_hash::FxHashMap;

use crate::bail::{Bail, EvalResult};
use crate::budget::NESTING_DEPTH_CAP;
use crate::frame::{Frame, FrameRef, WriteSlot};
use crate::heap::Charge;
use crate::names::Name;
use crate::stack;
use crate::value::{Eval, Value};

const SMALL_SCOPE_CAP: usize = 8;

#[derive(Debug)]
pub enum Bindings<'a> {
    Small(Vec<(Name, Value<'a>)>),
    Large(FxHashMap<Name, Value<'a>>),
    Immutable(Name, Value<'a>),
}

impl Default for Bindings<'_> {
    fn default() -> Self {
        Bindings::Small(Vec::new())
    }
}

impl<'a> Bindings<'a> {
    pub fn get(&self, name: Name) -> Option<&Value<'a>> {
        match self {
            Bindings::Small(entries) => entries
                .iter()
                .find(|(bound, _)| *bound == name)
                .map(|(_, value)| value),
            Bindings::Large(map) => map.get(&name),
            Bindings::Immutable(bound, value) => (*bound == name).then_some(value),
        }
    }

    pub fn is_immutable(&self, name: Name) -> bool {
        matches!(self, Bindings::Immutable(bound, _) if *bound == name)
    }

    pub fn get_mut(&mut self, name: Name) -> Option<&mut Value<'a>> {
        match self {
            Bindings::Small(entries) => entries
                .iter_mut()
                .find(|(bound, _)| *bound == name)
                .map(|(_, value)| value),
            Bindings::Large(map) => map.get_mut(&name),
            Bindings::Immutable(_, _) => None,
        }
    }

    pub fn contains_key(&self, name: Name) -> bool {
        self.get(name).is_some()
    }

    pub fn insert(&mut self, name: Name, value: Value<'a>) -> bool {
        if let Some(slot) = self.get_mut(name) {
            *slot = value;
            return false;
        }
        self.push_new(name, value);
        true
    }

    pub fn insert_if_absent(&mut self, name: Name, value: Value<'a>) -> bool {
        if self.contains_key(name) {
            return false;
        }
        self.push_new(name, value);
        true
    }

    fn push_new(&mut self, name: Name, value: Value<'a>) {
        if let Bindings::Small(entries) = self {
            if entries.len() < SMALL_SCOPE_CAP {
                entries.push((name, value));
                return;
            }
        }
        self.push_widening(name, value);
    }

    #[cold]
    #[inline(never)]
    fn push_widening(&mut self, name: Name, value: Value<'a>) {
        match self {
            Bindings::Small(entries) => {
                let mut map: FxHashMap<Name, Value<'a>> = entries.drain(..).collect();
                map.insert(name, value);
                *self = Bindings::Large(map);
            }
            Bindings::Large(map) => {
                map.insert(name, value);
            }
            Bindings::Immutable(bound, existing) => {
                let entries = vec![(*bound, existing.clone()), (name, value)];
                *self = Bindings::Small(entries);
            }
        }
    }

    pub fn clear(&mut self) {
        match self {
            Bindings::Small(entries) => entries.clear(),
            Bindings::Large(map) => map.clear(),
            Bindings::Immutable(_, _) => *self = Bindings::default(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Bindings::Small(entries) => entries.len(),
            Bindings::Large(map) => map.len(),
            Bindings::Immutable(_, _) => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub const SCOPE_CHAIN_CAP: usize = 256;

pub fn root_env<'a>() -> FrameRef<'a> {
    Frame::dynamic(None, None)
}

pub fn child_env<'a>(parent: &FrameRef<'a>) -> FrameRef<'a> {
    charged_env(Some(parent), None)
}

pub fn charged_env<'a>(parent: Option<&FrameRef<'a>>, charge: Option<Charge>) -> FrameRef<'a> {
    Frame::dynamic(parent, charge)
}

pub fn outermost<'a>(env: &FrameRef<'a>) -> FrameRef<'a> {
    let mut current = env.clone();
    loop {
        let parent = current.borrow().parent.clone();
        match parent {
            Some(parent) => current = parent,
            None => return current,
        }
    }
}

#[inline]
pub fn declare<'a>(env: &FrameRef<'a>, name: Name, value: Value<'a>) {
    Frame::declare(env, name, value);
}

#[inline]
pub fn declare_if_absent<'a>(env: &FrameRef<'a>, name: Name, value: Value<'a>) {
    Frame::declare_if_absent(env, name, value);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeChainTooDeep;

impl From<ScopeChainTooDeep> for Bail {
    fn from(_: ScopeChainTooDeep) -> Self {
        Bail::ChainTooDeep(&"scope")
    }
}

fn too_deep<T>() -> EvalResult<T> {
    Err(ScopeChainTooDeep.into())
}

pub fn read_binding<'a>(
    env: &FrameRef<'a>,
    name: Name,
    unbound: impl FnOnce() -> Bail,
) -> Eval<'a, Value<'a>> {
    let mut next = match read_here(env, name) {
        Ok(value) => return Ok(value),
        Err(Some(parent)) => parent,
        Err(None) => return Err(unbound().into()),
    };
    for _ in 1..SCOPE_CHAIN_CAP {
        next = match read_here(&next, name) {
            Ok(value) => return Ok(value),
            Err(Some(parent)) => parent,
            Err(None) => return Err(unbound().into()),
        };
    }
    Err(Bail::from(ScopeChainTooDeep).into())
}

#[inline(always)]
fn read_here<'a>(frame: &FrameRef<'a>, name: Name) -> Result<Value<'a>, Option<FrameRef<'a>>> {
    let scope = frame.borrow();
    if let Some(value) = scope.body.get(name) {
        if scope.parent.is_none() {
            crate::object::note_root_touch(value);
        }
        return Ok(value.clone());
    }
    Err(scope.parent.clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assigned {
    Written,
    Immutable,
    Unbound,
}

pub fn assign<'a>(env: &FrameRef<'a>, name: Name, value: Value<'a>) -> EvalResult<Assigned> {
    assign_from(env, name, value, 0)
}

fn assign_from<'a>(
    env: &FrameRef<'a>,
    name: Name,
    value: Value<'a>,
    depth: usize,
) -> EvalResult<Assigned> {
    if depth >= SCOPE_CHAIN_CAP {
        return too_deep();
    }
    let parent = {
        let mut scope = env.borrow_mut();
        match scope.body.slot_for_write(name) {
            WriteSlot::Slot(slot) => {
                *slot = value;
                return Ok(Assigned::Written);
            }
            WriteSlot::Immutable => return Ok(Assigned::Immutable),
            WriteSlot::Absent => {}
        }
        scope.parent.clone()
    };
    match parent {
        Some(parent) => assign_from(&parent, name, value, depth + 1),
        None => Ok(Assigned::Unbound),
    }
}

pub fn has_binding(env: &FrameRef<'_>, name: Name) -> EvalResult<bool> {
    has_binding_from(env, name, 0)
}

fn has_binding_from(env: &FrameRef<'_>, name: Name, depth: usize) -> EvalResult<bool> {
    if depth >= SCOPE_CHAIN_CAP {
        return too_deep();
    }
    let scope = env.borrow();
    if scope.body.contains_key(name) {
        return Ok(true);
    }
    match &scope.parent {
        Some(parent) => has_binding_from(parent, name, depth + 1),
        None => Ok(false),
    }
}

pub fn hoist_function_declarations<'a>(
    statements: &'a [Statement<'a>],
) -> Vec<(&'a str, &'a Function<'a>)> {
    let mut out = Vec::new();
    for statement in statements {
        let Statement::FunctionDeclaration(function) = statement else {
            continue;
        };
        let Some(id) = &function.id else {
            continue;
        };
        out.push((id.name.as_str(), function.as_ref()));
    }
    out
}

pub fn hoist_var_names<'a>(
    statements: &'a [Statement<'a>],
    out: &mut Vec<&'a str>,
) -> EvalResult<()> {
    hoist_body(statements, out, 0)
}

fn hoist_body<'a>(
    statements: &'a [Statement<'a>],
    out: &mut Vec<&'a str>,
    depth: u32,
) -> EvalResult<()> {
    for statement in statements {
        hoist_statement(statement, out, depth)?;
    }
    Ok(())
}

fn hoist_statement<'a>(
    statement: &'a Statement<'a>,
    out: &mut Vec<&'a str>,
    depth: u32,
) -> EvalResult<()> {
    if depth >= NESTING_DEPTH_CAP {
        return Err(Bail::NestingDepth);
    }
    stack::maybe_grow(|| hoist_statement_inner(statement, out, depth + 1))
}

fn hoist_statement_inner<'a>(
    statement: &'a Statement<'a>,
    out: &mut Vec<&'a str>,
    depth: u32,
) -> EvalResult<()> {
    match statement {
        Statement::VariableDeclaration(decl) => push_var_declaration(decl.as_ref(), out),
        Statement::BlockStatement(block) => hoist_body(&block.body, out, depth)?,
        Statement::IfStatement(node) => {
            hoist_statement(&node.consequent, out, depth)?;
            if let Some(alternate) = &node.alternate {
                hoist_statement(alternate, out, depth)?;
            }
        }
        Statement::WhileStatement(node) => hoist_statement(&node.body, out, depth)?,
        Statement::DoWhileStatement(node) => hoist_statement(&node.body, out, depth)?,
        Statement::LabeledStatement(node) => hoist_statement(&node.body, out, depth)?,
        Statement::ForStatement(node) => {
            if let Some(ForStatementInit::VariableDeclaration(decl)) = &node.init {
                push_var_declaration(decl.as_ref(), out);
            }
            hoist_statement(&node.body, out, depth)?;
        }
        Statement::ForInStatement(node) => {
            hoist_for_left(&node.left, out);
            hoist_statement(&node.body, out, depth)?;
        }
        Statement::ForOfStatement(node) => {
            hoist_for_left(&node.left, out);
            hoist_statement(&node.body, out, depth)?;
        }
        Statement::TryStatement(node) => {
            hoist_body(&node.block.body, out, depth)?;
            if let Some(handler) = &node.handler {
                hoist_body(&handler.body.body, out, depth)?;
            }
            if let Some(finalizer) = &node.finalizer {
                hoist_body(&finalizer.body, out, depth)?;
            }
        }
        Statement::SwitchStatement(node) => {
            for case in &node.cases {
                hoist_body(&case.consequent, out, depth)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn hoist_for_left<'a>(left: &'a ForStatementLeft<'a>, out: &mut Vec<&'a str>) {
    if let ForStatementLeft::VariableDeclaration(decl) = left {
        push_var_declaration(decl.as_ref(), out);
    }
}

fn push_var_declaration<'a>(declaration: &'a VariableDeclaration<'a>, out: &mut Vec<&'a str>) {
    if declaration.kind != VariableDeclarationKind::Var {
        return;
    }
    for declarator in &declaration.declarations {
        if let BindingPattern::BindingIdentifier(id) = &declarator.id {
            out.push(id.name.as_str());
        }
    }
}
