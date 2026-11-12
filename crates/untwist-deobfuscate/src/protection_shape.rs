use std::fmt::Write;
use std::sync::OnceLock;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::global::{global_is_written, is_global_reference};
use untwist_core::ast_util::js_text::js_text;
use untwist_core::parse::{parse_with, SourceType};
use untwist_core::semantic::build_scoping;

const TEMPLATES: &str = include_str!("protection_shapes.js");
const NODE_BUDGET: usize = 1024;
const OWN_NAME: &str = "SELF";
const CONTROLLER_NAME: &str = "CONTROLLER";
const PROTECTION_NAME: &str = "PROTECTION";

#[derive(Clone, Copy)]
enum Role {
    Own,
    Controller,
    Protection,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Roles<'k> {
    pub(crate) own: Option<SymbolId>,
    pub(crate) controller: Option<SymbolId>,
    pub(crate) protections: Option<&'k FxHashSet<SymbolId>>,
}

impl Roles<'_> {
    fn of(&self, symbol: SymbolId) -> Option<Role> {
        if self.own == Some(symbol) {
            return Some(Role::Own);
        }
        if self.controller == Some(symbol) {
            return Some(Role::Controller);
        }
        self.protections
            .is_some_and(|protections| protections.contains(&symbol))
            .then_some(Role::Protection)
    }
}

fn template_role(name: &str) -> Option<Role> {
    match name {
        OWN_NAME => Some(Role::Own),
        CONTROLLER_NAME => Some(Role::Controller),
        PROTECTION_NAME => Some(Role::Protection),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Slot {
    Symbol(SymbolId),
    TemplateProtection,
}

struct Locals {
    found: FxHashSet<SymbolId>,
    called: FxHashSet<ReferenceId>,
    functions: Vec<(SymbolId, Span)>,
    budget: usize,
}

fn is_plain_function(init: &Expression<'_>) -> bool {
    match init {
        Expression::FunctionExpression(function) => {
            function.id.is_none() && !function.r#async && !function.generator
        }
        Expression::ArrowFunctionExpression(arrow) => {
            !arrow.r#async && arrow.get_expression().is_none()
        }
        _ => false,
    }
}

impl Locals {
    fn callables(&self, scoping: &Scoping) -> FxHashSet<Span> {
        self.functions
            .iter()
            .filter(|(symbol, _)| {
                scoping
                    .get_resolved_reference_ids(*symbol)
                    .iter()
                    .all(|reference| self.called.contains(reference))
            })
            .map(|(_, span)| *span)
            .collect()
    }
}

impl<'a> VisitJs<'a> for Locals {
    fn enter_node(&mut self, _: AstKind<'a>) {
        self.budget = self.budget.saturating_sub(1);
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.budget > 0 {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.budget > 0 {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.found.extend(it.symbol_id.get());
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = it.callee.without_parentheses() {
            self.called.extend(callee.reference_id.get());
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let init = init.without_parentheses();
            if is_plain_function(init) {
                self.functions
                    .extend(id.symbol_id.get().map(|symbol| (symbol, init.span())));
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}

struct Shape<'k> {
    scoping: &'k Scoping,
    roles: Option<Roles<'k>>,
    any_number: bool,
    locals: FxHashSet<SymbolId>,
    callables: FxHashSet<Span>,
    slots: FxHashMap<Slot, usize>,
    globals: Vec<String>,
    text: String,
    poisoned: bool,
}

impl Shape<'_> {
    fn tag(&mut self, tag: impl std::fmt::Debug) {
        let _ = write!(self.text, " {tag:?}");
    }

    fn numbered(&mut self, prefix: char, slot: Slot) {
        let next = self.slots.len();
        let number = *self.slots.entry(slot).or_insert(next);
        let _ = write!(self.text, " {prefix}{number}");
    }

    fn role(&mut self, role: Role, slot: Slot) {
        match role {
            Role::Own => self.text.push_str(" S"),
            Role::Controller => self.text.push_str(" C"),
            Role::Protection => self.numbered('P', slot),
        }
    }

    fn binding(&mut self, id: &BindingIdentifier<'_>) {
        let Some(symbol) = id.symbol_id.get() else {
            self.poisoned = true;
            return;
        };
        let reassigned_constant = self
            .scoping
            .symbol_flags(symbol)
            .contains(SymbolFlags::ConstVariable)
            && self.scoping.symbol_is_mutated(symbol);
        self.poisoned |= reassigned_constant;
        self.numbered('$', Slot::Symbol(symbol));
    }

    fn free(&mut self, id: &IdentifierReference<'_>) {
        if !is_global_reference(self.scoping, id) {
            self.poisoned = true;
            return;
        }
        let name = id.name.as_str();
        let role = if self.roles.is_none() {
            template_role(name)
        } else {
            None
        };
        match role {
            Some(role) => self.role(role, Slot::TemplateProtection),
            None => {
                let _ = write!(self.text, " G:{name}");
                self.globals.push(name.to_string());
            }
        }
    }

    fn reference(&mut self, id: &IdentifierReference<'_>) {
        let Some(reference) = id.reference_id.get() else {
            self.poisoned = true;
            return;
        };
        let Some(symbol) = self.scoping.get_reference(reference).symbol_id() else {
            self.free(id);
            return;
        };
        if self.locals.contains(&symbol) {
            self.numbered('$', Slot::Symbol(symbol));
            return;
        }
        match self.roles.and_then(|roles| roles.of(symbol)) {
            Some(role) => self.role(role, Slot::Symbol(symbol)),
            None => self.poisoned = true,
        }
    }

    fn number(&mut self, literal: &NumericLiteral<'_>) {
        if self.any_number {
            self.text.push_str(" #");
        } else {
            let _ = write!(self.text, " {:x}", literal.value.to_bits());
        }
    }

    fn declaration_kind(&mut self, kind: VariableDeclarationKind) {
        match kind {
            VariableDeclarationKind::Var
            | VariableDeclarationKind::Let
            | VariableDeclarationKind::Const => {}
            other => self.tag(other),
        }
    }

    fn leaf(&mut self, kind: AstKind<'_>) -> bool {
        match kind {
            AstKind::BindingIdentifier(it) => self.binding(it),
            AstKind::IdentifierReference(it) => self.reference(it),
            AstKind::IdentifierName(it) => self.tag(it.name.as_str()),
            AstKind::StringLiteral(it) => self.tag(js_text(it.value)),
            AstKind::NumericLiteral(it) => self.number(it),
            AstKind::BooleanLiteral(it) => self.tag(it.value),
            _ => return false,
        }
        true
    }

    fn operator(&mut self, kind: AstKind<'_>) -> bool {
        match kind {
            AstKind::AssignmentExpression(it) => self.tag(it.operator),
            AstKind::UpdateExpression(it) => self.tag((it.operator, it.prefix)),
            AstKind::UnaryExpression(it) => self.tag(it.operator),
            AstKind::BinaryExpression(it) => self.tag(it.operator),
            AstKind::LogicalExpression(it) => self.tag(it.operator),
            AstKind::CallExpression(it) => self.tag(it.optional),
            AstKind::StaticMemberExpression(it) => self.tag(it.optional),
            AstKind::ComputedMemberExpression(it) => self.tag(it.optional),
            _ => return false,
        }
        true
    }

    fn flagged(&mut self, kind: AstKind<'_>) -> bool {
        match kind {
            AstKind::Function(it) => self.tag((it.r#type, it.r#async, it.generator)),
            AstKind::ArrowFunctionExpression(it) => {
                self.tag((it.get_expression().is_some(), it.r#async))
            }
            AstKind::VariableDeclaration(it) => self.declaration_kind(it.kind),
            AstKind::ForStatement(it) => {
                self.tag((it.init.is_some(), it.test.is_some(), it.update.is_some()));
            }
            _ => return false,
        }
        true
    }

    fn is_callable(&self, kind: AstKind<'_>) -> bool {
        matches!(
            kind,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        ) && self.callables.contains(&kind.span())
    }

    fn is_plain(kind: AstKind<'_>) -> bool {
        matches!(
            kind,
            AstKind::FunctionBody(_)
                | AstKind::FormalParameters(_)
                | AstKind::FormalParameter(_)
                | AstKind::VariableDeclarator(_)
                | AstKind::ExpressionStatement(_)
                | AstKind::ReturnStatement(_)
                | AstKind::IfStatement(_)
                | AstKind::BlockStatement(_)
                | AstKind::TryStatement(_)
                | AstKind::CatchClause(_)
                | AstKind::CatchParameter(_)
                | AstKind::NewExpression(_)
                | AstKind::ArrayExpression(_)
                | AstKind::ObjectExpression(_)
        )
    }

    fn finish(self) -> Option<String> {
        if self.poisoned {
            return None;
        }
        let written = self
            .globals
            .iter()
            .any(|name| global_is_written(self.scoping, name));
        (!written).then_some(self.text)
    }
}

impl<'a> VisitJs<'a> for Shape<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if matches!(kind, AstKind::ParenthesizedExpression(_)) {
            return;
        }
        if self.is_callable(kind) {
            self.text.push_str("(Callable");
            return;
        }
        let _ = write!(self.text, "({:?}", kind.ty());
        let known = self.leaf(kind) || self.operator(kind) || self.flagged(kind);
        self.poisoned |= !known && !Self::is_plain(kind);
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if !matches!(kind, AstKind::ParenthesizedExpression(_)) {
            self.text.push(')');
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if !self.poisoned {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if !self.poisoned {
            walk_js::walk_expression(self, it);
        }
    }
}

fn shape(
    function: &Function<'_>,
    scoping: &Scoping,
    roles: Option<Roles<'_>>,
    any_number: bool,
) -> Option<String> {
    if function.r#async || function.generator {
        return None;
    }
    let body = function.body.as_ref()?;
    let mut locals = Locals {
        found: FxHashSet::default(),
        called: FxHashSet::default(),
        functions: Vec::new(),
        budget: NODE_BUDGET,
    };
    locals.visit_formal_parameters(&function.params);
    locals.visit_function_body(body);
    if locals.budget == 0 {
        return None;
    }
    let callables = locals.callables(scoping);
    let mut shape = Shape {
        scoping,
        roles,
        any_number,
        callables,
        locals: locals.found,
        slots: FxHashMap::default(),
        globals: Vec::new(),
        text: String::new(),
        poisoned: false,
    };
    shape.visit_formal_parameters(&function.params);
    shape.visit_function_body(body);
    shape.finish()
}

#[derive(Default)]
struct Templates {
    callbacks: FxHashSet<String>,
    protections: FxHashSet<String>,
    intervals: FxHashSet<String>,
}

fn load() -> Templates {
    let alloc = Allocator::default();
    let parsed = parse_with(&alloc, TEMPLATES, SourceType::cjs());
    let semantics = build_scoping(&parsed.program);
    let mut templates = Templates::default();
    for stmt in &parsed.program.body {
        let Statement::FunctionDeclaration(function) = stmt else {
            continue;
        };
        let name = function.id.as_ref().map_or("", |id| id.name.as_str());
        let interval = name.starts_with("interval_");
        let Some(text) = shape(function, &semantics.scoping, None, interval) else {
            continue;
        };
        if interval {
            templates.intervals.insert(text);
        } else if name.starts_with("protection_") {
            templates.protections.insert(text);
        } else {
            templates.callbacks.insert(text);
        }
    }
    templates
}

fn templates() -> &'static Templates {
    static LOADED: OnceLock<Templates> = OnceLock::new();
    LOADED.get_or_init(load)
}

pub(crate) fn is_protection_callback(
    callback: &Expression<'_>,
    roles: Roles<'_>,
    scoping: &Scoping,
) -> bool {
    let Expression::FunctionExpression(function) = callback.without_parentheses() else {
        return false;
    };
    function.id.is_none()
        && shape(function, scoping, Some(roles), false)
            .is_some_and(|text| templates().callbacks.contains(&text))
}

pub(crate) fn is_protection_function(function: &Function<'_>, scoping: &Scoping) -> bool {
    shape(function, scoping, Some(Roles::default()), false)
        .is_some_and(|text| templates().protections.contains(&text))
}

pub(crate) fn is_interval_function(
    function: &Function<'_>,
    protections: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    let roles = Roles {
        protections: Some(protections),
        ..Roles::default()
    };
    shape(function, scoping, Some(roles), true)
        .is_some_and(|text| templates().intervals.contains(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_in_the_file_has_a_shape() {
        let declared = TEMPLATES.matches("\nfunction ").count() + 1;
        let loaded = templates();
        let shaped = loaded.callbacks.len() + loaded.protections.len() + loaded.intervals.len();
        assert_eq!(shaped, declared);
        assert_eq!(loaded.protections.len(), 1);
        assert_eq!(loaded.intervals.len(), 2);
    }
}
