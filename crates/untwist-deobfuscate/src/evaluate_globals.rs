use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};
use untwist_eval::fold::{const_to_expression, fold_to_const, Const, FoldOptions};

use crate::notes;

pub const PASS: &str = "evaluate-globals";

pub const EVALUATED_GLOBALS: [&str; 4] = ["atob", "unescape", "decodeURI", "decodeURIComponent"];

#[derive(Default)]
pub struct EvaluateGlobals;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(EvaluateGlobals)
}

fn evaluated_global<'k, 'a>(expr: &'k Expression<'a>, cx: &Cx<'a, '_>) -> Option<&'k str> {
    let (callee, arguments) = call_parts(expr)?;
    let name = identifier_name(callee)?;
    if !EVALUATED_GLOBALS.contains(&name) || !cx.is_global_name(name) {
        return None;
    }
    if arguments.len() != 1 {
        return None;
    }
    if !matches!(
        arguments[0].as_expression(),
        Some(Expression::StringLiteral(_))
    ) {
        return None;
    }
    Some(name)
}

const ACTED_ON_EXPRESSIONS: ExpressionVariants =
    ExpressionVariants::of(&[ExpressionKind::CallExpression]);

impl<'a> Pass<'a> for EvaluateGlobals {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::EXIT_EXPRESSION.on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Some(name) = evaluated_global(expr, cx) else {
            return;
        };
        let span = expr.span();
        let Some(value @ Const::Str(_)) = fold_to_const(expr, &FoldOptions::default()) else {
            cx.note(notes::global_not_evaluated(name), Some(span));
            return;
        };
        let Some(replacement) = const_to_expression(&value, span, &*cx) else {
            return;
        };
        *expr = replacement;
        cx.changed();
    }
}
