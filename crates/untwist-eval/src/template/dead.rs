use oxc_ast::ast::{
    Argument, AssignmentTarget, BindingPattern, CallExpression, Expression, MemberExpression,
    NewExpression, Statement,
};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashSet;

use super::terms::{context, wrapper_body, wrapper_params, Context};
use crate::stack;

const CONVERSIONS: [&str; 4] = ["parseInt", "parseFloat", "Number", "String"];

#[derive(Default)]
struct Calls {
    found: bool,
}

impl<'a> VisitJs<'a> for Calls {
    fn visit_assignment_target(&mut self, it: &AssignmentTarget<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_assignment_target(self, it));
    }

    fn visit_binding_pattern(&mut self, it: &BindingPattern<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_binding_pattern(self, it));
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_expression(self, it));
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        stack::maybe_grow_dyn(&mut || walk_js::walk_statement(self, it));
    }

    fn visit_call_expression(&mut self, _it: &CallExpression<'a>) {
        self.found = true;
    }

    fn visit_new_expression(&mut self, _it: &NewExpression<'a>) {
        self.found = true;
    }
}

fn calls_free(expr: &Expression<'_>) -> bool {
    let mut calls = Calls::default();
    calls.visit_expression(expr);
    !calls.found
}

fn constant(expr: &Expression<'_>, constants: &FxHashSet<&str>) -> bool {
    match expr {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::Identifier(id) => constants.contains(id.name.as_str()),
        Expression::ParenthesizedExpression(node) => constant(&node.expression, constants),
        Expression::UnaryExpression(node) => {
            matches!(
                node.operator,
                UnaryOperator::UnaryNegation
                    | UnaryOperator::UnaryPlus
                    | UnaryOperator::BitwiseNot
                    | UnaryOperator::LogicalNot
            ) && constant(&node.argument, constants)
        }
        Expression::BinaryExpression(node) => {
            !matches!(
                node.operator,
                BinaryOperator::In | BinaryOperator::Instanceof
            ) && constant(&node.left, constants)
                && constant(&node.right, constants)
        }
        _ => false,
    }
}

fn arguments<'s, 'a>(call: &'s CallExpression<'a>) -> Option<Vec<&'s Expression<'a>>> {
    call.arguments
        .iter()
        .map(|argument| match argument {
            Argument::SpreadElement(_) => None,
            other => Some(other.to_expression()),
        })
        .collect()
}

struct Dead<'s, 'a> {
    context: &'s Context<'a>,
    machinery: &'s [&'s str],
    constants: &'s FxHashSet<&'a str>,
    shadowed: &'s FxHashSet<&'a str>,
    nested: bool,
}

impl<'a> Dead<'_, 'a> {
    fn term(&self, expr: &'a Expression<'a>) -> bool {
        match expr {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::Identifier(_) => true,
            Expression::ParenthesizedExpression(node) => self.term(&node.expression),
            Expression::UnaryExpression(node) => self.term(&node.argument),
            Expression::BinaryExpression(node) => self.term(&node.left) && self.term(&node.right),
            Expression::LogicalExpression(node) => {
                calls_free(&node.left) && self.term(&node.left) && self.term(&node.right)
            }
            Expression::ConditionalExpression(node) => {
                calls_free(&node.test)
                    && self.term(&node.test)
                    && self.term(&node.consequent)
                    && self.term(&node.alternate)
            }
            Expression::CallExpression(call) => self.call(call),
            other => other
                .as_member_expression()
                .is_some_and(|member| self.member(member)),
        }
    }

    fn member(&self, member: &'a MemberExpression<'a>) -> bool {
        match member {
            MemberExpression::ComputedMemberExpression(node) => {
                self.term(&node.object) && self.term(&node.expression)
            }
            MemberExpression::StaticMemberExpression(node) => self.term(&node.object),
            MemberExpression::PrivateFieldExpression(_) => false,
        }
    }

    fn is_conversion(&self, name: &str) -> bool {
        CONVERSIONS.contains(&name)
            && !self.context.locals.contains(name)
            && !self.shadowed.contains(name)
            && !self.machinery.contains(&name)
    }

    fn call(&self, call: &'a CallExpression<'a>) -> bool {
        let (Expression::Identifier(callee), Some(arguments)) = (&call.callee, arguments(call))
        else {
            return false;
        };
        let name = callee.name.as_str();
        if self.is_conversion(name) {
            return arguments.into_iter().all(|argument| self.term(argument));
        }
        let literal = arguments
            .iter()
            .all(|argument| constant(argument, self.constants));
        match self.context.wrappers.get(name) {
            Some(function) if !self.nested && !self.shadowed.contains(name) => {
                literal && self.wrapper(function)
            }
            _ => literal,
        }
    }

    fn wrapper(&self, function: &'a oxc_ast::ast::Function<'a>) -> bool {
        let Some(body) = wrapper_body(function) else {
            return false;
        };
        let params = wrapper_params(function);
        let mut constants = self.constants.clone();
        constants.extend(params.iter().copied());
        let inner = Dead {
            context: self.context,
            machinery: self.machinery,
            constants: &constants,
            shadowed: &params,
            nested: true,
        };
        inner.term(body)
    }
}

pub fn dead_tail_provable<'a>(
    statements: &'a [Statement<'a>],
    terms: &[&'a Expression<'a>],
    machinery: &[&str],
) -> bool {
    let context = context(statements);
    let shadowed = FxHashSet::default();
    let dead = Dead {
        context: &context,
        machinery,
        constants: &context.primitives,
        shadowed: &shadowed,
        nested: false,
    };
    terms.iter().all(|term| dead.term(term))
}
