use oxc_ast::ast::{Argument, ChainElement, Expression, MemberExpression};
use oxc_syntax::operator::UnaryOperator;

use super::eval::Leaves;
use super::resolved;
use super::Lax;

const MAX_DEPTH: u32 = 64;

pub(super) fn runs_no_code(expr: &Expression<'_>, lax: &Lax<'_, '_, '_, '_>) -> bool {
    Inert { lax }.expression(expr, 0)
}

struct Inert<'l, 'k, 's, 'r, 'p> {
    lax: &'l Lax<'k, 's, 'r, 'p>,
}

impl Inert<'_, '_, '_, '_, '_> {
    fn expression(&self, expr: &Expression<'_>, depth: u32) -> bool {
        if depth > MAX_DEPTH {
            return false;
        }
        let depth = depth + 1;
        match expr.without_parentheses() {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => true,
            Expression::Identifier(id) => {
                resolved(id, self.lax.scoping).is_none() || self.lax.leaf(id).is_some()
            }
            Expression::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::Delete => false,
                UnaryOperator::Typeof => self.typeof_operand(&unary.argument, depth),
                _ => self.expression(&unary.argument, depth),
            },
            Expression::BinaryExpression(binary) => {
                self.decided(expr)
                    || (self.expression(&binary.left, depth)
                        && self.expression(&binary.right, depth))
            }
            Expression::LogicalExpression(logical) => {
                self.decided(expr)
                    || (self.expression(&logical.left, depth)
                        && self.expression(&logical.right, depth))
            }
            Expression::ConditionalExpression(conditional) => {
                (self.decided(&conditional.test) || self.expression(&conditional.test, depth))
                    && self.expression(&conditional.consequent, depth)
                    && self.expression(&conditional.alternate, depth)
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => {
                    self.host_callee(&call.callee, depth) && self.arguments(&call.arguments, depth)
                }
                other => other
                    .as_member_expression()
                    .is_some_and(|member| self.host_member(member, depth)),
            },
            Expression::CallExpression(call) => {
                self.host_callee(&call.callee, depth) && self.arguments(&call.arguments, depth)
            }
            other => other
                .as_member_expression()
                .is_some_and(|member| self.host_member(member, depth)),
        }
    }

    fn decided(&self, test: &Expression<'_>) -> bool {
        self.lax.decided(test).is_some()
    }

    fn typeof_operand(&self, operand: &Expression<'_>, depth: u32) -> bool {
        match operand.without_parentheses() {
            Expression::Identifier(_) => true,
            other => self.expression(other, depth),
        }
    }

    fn arguments(&self, arguments: &[Argument<'_>], depth: u32) -> bool {
        arguments.iter().all(|argument| {
            argument
                .as_expression()
                .is_some_and(|argument| self.expression(argument, depth))
        })
    }

    fn host_callee(&self, callee: &Expression<'_>, depth: u32) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(id) => resolved(id, self.lax.scoping).is_none(),
            other => other
                .as_member_expression()
                .is_some_and(|member| self.host_member(member, depth)),
        }
    }

    fn host_member(&self, member: &MemberExpression<'_>, depth: u32) -> bool {
        if let MemberExpression::ComputedMemberExpression(computed) = member {
            if !self.expression(&computed.expression, depth) {
                return false;
            }
        }
        if matches!(member, MemberExpression::PrivateFieldExpression(_)) {
            return false;
        }
        match member.object().without_parentheses() {
            Expression::Identifier(id) => resolved(id, self.lax.scoping).is_none(),
            other => other
                .as_member_expression()
                .is_some_and(|inner| self.host_member(inner, depth)),
        }
    }
}
