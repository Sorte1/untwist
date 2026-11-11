use oxc_ast::ast::*;
use oxc_span::GetSpan;
use untwist_core::ast_util::global::program_rebinds_global;
use untwist_core::ast_util::matcher::call_parts;
use untwist_core::ast_util::prop::identifier_name;
use untwist_core::pass::{Cx, ExpressionKind, ExpressionVariants, Interest, Pass};
use untwist_core::semantic::builtins::{Reliance, Unpatched};
use untwist_eval::fold::{const_to_expression, fold_to_const, Const, FoldOptions};

use crate::notes;

pub const PASS: &str = "evaluate-globals";

pub const EVALUATED_GLOBALS: [&str; 4] = ["atob", "unescape", "decodeURI", "decodeURIComponent"];

static RELIED_ON: [Reliance; 4] = [
    Reliance {
        roots: &["atob"],
        members: &[],
    },
    Reliance {
        roots: &["unescape"],
        members: &[],
    },
    Reliance {
        roots: &["decodeURI"],
        members: &[],
    },
    Reliance {
        roots: &["decodeURIComponent"],
        members: &[],
    },
];

pub struct EvaluateGlobals {
    unpatched: [Unpatched; 4],
}

impl Default for EvaluateGlobals {
    fn default() -> Self {
        Self {
            unpatched: [
                Unpatched::new(&RELIED_ON[0]),
                Unpatched::new(&RELIED_ON[1]),
                Unpatched::new(&RELIED_ON[2]),
                Unpatched::new(&RELIED_ON[3]),
            ],
        }
    }
}

impl EvaluateGlobals {
    fn unpatched(&mut self, name: &str) -> Option<&mut Unpatched> {
        let index = EVALUATED_GLOBALS
            .iter()
            .position(|global| *global == name)?;
        self.unpatched.get_mut(index)
    }
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(EvaluateGlobals::default())
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
        (Interest::ENTER_PROGRAM | Interest::EXIT_EXPRESSION).on_expressions(ACTED_ON_EXPRESSIONS)
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        for unpatched in &mut self.unpatched {
            unpatched.learn(program, cx);
        }
    }

    fn exit_expression(&mut self, expr: &mut Expression<'a>, cx: &mut Cx<'a, '_>) {
        let Some(name) = evaluated_global(expr, cx) else {
            return;
        };
        let span = expr.span();
        if !cx.opts.unsafe_transforms && program_rebinds_global(cx.scoping(), name) {
            cx.note(notes::global_rebound(name), Some(span));
            return;
        }
        let Some(unpatched) = self.unpatched(name) else {
            return;
        };
        if !unpatched.holds(cx) {
            if unpatched.decided() {
                cx.note(notes::global_patched(name), Some(span));
            }
            return;
        }
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
