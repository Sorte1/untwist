use oxc_ast::ast::*;
use oxc_span::{GetSpan, SPAN};
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct InvertBooleanLogic;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(InvertBooleanLogic)
}

fn inverted_binary(operator: BinaryOperator) -> Option<BinaryOperator> {
    match operator {
        BinaryOperator::Equality => Some(BinaryOperator::Inequality),
        BinaryOperator::Inequality => Some(BinaryOperator::Equality),
        BinaryOperator::StrictEquality => Some(BinaryOperator::StrictInequality),
        BinaryOperator::StrictInequality => Some(BinaryOperator::StrictEquality),
        _ => None,
    }
}

fn inverted_logical(operator: LogicalOperator) -> Option<LogicalOperator> {
    match operator {
        LogicalOperator::Or => Some(LogicalOperator::And),
        LogicalOperator::And => Some(LogicalOperator::Or),
        LogicalOperator::Coalesce => None,
    }
}

fn is_invertible_logical(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::LogicalExpression(logical)
        if inverted_logical(logical.operator).is_some())
}

fn negate<'a>(slot: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
    if let Expression::BinaryExpression(binary) = slot {
        if let Some(operator) = inverted_binary(binary.operator) {
            binary.operator = operator;
            return;
        }
    }
    if is_invertible_logical(slot) {
        invert_logical_spine(slot, cx);
        return;
    }
    let placeholder = Expression::new_null_literal(SPAN, &*cx);
    let inner = std::mem::replace(slot, placeholder);
    let span = inner.span();
    *slot = Expression::new_unary_expression(span, UnaryOperator::LogicalNot, inner, &*cx);
}

fn invert_logical_spine<'a>(argument: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
    let mut current = argument;
    loop {
        let Expression::LogicalExpression(logical) = current else {
            return;
        };
        let Some(operator) = inverted_logical(logical.operator) else {
            return;
        };
        logical.operator = operator;
        negate(&mut logical.right, cx);
        if !is_invertible_logical(&logical.left) {
            negate(&mut logical.left, cx);
            return;
        }
        current = &mut logical.left;
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::UnaryExpression]);

impl<'a> Pass<'a> for InvertBooleanLogic {
    fn name(&self) -> &'static str {
        "invert-boolean-logic"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let mut argument = {
            let Expression::UnaryExpression(unary) = &*expr else {
                return;
            };
            if unary.operator != UnaryOperator::LogicalNot {
                return;
            }
            let invertible = match &unary.argument {
                Expression::BinaryExpression(binary) => inverted_binary(binary.operator).is_some(),
                Expression::LogicalExpression(logical) => {
                    inverted_logical(logical.operator).is_some()
                }
                _ => false,
            };
            if !invertible {
                return;
            }
            let placeholder = Expression::new_null_literal(SPAN, &*cx);
            let old = std::mem::replace(expr, placeholder);
            let unary = match old {
                Expression::UnaryExpression(unary) => unary,
                other => {
                    *expr = other;
                    return;
                }
            };
            unary.unbox().argument
        };
        if let Expression::BinaryExpression(binary) = &mut argument {
            match inverted_binary(binary.operator) {
                Some(operator) => binary.operator = operator,
                None => {
                    *expr = argument;
                    return;
                }
            }
        } else {
            invert_logical_spine(&mut argument, cx);
        }
        *expr = argument;
        cx.changed();
    }
}
