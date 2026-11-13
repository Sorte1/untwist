use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{walk, walk_mut, Visit, VisitMut};
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

use super::build::Rewrite;
use crate::runtime::synthetic_statements;

struct Setters<'r, 'a> {
    alloc: &'a Allocator,
    rewrites: &'r FxHashMap<u32, Rewrite>,
    failed: bool,
}

fn first_argument<'k, 'a>(expression: &'k mut Expression<'a>) -> Option<&'k mut Expression<'a>> {
    let Expression::CallExpression(call) = expression else {
        return None;
    };
    call.arguments.first_mut()?.as_expression_mut()
}

fn named_slot<'k, 'a>(expression: &'k mut Expression<'a>) -> Option<&'k mut Expression<'a>> {
    let Expression::StaticMemberExpression(member) =
        first_argument(expression)?.without_parentheses_mut()
    else {
        return None;
    };
    let Expression::ObjectExpression(object) = member.object.without_parentheses_mut() else {
        return None;
    };
    let ObjectPropertyKind::ObjectProperty(property) = object.properties.first_mut()? else {
        return None;
    };
    Some(&mut property.value)
}

impl<'a> Setters<'_, 'a> {
    fn call(&self, rewrite: &Rewrite, value: Expression<'a>) -> Option<Expression<'a>> {
        let text = match rewrite.named {
            true => format!(
                "{}({{ {}: 0 }}.{});",
                rewrite.setter, rewrite.binding, rewrite.binding
            ),
            false => format!("{}(0);", rewrite.setter),
        };
        let mut parsed = synthetic_statements(self.alloc, text)?;
        let Some(Statement::ExpressionStatement(statement)) = parsed.pop() else {
            return None;
        };
        let mut expression = statement.unbox().expression;
        let slot = match rewrite.named {
            true => named_slot(&mut expression)?,
            false => first_argument(&mut expression)?,
        };
        *slot = value;
        Some(expression)
    }
}

struct Check<'r> {
    rewrites: &'r FxHashMap<u32, Rewrite>,
    found: usize,
    wrong: bool,
}

impl<'a> Visit<'a> for Check<'_> {
    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let Some(rewrite) = self.rewrites.get(&it.span.start) {
            let plain = it.operator == AssignmentOperator::Assign
                && matches!(&it.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == rewrite.binding.as_str());
            match plain {
                true => self.found += 1,
                false => self.wrong = true,
            }
        }
        walk::walk_assignment_expression(self, it);
    }
}

impl<'a> VisitMut<'a> for Setters<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let Expression::AssignmentExpression(assignment) = it else {
            return;
        };
        let Some(rewrite) = self.rewrites.get(&assignment.span.start) else {
            return;
        };
        let builder = AstBuilder::new(self.alloc);
        let value = std::mem::replace(
            &mut assignment.right,
            Expression::new_null_literal(SPAN, &builder),
        );
        match self.call(rewrite, value) {
            Some(call) => *it = call,
            None => self.failed = true,
        }
    }
}

fn checked<'a>(
    alloc: &'a Allocator,
    rewrites: &FxHashMap<u32, Rewrite>,
    statements: &[Statement<'a>],
) -> bool {
    let mut check = Check {
        rewrites,
        found: 0,
        wrong: false,
    };
    for statement in statements {
        check.visit_statement(statement);
    }
    let setters = Setters {
        alloc,
        rewrites,
        failed: false,
    };
    let builder = AstBuilder::new(alloc);
    let templates = rewrites.values().all(|rewrite| {
        setters
            .call(rewrite, Expression::new_null_literal(SPAN, &builder))
            .is_some()
    });
    !check.wrong && check.found == rewrites.len() && templates
}

pub fn route<'a>(
    alloc: &'a Allocator,
    rewrites: &FxHashMap<u32, Rewrite>,
    statements: &mut [Statement<'a>],
) -> bool {
    if rewrites.is_empty() {
        return true;
    }
    if !checked(alloc, rewrites, statements) {
        return false;
    }
    let mut setters = Setters {
        alloc,
        rewrites,
        failed: false,
    };
    for statement in statements.iter_mut() {
        setters.visit_statement(statement);
    }
    !setters.failed
}
