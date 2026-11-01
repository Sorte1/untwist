use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::ast_util::matcher::{is_literal_argument, statement_iife};

use crate::stamp::string_array_unit;

pub const ROTATOR_GLOBALS: &[&str] = &["parseInt", "undefined", "NaN", "Infinity"];

struct Sealed<'s> {
    scoping: &'s Scoping,
    scope: ScopeId,
    allowed: &'s FxHashSet<SymbolId>,
    fail: Option<Span>,
    array_param: Option<SymbolId>,
    array_param_uses: Vec<Span>,
    array_use: Option<Span>,
}

impl Sealed<'_> {
    fn declared_inside(&self, symbol: SymbolId) -> bool {
        let declared = self.scoping.symbol_scope_id(symbol);
        self.scoping
            .scope_ancestors(declared)
            .any(|scope| scope == self.scope)
    }

    fn resolve(&self, id: &IdentifierReference<'_>) -> Option<Option<SymbolId>> {
        Some(
            self.scoping
                .get_reference(id.reference_id.get()?)
                .symbol_id(),
        )
    }

    fn local(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        matches!(self.resolve(id), Some(Some(symbol)) if self.declared_inside(symbol))
    }

    fn fail_at(&mut self, span: Span) {
        self.fail.get_or_insert(span);
    }

    fn is_array(&self, id: &IdentifierReference<'_>, symbol: SymbolId) -> bool {
        self.allowed.contains(&symbol) && string_array_unit(id.name.as_str()).is_some()
    }

    fn note_machinery_use(&mut self, it: &IdentifierReference<'_>, symbol: SymbolId) {
        if self.array_param == Some(symbol) {
            self.array_param_uses.push(it.span);
        }
        if self.is_array(it, symbol) {
            self.array_use.get_or_insert(it.span);
        }
    }
}

impl<'a> VisitJs<'a> for Sealed<'_> {
    fn visit_class(&mut self, it: &Class<'a>) {
        self.fail_at(it.span);
    }

    fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
        self.fail_at(it.span);
    }

    fn visit_tagged_template_expression(&mut self, it: &TaggedTemplateExpression<'a>) {
        self.fail_at(it.span);
    }

    fn visit_this_expression(&mut self, it: &ThisExpression) {
        self.fail_at(it.span);
    }

    fn visit_assignment_target_pattern(&mut self, it: &AssignmentTargetPattern<'a>) {
        self.fail_at(it.span());
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        if it.operator == UnaryOperator::Delete {
            self.fail_at(it.span);
        }
        walk_js::walk_unary_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let ok = match self.resolve(it) {
            Some(Some(symbol)) => {
                self.note_machinery_use(it, symbol);
                self.declared_inside(symbol) || self.allowed.contains(&symbol)
            }
            Some(None) => ROTATOR_GLOBALS.contains(&it.name.as_str()),
            None => false,
        };
        if !ok {
            self.fail_at(it.span);
        }
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        let ok = match it {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(id) => {
                matches!(self.resolve(id), Some(Some(symbol)) if self.declared_inside(symbol))
            }
            _ => false,
        };
        if !ok {
            self.fail_at(it.span());
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        let ok = match it.callee.without_parentheses() {
            Expression::Identifier(_) => true,
            callee => callee.as_member_expression().is_some_and(|member| {
                matches!(member.static_property_name(), Some("push" | "shift"))
                    && self.local(member.object())
            }),
        };
        if !ok {
            self.fail_at(it.span);
        }
        walk_js::walk_call_expression(self, it);
    }
}

fn simple_parameters(params: &FormalParameters<'_>) -> bool {
    params.rest.is_none()
        && params.items.iter().all(|param| {
            param.initializer.is_none()
                && matches!(param.pattern, BindingPattern::BindingIdentifier(_))
        })
}

fn array_argument_index(
    scoping: &Scoping,
    arguments: &[Argument<'_>],
) -> Result<Option<usize>, Span> {
    let mut found = None;
    for (index, argument) in arguments.iter().enumerate() {
        let Some(Expression::Identifier(id)) = argument.as_expression() else {
            continue;
        };
        let resolved = id
            .reference_id
            .get()
            .and_then(|r| scoping.get_reference(r).symbol_id());
        if resolved.is_none() || string_array_unit(id.name.as_str()).is_none() {
            continue;
        }
        if found.replace(index).is_some() {
            return Err(argument.span());
        }
    }
    Ok(found)
}

fn parameter_symbol(params: &FormalParameters<'_>, index: usize) -> Option<SymbolId> {
    match &params.items.get(index)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn runs_nothing(init: Option<&Expression<'_>>) -> bool {
    init.is_none_or(|init| {
        is_literal_argument(init) || matches!(init.without_parentheses(), Expression::Identifier(_))
    })
}

fn creates_the_array(declarator: &VariableDeclarator<'_>, callee: Span) -> bool {
    matches!(
        declarator.init.as_ref().map(Expression::without_parentheses),
        Some(Expression::CallExpression(call))
            if call.arguments.is_empty() && call.callee.without_parentheses().span() == callee
    )
}

enum Before {
    Created,
    Quiet,
    Ran,
}

fn before_the_creation(statement: &Statement<'_>, callee: Span) -> Before {
    match statement {
        Statement::FunctionDeclaration(_) | Statement::EmptyStatement(_) => Before::Quiet,
        Statement::VariableDeclaration(declaration) => {
            for declarator in &declaration.declarations {
                if creates_the_array(declarator, callee) {
                    return Before::Created;
                }
                if !runs_nothing(declarator.init.as_ref()) {
                    return Before::Ran;
                }
            }
            Before::Quiet
        }
        _ => Before::Ran,
    }
}

fn creates_the_array_first(body: &FunctionBody<'_>, callee: Span) -> bool {
    for statement in &body.statements {
        match before_the_creation(statement, callee) {
            Before::Created => return true,
            Before::Quiet => {}
            Before::Ran => return false,
        }
    }
    false
}

fn array_runs_first(seal: &Sealed<'_>, function: &Function<'_>) -> Option<Span> {
    if let Some(span) = seal.array_use {
        return Some(span);
    }
    match seal.array_param_uses.as_slice() {
        [] => None,
        [callee] => {
            let first = function
                .body
                .as_ref()
                .is_some_and(|body| creates_the_array_first(body, *callee));
            (!first).then_some(*callee)
        }
        [_, second, ..] => Some(*second),
    }
}

fn open_seal<'s>(
    function: &Function<'_>,
    arguments: &[Argument<'_>],
    scoping: &'s Scoping,
    allowed: &'s FxHashSet<SymbolId>,
) -> Result<Sealed<'s>, Span> {
    let scope = function.scope_id.get().ok_or(function.span)?;
    if !simple_parameters(&function.params) {
        return Err(function.params.span);
    }
    let array_param = array_argument_index(scoping, arguments)?
        .and_then(|index| parameter_symbol(&function.params, index));
    Ok(Sealed {
        scoping,
        scope,
        allowed,
        fail: None,
        array_param,
        array_param_uses: Vec::new(),
        array_use: None,
    })
}

pub(super) fn sealed(
    stmt: &Statement<'_>,
    scoping: &Scoping,
    allowed: &FxHashSet<SymbolId>,
) -> Option<Span> {
    let Some((function, arguments)) = statement_iife(stmt) else {
        return Some(stmt.span());
    };
    let mut seal = match open_seal(function, arguments, scoping, allowed) {
        Ok(seal) => seal,
        Err(span) => return Some(span),
    };
    for argument in arguments {
        let Some(expr) = argument.as_expression() else {
            return Some(argument.span());
        };
        if !is_literal_argument(expr) && !matches!(expr, Expression::Identifier(_)) {
            return Some(argument.span());
        }
        seal.visit_expression(expr);
    }
    seal.array_use = None;
    seal.visit_formal_parameters(&function.params);
    if let Some(body) = &function.body {
        seal.visit_function_body(body);
    }
    seal.fail.or_else(|| array_runs_first(&seal, function))
}
