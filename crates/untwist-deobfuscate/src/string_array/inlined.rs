use oxc_allocator::{TakeIn, Vec as AVec};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitJs, VisitMut};
use oxc_semantic::Scoping;
use oxc_span::GetSpan;
use oxc_syntax::operator::{AssignmentOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;
use untwist_core::ast_util::matcher::iife;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::ast_util::respan::SYNTHETIC_BASE;
use untwist_core::pass::Cx;

use super::array_declaration;
use super::checksum::parsed_statement;
use super::legacy::{incremented, is_countdown, literal_arithmetic};
use crate::mutants::inlined_rotator_vars_trusted;
use crate::stamp::DynamicScopes;
struct Plan {
    first: usize,
    array_name: String,
    target: String,
    counter: String,
    count: usize,
}

fn assigned<'k, 'a>(stmt: &'k Statement<'a>) -> Option<(&'k str, &'k Expression<'a>)> {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return None;
    };
    let Expression::AssignmentExpression(assignment) = stmt.expression.without_parentheses() else {
        return None;
    };
    let AssignmentTarget::AssignmentTargetIdentifier(target) = &assignment.left else {
        return None;
    };
    (assignment.operator == AssignmentOperator::Assign)
        .then_some((target.name.as_str(), &assignment.right))
}

fn whole(value: f64) -> Option<usize> {
    let fits = value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value);
    fits.then_some(value as usize)
}

fn countdown_call(stmt: &Statement<'_>, target: &str, counter: &str) -> bool {
    let Statement::ExpressionStatement(stmt) = stmt else {
        return false;
    };
    let call = match stmt.expression.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            unary.argument.without_parentheses()
        }
        other => other,
    };
    iife(call).is_some_and(|(function, arguments)| {
        is_countdown(function, target) && incremented(arguments, counter)
    })
}

fn rotation(statements: &[Statement<'_>], array: &str) -> Option<(String, String, usize)> {
    let [first, second, call] = statements else {
        return None;
    };
    let (first, second) = (assigned(first)?, assigned(second)?);
    let ((target, _), (counter, count)) =
        match (identifier_name(first.1), identifier_name(second.1)) {
            (Some(name), _) if name == array => (first, second),
            (_, Some(name)) if name == array => (second, first),
            _ => return None,
        };
    let count = whole(literal_arithmetic(count)?)?;
    let distinct = target != counter && target != array && counter != array;
    (distinct && countdown_call(call, target, counter))
        .then(|| (target.to_string(), counter.to_string(), count))
}

fn var_declared(body: &[Statement<'_>], name: &str) -> Option<SymbolId> {
    body.iter().find_map(|stmt| {
        let Statement::VariableDeclaration(declaration) = stmt else {
            return None;
        };
        if declaration.kind != VariableDeclarationKind::Var {
            return None;
        }
        declaration
            .declarations
            .iter()
            .find_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(id) if id.name == name => id.symbol_id.get(),
                _ => None,
            })
    })
}

struct Counted<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    seen: usize,
}

impl<'a> VisitJs<'a> for Counted<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.seen += usize::from(symbol == Some(self.symbol));
    }
}

fn only_used_within(scoping: &Scoping, symbol: SymbolId, statements: &[Statement<'_>]) -> bool {
    let mut counted = Counted {
        scoping,
        symbol,
        seen: 0,
    };
    for stmt in statements {
        counted.visit_statement(stmt);
    }
    let sealed = inlined_rotator_vars_trusted()
        || counted.seen == scoping.get_resolved_reference_ids(symbol).len();
    sealed
        && scoping.symbol_redeclarations(symbol).is_empty()
        && scoping.symbol_scope_id(symbol) != scoping.root_scope_id()
        && DynamicScopes::of(scoping)
            .refusal(scoping, symbol)
            .is_none()
}

fn plan(body: &[Statement<'_>], scoping: &Scoping) -> Option<Plan> {
    (0..body.len().saturating_sub(3)).find_map(|index| {
        let (array, length) = array_declaration(&body[index])?;
        let rotator = body.get(index + 1..index + 4)?;
        let (target, counter, count) = rotation(rotator, array)?;
        let sealed = length > 0
            && [&target, &counter].iter().all(|name| {
                var_declared(body, name)
                    .is_some_and(|symbol| only_used_within(scoping, symbol, rotator))
            });
        let original = rotator
            .iter()
            .all(|stmt| stmt.span().start < SYNTHETIC_BASE);
        (sealed && original).then(|| Plan {
            first: index + 1,
            array_name: array.to_string(),
            target,
            counter,
            count,
        })
    })
}

fn wrapper<'a>(plan: &Plan, cx: &Cx<'a, '_>) -> Option<Statement<'a>> {
    let text = format!(
        "!function ({}, {}) {{ 0; }}({}, {});",
        plan.target, plan.counter, plan.array_name, plan.count
    );
    parsed_statement(&text, cx)
}

fn placeholder<'k, 'a>(statement: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    let Statement::ExpressionStatement(outer) = statement else {
        return None;
    };
    let Expression::UnaryExpression(unary) = &mut outer.expression else {
        return None;
    };
    let Expression::CallExpression(call) = unary.argument.without_parentheses_mut() else {
        return None;
    };
    let Expression::FunctionExpression(function) = call.callee.without_parentheses_mut() else {
        return None;
    };
    match function.body.as_mut()?.statements.first_mut()? {
        Statement::ExpressionStatement(placeholder) => Some(&mut placeholder.expression),
        _ => None,
    }
}

fn countdown_of<'k, 'a>(rotator: &'k mut Statement<'a>) -> Option<&'k mut Expression<'a>> {
    match rotator {
        Statement::ExpressionStatement(rotator) => Some(&mut rotator.expression),
        _ => None,
    }
}

fn rewrite<'a>(it: &mut AVec<'a, Statement<'a>>, plan: &Plan, cx: &Cx<'a, '_>) -> bool {
    let Some(mut wrapped) = wrapper(plan, cx) else {
        return false;
    };
    let (Some(slot), Some(countdown)) = (
        placeholder(&mut wrapped),
        countdown_of(&mut it[plan.first + 2]),
    ) else {
        return false;
    };
    *slot = countdown.take_in(cx);
    it[plan.first] = wrapped;
    it.remove(plan.first + 2);
    it.remove(plan.first + 1);
    it.retain(|stmt| !declares_only(stmt, &plan.target) && !declares_only(stmt, &plan.counter));
    true
}

fn declares_only(stmt: &Statement<'_>, name: &str) -> bool {
    let Statement::VariableDeclaration(declaration) = stmt else {
        return false;
    };
    matches!(declaration.declarations.as_slice(), [declarator]
        if declarator.init.is_none()
            && matches!(&declarator.id, BindingPattern::BindingIdentifier(id) if id.name == name))
}

struct Inline<'c, 'a, 'x> {
    cx: &'c Cx<'a, 'x>,
    changed: usize,
}

impl<'a> VisitMut<'a> for Inline<'_, 'a, '_> {
    fn visit_statements(&mut self, it: &mut AVec<'a, Statement<'a>>) {
        if self.changed == 0 {
            if let Some(plan) = plan(it, self.cx.scoping()) {
                self.changed += usize::from(rewrite(it, &plan, self.cx));
            }
        }
        walk_mut::walk_statements(self, it);
    }
}

pub(super) fn inline_assigned_rotators<'a>(program: &mut Program<'a>, cx: &Cx<'a, '_>) -> usize {
    let mut inline = Inline { cx, changed: 0 };
    inline.visit_program(program);
    inline.changed
}
