use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::ast_util::literal::{is_false_literal, is_true_literal};
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};

#[derive(Default)]
pub struct UnminifyBooleans;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UnminifyBooleans)
}

fn is_numeric_one(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::NumericLiteral(literal) if literal.value == 1.0
    )
}

fn minified_boolean(expr: &Expression<'_>) -> Option<bool> {
    let Expression::UnaryExpression(unary) = expr else {
        return None;
    };
    if unary.operator != UnaryOperator::LogicalNot {
        return None;
    }
    if is_true_literal(expr) {
        return Some(true);
    }
    if is_false_literal(expr) || is_numeric_one(&unary.argument) {
        return Some(false);
    }
    None
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::UnaryExpression]);

impl<'a> Pass<'a> for UnminifyBooleans {
    fn name(&self) -> &'static str {
        "unminify-booleans"
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Some(value) = minified_boolean(expr) else {
            return;
        };
        let span = expr.span();
        *expr = Expression::new_boolean_literal(span, value, cx);
        cx.changed();
    }
}
