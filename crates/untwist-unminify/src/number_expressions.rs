use oxc_ast::ast::*;
use oxc_span::{ContentEq, GetSpan};
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};
use untwist_eval::fold::{const_from_literal, const_to_expression, fold_binary, fold_unary, Const};

use crate::merge_strings::{literal_chain, waits_for_its_chain_root};

#[derive(Default)]
pub struct NumberExpressions;

fn is_folded_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Addition
            | BinaryOperator::Subtraction
            | BinaryOperator::Division
            | BinaryOperator::Remainder
            | BinaryOperator::Multiplication
            | BinaryOperator::Exponential
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
            | BinaryOperator::ShiftLeft
            | BinaryOperator::BitwiseXOR
    )
}

fn operand(expr: &Expression<'_>) -> Option<Const> {
    match expr {
        Expression::StringLiteral(_) | Expression::NumericLiteral(_) => const_from_literal(expr),
        Expression::UnaryExpression(unary)
            if unary.operator == UnaryOperator::UnaryNegation
                && matches!(unary.argument, Expression::NumericLiteral(_)) =>
        {
            const_from_literal(expr)
        }
        _ => None,
    }
}

fn folded(expr: &Expression<'_>) -> Option<Const> {
    match expr {
        Expression::UnaryExpression(unary) => {
            if unary.operator != UnaryOperator::UnaryNegation {
                return None;
            }
            if !matches!(
                unary.argument,
                Expression::StringLiteral(_) | Expression::NumericLiteral(_)
            ) {
                return None;
            }
            let argument = const_from_literal(&unary.argument)?;
            fold_unary(UnaryOperator::UnaryNegation, &argument)
        }
        Expression::BinaryExpression(binary) => {
            if !is_folded_operator(binary.operator) {
                return None;
            }
            let left = operand(&binary.left)?;
            let right = operand(&binary.right)?;
            let result = fold_binary(binary.operator, &left, &right)?;
            if binary.operator == BinaryOperator::Division {
                match &result {
                    Const::Num(value) if value.fract() == 0.0 => {}
                    _ => return None,
                }
            }
            Some(result)
        }
        _ => None,
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::BinaryExpression,
    ExpressionKind::UnaryExpression,
]);

impl<'a> Pass<'a> for NumberExpressions {
    fn name(&self) -> &'static str {
        "number-expressions"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if waits_for_its_chain_root(expr, cx) {
            return;
        }
        if let Some((text, links)) = literal_chain(expr, cx) {
            let span = expr.span();
            if let Some(merged) = const_to_expression(&Const::Str(text), span, &*cx) {
                *expr = merged;
                cx.changed_by(links);
            }
            return;
        }
        let Some(value) = folded(expr) else {
            return;
        };
        if let Const::Num(number) = &value {
            if *number == 0.0 && number.is_sign_negative() {
                return;
            }
        }
        let span = expr.span();
        let Some(replacement) = const_to_expression(&value, span, &*cx) else {
            return;
        };
        if expr.content_eq(&replacement) {
            return;
        }
        *expr = replacement;
        cx.changed();
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(NumberExpressions)
}
