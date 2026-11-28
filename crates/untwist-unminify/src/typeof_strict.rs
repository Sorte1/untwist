use oxc_ast::ast::*;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct TypeofStrict;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TypeofStrict)
}

fn strict(operator: BinaryOperator) -> Option<BinaryOperator> {
    match operator {
        BinaryOperator::Equality => Some(BinaryOperator::StrictEquality),
        BinaryOperator::Inequality => Some(BinaryOperator::StrictInequality),
        _ => None,
    }
}

fn is_typeof(expr: &Expression<'_>) -> bool {
    matches!(
        expr.without_parentheses(),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Typeof
    )
}

fn is_string(expr: &Expression<'_>) -> bool {
    is_typeof(expr)
        || matches!(
            expr.without_parentheses(),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
        )
}

fn compares_strings(binary: &BinaryExpression<'_>) -> bool {
    (is_typeof(&binary.left) || is_typeof(&binary.right))
        && is_string(&binary.left)
        && is_string(&binary.right)
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for TypeofStrict {
    fn name(&self) -> &'static str {
        "typeof-strict"
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
        let Some(operator) = strict(binary.operator) else {
            return;
        };
        if compares_strings(binary) {
            binary.operator = operator;
            cx.changed();
        }
    }
}
