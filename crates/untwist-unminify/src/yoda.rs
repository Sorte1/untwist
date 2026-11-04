use oxc_ast::ast::*;
use untwist_core::ast_util::global::pure_value_reads_globals;
use untwist_core::ast_util::literal::is_pure_value;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct Yoda;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(Yoda)
}

fn flipped(operator: BinaryOperator) -> Option<BinaryOperator> {
    match operator {
        BinaryOperator::Equality => Some(BinaryOperator::Equality),
        BinaryOperator::StrictEquality => Some(BinaryOperator::StrictEquality),
        BinaryOperator::Inequality => Some(BinaryOperator::Inequality),
        BinaryOperator::StrictInequality => Some(BinaryOperator::StrictInequality),
        BinaryOperator::GreaterThan => Some(BinaryOperator::LessThan),
        BinaryOperator::LessThan => Some(BinaryOperator::GreaterThan),
        BinaryOperator::GreaterEqualThan => Some(BinaryOperator::LessEqualThan),
        BinaryOperator::LessEqualThan => Some(BinaryOperator::GreaterEqualThan),
        BinaryOperator::Multiplication => Some(BinaryOperator::Multiplication),
        BinaryOperator::BitwiseXOR => Some(BinaryOperator::BitwiseXOR),
        BinaryOperator::BitwiseAnd => Some(BinaryOperator::BitwiseAnd),
        BinaryOperator::BitwiseOR => Some(BinaryOperator::BitwiseOR),
        _ => None,
    }
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::BinaryExpression]);

impl<'a> Pass<'a> for Yoda {
    fn name(&self) -> &'static str {
        "yoda"
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
        if !is_pure_value(&binary.left)
            || is_pure_value(&binary.right)
            || !pure_value_reads_globals(&binary.left, cx.scoping())
        {
            return;
        }
        binary.operator = operator;
        let BinaryExpression { left, right, .. } = &mut **binary;
        std::mem::swap(left, right);
        cx.changed();
    }
}
