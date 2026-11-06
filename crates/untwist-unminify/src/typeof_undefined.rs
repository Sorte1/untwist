use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct TypeofUndefined;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TypeofUndefined)
}

fn flipped(operator: BinaryOperator) -> Option<BinaryOperator> {
    match operator {
        BinaryOperator::GreaterThan => Some(BinaryOperator::StrictEquality),
        BinaryOperator::LessThan => Some(BinaryOperator::StrictInequality),
        _ => None,
    }
}

fn is_typeof(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Typeof
    )
}

fn is_u_literal(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::StringLiteral(literal) if literal.value == "u"
    )
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for TypeofUndefined {
    fn name(&self) -> &'static str {
        "typeof-undefined"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Expression::BinaryExpression(binary) = expr else {
            return;
        };
        let Some(operator) = flipped(binary.operator) else {
            return;
        };
        if !is_typeof(&binary.left) || !is_u_literal(&binary.right) {
            return;
        }
        let right_span = binary.right.span();
        binary.operator = operator;
        binary.right = Expression::new_string_literal(right_span, "undefined", None, cx);
        cx.changed();
    }
}
