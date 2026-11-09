use oxc_ast::ast::*;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use super::stored::Facts;
use crate::value_flow::{flow_rule_disabled, OPAQUE_BRANCHES};

const MAX_PEEL: usize = 8;

pub(super) fn choice(test: &Expression<'_>, facts: Facts<'_, '_>) -> Option<bool> {
    if flow_rule_disabled(OPAQUE_BRANCHES) {
        return None;
    }
    folded(test, facts)
}

fn equal(operator: BinaryOperator) -> Option<bool> {
    match operator {
        BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(true),
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => Some(false),
        _ => None,
    }
}

fn texts_equal(
    left: &Expression<'_>,
    right: &Expression<'_>,
    facts: Facts<'_, '_>,
) -> Option<bool> {
    let left = facts.text(left, 0)?;
    let right = facts.text(right, 0)?;
    Some(left == right)
}

fn folded(test: &Expression<'_>, facts: Facts<'_, '_>) -> Option<bool> {
    match test.without_parentheses() {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            folded(&unary.argument, facts).map(|value| !value)
        }
        Expression::BinaryExpression(binary) => {
            let wanted = equal(binary.operator)?;
            texts_equal(&binary.left, &binary.right, facts).map(|same| same == wanted)
        }
        Expression::CallExpression(call) => {
            let wanted = equal(facts.compare(call)?)?;
            let [left, right] = call.arguments.as_slice() else {
                return None;
            };
            texts_equal(left.as_expression()?, right.as_expression()?, facts)
                .map(|same| same == wanted)
        }
        _ => None,
    }
}

pub(super) fn live<'k, 'a>(
    statement: &'k Statement<'a>,
    facts: Facts<'_, '_>,
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
        let Some(taken) = choice(&it.test, facts) else {
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
