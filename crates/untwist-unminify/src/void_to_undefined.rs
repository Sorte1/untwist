use oxc_ast::ast::*;
use oxc_span::GetSpan;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceFlags;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};

#[derive(Default)]
pub struct VoidToUndefined;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(VoidToUndefined)
}

fn is_void_zero(expr: &Expression<'_>) -> bool {
    let Expression::UnaryExpression(unary) = expr.without_parentheses() else {
        return false;
    };
    if unary.operator != UnaryOperator::Void {
        return false;
    }
    matches!(
        unary.argument.without_parentheses(),
        Expression::NumericLiteral(literal) if literal.value == 0.0
    )
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants = ExpressionVariants::of(&[
    ExpressionKind::UnaryExpression,
    ExpressionKind::ParenthesizedExpression,
]);

impl<'a> Pass<'a> for VoidToUndefined {
    fn name(&self) -> &'static str {
        "void-to-undefined"
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if !is_void_zero(expr) {
            return;
        }
        if !cx.is_global_name("undefined") {
            return;
        }
        let span = expr.span();
        *expr =
            cx.tcx
                .create_unbound_ident_expr(span, Ident::from("undefined"), ReferenceFlags::Read);
        cx.changed();
    }
}
