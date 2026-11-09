use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use untwist_core::ast_util::js_text::js_text;

use super::bootstrap::Resolve;
use crate::value_flow::{flow_rule_disabled, OPAQUE_BRANCHES};

const MAX_PEEL: usize = 8;

fn text(expr: &Expression<'_>, resolve: Resolve<'_>, depth: usize) -> Option<String> {
    if depth > MAX_PEEL {
        return None;
    }
    match expr.without_parentheses() {
        Expression::StringLiteral(literal) => Some(js_text(literal.value).into_owned()),
        Expression::CallExpression(call) => resolve?.get(&call.span)?.as_string(),
        Expression::BinaryExpression(binary) if binary.operator == BinaryOperator::Addition => {
            let left = text(&binary.left, resolve, depth + 1)?;
            let right = text(&binary.right, resolve, depth + 1)?;
            Some(left + &right)
        }
        _ => None,
    }
}

pub(super) fn choice(test: &Expression<'_>, resolve: Resolve<'_>) -> Option<bool> {
    if flow_rule_disabled(OPAQUE_BRANCHES) {
        return None;
    }
    folded(test, resolve)
}

fn folded(test: &Expression<'_>, resolve: Resolve<'_>) -> Option<bool> {
    match test.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            folded(&unary.argument, resolve).map(|value| !value)
        }
        Expression::BinaryExpression(binary) => {
            let equal = match binary.operator {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => true,
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => false,
                _ => return None,
            };
            let left = text(&binary.left, resolve, 0)?;
            let right = text(&binary.right, resolve, 0)?;
            Some((left == right) == equal)
        }
        _ => None,
    }
}

pub(super) fn live<'k, 'a>(
    statement: &'k Statement<'a>,
    resolve: Resolve<'_>,
) -> Option<&'k Statement<'a>> {
    let mut current = statement;
    for _ in 0..MAX_PEEL {
        let inner = match current {
            Statement::BlockStatement(block) => match block.body.as_slice() {
                [only] => only,
                _ => return Some(current),
            },
            other => other,
        };
        let Statement::IfStatement(it) = inner else {
            return Some(current);
        };
        let Some(taken) = choice(&it.test, resolve) else {
            return Some(current);
        };
        current = if taken {
            &it.consequent
        } else {
            it.alternate.as_ref()?
        };
    }
    Some(current)
}
