use oxc_ast::ast::*;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceFlags;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};

#[derive(Default)]
pub struct Infinity;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Infinity)
}

fn is_numeric_literal(expr: &Expression<'_>, value: f64) -> bool {
    matches!(expr, Expression::NumericLiteral(literal) if literal.value == value)
}

fn is_negative_one(expr: &Expression<'_>) -> bool {
    let Expression::UnaryExpression(unary) = expr else {
        return false;
    };
    unary.operator == UnaryOperator::UnaryNegation && is_numeric_literal(&unary.argument, 1.0)
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for Infinity {
    fn name(&self) -> &'static str {
        "infinity"
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let (span, negative) = {
            let Expression::BinaryExpression(binary) = &*expr else {
                return;
            };
            if binary.operator != BinaryOperator::Division {
                return;
            }
            if !is_numeric_literal(&binary.right, 0.0) {
                return;
            }
            if is_numeric_literal(&binary.left, 1.0) {
                (binary.span, false)
            } else if is_negative_one(&binary.left) {
                (binary.span, true)
            } else {
                return;
            }
        };
        if !cx.is_global_name("Infinity") {
            return;
        }
        let infinity =
            cx.tcx
                .create_unbound_ident_expr(span, Ident::from("Infinity"), ReferenceFlags::Read);
        *expr = if negative {
            Expression::new_unary_expression(span, UnaryOperator::UnaryNegation, infinity, &*cx)
        } else {
            infinity
        };
        cx.changed();
    }
}
