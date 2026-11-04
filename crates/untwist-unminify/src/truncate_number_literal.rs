use oxc_ast::ast::*;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};
use untwist_eval::coerce::to_int32;

#[derive(Default)]
pub struct TruncateNumberLiteral;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(TruncateNumberLiteral)
}

fn is_bitwise(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
    )
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for TruncateNumberLiteral {
    fn name(&self) -> &'static str {
        "truncate-number-literal"
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
        if !is_bitwise(binary.operator) {
            return;
        }
        let left_is_literal = matches!(binary.left, Expression::NumericLiteral(_));
        let is_shifter = !left_is_literal
            && matches!(
                binary.operator,
                BinaryOperator::ShiftLeft | BinaryOperator::ShiftRight
            );
        let operand = if left_is_literal {
            &mut binary.left
        } else {
            &mut binary.right
        };
        let Expression::NumericLiteral(literal) = operand else {
            return;
        };
        let value = literal.value;
        let truncated = if is_shifter {
            f64::from(to_int32(value) & 31)
        } else {
            f64::from(to_int32(value))
        };
        if truncated == value {
            return;
        }
        literal.value = truncated;
        literal.raw = None;
        literal.base = NumberBase::Decimal;
        cx.changed();
    }
}
