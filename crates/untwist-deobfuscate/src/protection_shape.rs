use std::fmt::Write;
use std::sync::OnceLock;

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::identifier::is_identifier_name;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::global::{global_is_written, is_global_reference};
use untwist_core::ast_util::js_text::js_text;
use untwist_core::parse::{parse_with, SourceType};
use untwist_core::semantic::build_scoping;
use untwist_eval::value::JsString;

use crate::protection_proxies::{Proxies, Proxy};

pub(crate) type Decoded<'k> = Option<&'k FxHashMap<Span, &'k JsString>>;

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
    pub(crate) proxies: Option<&'k Proxies>,
    pub(crate) arrows: bool,
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
    decoded: Decoded<'k>,
    roles: Option<Roles<'k>>,
    any_number: bool,
    locals: FxHashSet<SymbolId>,
    callables: FxHashSet<Span>,
    slots: FxHashMap<Slot, usize>,
    globals: Vec<String>,
    text: String,
    poisoned: bool,
    named_keys: FxHashSet<Span>,
    skipped_redeclared: FxHashSet<SymbolId>,
    bound_redeclared: FxHashSet<SymbolId>,
}

impl Shape<'_> {
    fn node_type(&mut self, kind: AstKind<'_>) -> String {
        match kind {
            AstKind::ComputedMemberExpression(member) => match &member.expression {
                Expression::StringLiteral(key) if is_identifier_name(&js_text(key.value)) => {
                    self.named_keys.insert(key.span);
                    "StaticMemberExpression".to_string()
                }
                _ => format!("{:?}", kind.ty()),
            },
            AstKind::StringLiteral(literal) if self.named_keys.contains(&literal.span) => {
                "IdentifierName".to_string()
            }
            _ => format!("{:?}", kind.ty()),
        }
    }

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
        if !self.scoping.symbol_redeclarations(symbol).is_empty() {
            self.bound_redeclared.insert(symbol);
        }
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
                | AstKind::ConditionalExpression(_)
                | AstKind::ThisExpression(_)
                | AstKind::WhileStatement(_)
                | AstKind::DebuggerStatement(_)
        )
    }

    fn finish(self) -> Option<String> {
        if self.poisoned || !self.skipped_redeclared.is_disjoint(&self.bound_redeclared) {
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
        let ty = self.node_type(kind);
        let _ = write!(self.text, "({ty}");
        let known = self.leaf(kind) || self.operator(kind) || self.flagged(kind);
        self.poisoned |= !known && !Self::is_plain(kind);
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if !matches!(kind, AstKind::ParenthesizedExpression(_)) {
            self.text.push(')');
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.poisoned {
            return;
        }
        match unread_bare_var(it, self.scoping) {
            Some(redeclared) => self.skipped_redeclared.extend(redeclared),
            None => walk_js::walk_statement(self, it),
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.poisoned {
            return;
        }
        if let Some(Proxy::Text(text)) = self.proxy(it) {
            return self.literal(text);
        }
        if let Some(text) = self.decoded_text(it) {
            return self.literal(&text);
        }
        walk_js::walk_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if !self.proxied_call(it) {
            walk_js::walk_call_expression(self, it);
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        match self.decoded_text(&it.expression) {
            Some(key) => self.static_member(it, &key),
            None => walk_js::walk_computed_member_expression(self, it),
        }
    }

    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if self.decoded.is_none() {
            return walk_js::walk_variable_declaration(self, it);
        }
        for declarator in &it.declarations {
            if self.decoder_alias(declarator) {
                continue;
            }
            self.text.push_str("(VariableDeclaration");
            self.declaration_kind(it.kind);
            self.visit_variable_declarator(declarator);
            self.text.push(')');
        }
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        self.text.push_str("(FunctionBody");
        for directive in &it.directives {
            self.visit_directive(directive);
        }
        for statement in unbraced(&it.statements) {
            self.visit_statement(statement);
        }
        self.text.push(')');
    }

    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        let sequence = match it.expression.without_parentheses() {
            Expression::SequenceExpression(sequence) if self.decoded.is_some() => sequence,
            _ => return walk_js::walk_expression_statement(self, it),
        };
        for expression in &sequence.expressions {
            self.text.push_str("(ExpressionStatement");
            self.visit_expression(expression);
            self.text.push(')');
        }
    }
}

fn unbraced<'k, 'a>(mut statements: &'k [Statement<'a>]) -> &'k [Statement<'a>] {
    while let [Statement::BlockStatement(block)] = statements {
        statements = &block.body;
    }
    statements
}

fn is_plain_var_redeclared(symbol: SymbolId, scoping: &Scoping) -> bool {
    scoping.symbol_flags(symbol) == SymbolFlags::FunctionScopedVariable
        && scoping
            .symbol_redeclarations(symbol)
            .iter()
            .all(|redeclaration| redeclaration.flags == SymbolFlags::FunctionScopedVariable)
}

fn unread_bare_var(stmt: &Statement<'_>, scoping: &Scoping) -> Option<Vec<SymbolId>> {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Var {
        return None;
    }
    let mut redeclared = Vec::new();
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        let symbol = id.symbol_id.get()?;
        if declarator.init.is_some() || !scoping.get_resolved_reference_ids(symbol).is_empty() {
            return None;
        }
        if scoping.symbol_redeclarations(symbol).is_empty() {
            continue;
        }
        if !is_plain_var_redeclared(symbol, scoping) {
            return None;
        }
        redeclared.push(symbol);
    }
    Some(redeclared)
}

impl<'k, 'a> Shape<'k> {
    fn proxy(&self, expr: &Expression<'a>) -> Option<&'k Proxy> {
        let proxies = self.roles?.proxies?;
        proxies.member(expr, self.scoping)
    }

    fn proxied_call(&mut self, it: &CallExpression<'a>) -> bool {
        if it.optional {
            return false;
        }
        let arguments: Option<Vec<&Expression<'a>>> =
            it.arguments.iter().map(Argument::as_expression).collect();
        let (Some(proxy), Some(arguments)) = (self.proxy(&it.callee), arguments) else {
            return false;
        };
        match (proxy, arguments.as_slice()) {
            (Proxy::Call(count), [callee, rest @ ..]) if rest.len() == *count => {
                self.text.push_str("(CallExpression");
                self.tag(false);
                self.visit_expression(callee);
                for argument in rest {
                    self.visit_expression(argument);
                }
            }
            (Proxy::Binary(operator), [left, right]) => {
                self.text.push_str("(BinaryExpression");
                self.tag(*operator);
                self.visit_expression(left);
                self.visit_expression(right);
            }
            _ => return false,
        }
        self.text.push(')');
        true
    }

    fn decoded_text(&self, expr: &Expression<'a>) -> Option<String> {
        let decoded = self.decoded?;
        match expr.without_parentheses() {
            Expression::CallExpression(call) => decoded.get(&call.span)?.as_string(),
            Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
                let left = self.piece(&binary.left)?;
                let right = self.piece(&binary.right)?;
                Some(left + &right)
            }
            Expression::StringLiteral(text) if self.decoded.is_some() => {
                Some(js_text(text.value).into_owned())
            }
            _ => None,
        }
    }

    fn piece(&self, expr: &Expression<'a>) -> Option<String> {
        self.decoded_text(expr)
    }

    fn literal(&mut self, text: &str) {
        self.text.push_str("(StringLiteral");
        self.tag(text);
        self.text.push(')');
    }

    fn static_member(&mut self, it: &ComputedMemberExpression<'a>, key: &str) {
        self.text.push_str("(StaticMemberExpression");
        self.tag(it.optional);
        self.visit_expression(&it.object);
        self.text.push_str("(IdentifierName");
        self.tag(key);
        self.text.push_str("))");
    }

    fn decoder_alias(&self, declarator: &VariableDeclarator<'a>) -> bool {
        let (BindingPattern::BindingIdentifier(_), Some(Expression::Identifier(init))) = (
            &declarator.id,
            declarator
                .init
                .as_ref()
                .map(Expression::without_parentheses),
        ) else {
            return false;
        };
        init.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| !self.locals.contains(&symbol))
    }
}

fn shape<'k>(
    function: &Function<'_>,
    scoping: &'k Scoping,
    roles: Option<Roles<'k>>,
    (any_number, decoded): (bool, Decoded<'k>),
) -> Option<String> {
    if function.r#async || function.generator {
        return None;
    }
    let body = function.body.as_ref()?;
    shape_of(
        &function.params,
        body,
        scoping,
        roles,
        (any_number, decoded),
    )
}

fn shape_of<'k>(
    params: &FormalParameters<'_>,
    body: &FunctionBody<'_>,
    scoping: &'k Scoping,
    roles: Option<Roles<'k>>,
    (any_number, decoded): (bool, Decoded<'k>),
) -> Option<String> {
    let mut locals = Locals {
        found: FxHashSet::default(),
        called: FxHashSet::default(),
        functions: Vec::new(),
        budget: NODE_BUDGET,
    };
    locals.visit_formal_parameters(params);
    locals.visit_function_body(body);
    if locals.budget == 0 {
        return None;
    }
    let callables = locals.callables(scoping);
    let mut shape = Shape {
        scoping,
        decoded,
        roles,
        any_number,
        callables,
        locals: locals.found,
        slots: FxHashMap::default(),
        globals: Vec::new(),
        text: String::new(),
        poisoned: false,
        named_keys: FxHashSet::default(),
        skipped_redeclared: FxHashSet::default(),
        bound_redeclared: FxHashSet::default(),
    };
    shape.visit_formal_parameters(params);
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
        let Some(text) = shape(function, &semantics.scoping, None, (interval, None)) else {
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
    is_decoded_protection_callback(callback, roles, scoping, None)
}

pub(crate) fn is_decoded_protection_callback<'k>(
    callback: &Expression<'_>,
    roles: Roles<'k>,
    scoping: &'k Scoping,
    decoded: Decoded<'k>,
) -> bool {
    let text = match callback.without_parentheses() {
        Expression::FunctionExpression(function) if function.id.is_none() => {
            shape(function, scoping, Some(roles), (false, decoded))
        }
        Expression::ArrowFunctionExpression(arrow) if roles.arrows && !arrow.r#async => {
            match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => {
                    shape_of(&arrow.params, body, scoping, Some(roles), (false, decoded))
                }
                _ => None,
            }
        }
        _ => None,
    };
    text.is_some_and(|text| templates().callbacks.contains(&text))
}

pub(crate) fn is_protection_function(function: &Function<'_>, scoping: &Scoping) -> bool {
    is_decoded_protection_function(function, scoping, None)
}

pub(crate) fn is_decoded_protection_function<'k>(
    function: &Function<'_>,
    scoping: &'k Scoping,
    decoded: Decoded<'k>,
) -> bool {
    shape(function, scoping, Some(Roles::default()), (false, decoded))
        .is_some_and(|text| templates().protections.contains(&text))
}

pub(crate) fn is_interval_function(
    function: &Function<'_>,
    protections: &FxHashSet<SymbolId>,
    scoping: &Scoping,
) -> bool {
    is_decoded_interval_function(function, protections, scoping, None)
}

pub(crate) fn is_decoded_interval_function<'k>(
    function: &Function<'_>,
    protections: &'k FxHashSet<SymbolId>,
    scoping: &'k Scoping,
    decoded: Decoded<'k>,
) -> bool {
    let roles = Roles {
        protections: Some(protections),
        ..Roles::default()
    };
    shape(function, scoping, Some(roles), (true, decoded))
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
        assert_eq!(loaded.protections.len(), 2);
        assert_eq!(loaded.intervals.len(), 4);
    }
}
