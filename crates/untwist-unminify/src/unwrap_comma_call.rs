use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use untwist_core::ast_util::global::pure_value_reads_globals;
use untwist_core::ast_util::literal::{is_pure_value, is_undefined_like};
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Locality, Pass};
use untwist_core::semantic::scope_hazard;

pub const PASS: &str = "unwrap-comma-call";

pub use crate::notes::UNWRAP_COMMA_CALL_SCOPE_HAZARD as SCOPE_HAZARD;

#[derive(Default)]
pub struct UnwrapCommaCall {
    hazardous: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(UnwrapCommaCall::default())
}

fn is_eval_identifier(expr: &Expression<'_>) -> bool {
    matches!(expr, Expression::Identifier(id) if id.name.as_str() == "eval")
}

fn unwrappable_callee(expr: &Expression<'_>) -> bool {
    let expr = expr.without_parentheses();
    if is_eval_identifier(expr) {
        return false;
    }
    matches!(
        expr,
        Expression::Identifier(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
    )
}

fn is_droppable(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    (is_pure_value(expr) || is_undefined_like(expr)) && pure_value_reads_globals(expr, scoping)
}

fn droppable_prefix(sequence: &SequenceExpression<'_>, scoping: &Scoping) -> bool {
    let Some((last, prefix)) = sequence.expressions.split_last() else {
        return false;
    };
    !prefix.is_empty()
        && prefix.iter().all(|expr| is_droppable(expr, scoping))
        && unwrappable_callee(last)
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::CallExpression]);

impl<'a> Pass<'a> for UnwrapCommaCall {
    fn name(&self) -> &'static str {
        PASS
    }

    fn locality(&self) -> Locality {
        Locality::Neutral
    }

    fn interest(&self) -> Interest {
        (Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION).on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.hazardous = false;
    }

    fn enter_program(&mut self, _program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        self.hazardous = scope_hazard(cx.scoping()).is_some();
        if self.hazardous {
            cx.note(SCOPE_HAZARD, None);
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        if self.hazardous {
            return;
        }
        let Expression::CallExpression(call) = expr else {
            return;
        };
        if call.optional {
            return;
        }
        let Expression::SequenceExpression(sequence) = &mut call.callee else {
            return;
        };
        if !droppable_prefix(sequence, cx.scoping()) {
            return;
        }
        let Some(callee) = sequence.expressions.pop() else {
            return;
        };
        call.callee = callee;
        cx.changed();
    }
}
