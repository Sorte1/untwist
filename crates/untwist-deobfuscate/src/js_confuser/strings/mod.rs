use oxc_allocator::GetAllocator;
use oxc_ast::ast::*;
use oxc_span::Span;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::builtins::Reliance;
use untwist_eval::fold::{const_to_expression, Const};

use super::refs::{judge_replaced, names_observed};
use analyse::{analyse, Plan};

mod analyse;
mod moved;
mod pairs;
mod sandbox;

pub const PASS: &str = "decode-js-confuser-strings";

static RELIED_ON: Reliance = Reliance {
    roots: &[
        "globalThis",
        "Object",
        "String",
        "Array",
        "TextDecoder",
        "Uint8Array",
    ],
    members: &[],
};

static EVALUATED_RELIED_ON: Reliance = Reliance {
    roots: &[
        "globalThis",
        "Object",
        "String",
        "Array",
        "TextDecoder",
        "Uint8Array",
        "Number",
        "Boolean",
        "Function",
        "Symbol",
        "Math",
        "JSON",
    ],
    members: &[],
};

#[derive(Default)]
pub struct DecodeStrings {
    settled: bool,
}

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(DecodeStrings::default())
}

fn drop_declarators(program: &mut Program<'_>, dead: &[oxc_syntax::symbol::SymbolId]) -> usize {
    let mut dropped = 0;
    for statement in program.body.iter_mut() {
        let Statement::VariableDeclaration(declaration) = statement else {
            continue;
        };
        let before = declaration.declarations.len();
        declaration.declarations.retain(|d| {
            d.init.is_some()
                || !matches!(&d.id, BindingPattern::BindingIdentifier(id) if id.symbol_id.get().is_some_and(|s| dead.contains(&s)))
        });
        dropped += before - declaration.declarations.len();
    }
    program
        .body
        .retain(|s| !matches!(s, Statement::VariableDeclaration(d) if d.declarations.is_empty()));
    dropped
}

fn apply<'a>(
    plan: &Plan,
    (removals_list, drop_vars): (&[Span], bool),
    program: &mut Program<'a>,
    cx: &mut Cx<'a, '_>,
) -> bool {
    let mut folds = ExprEdits::new();
    for (span, text) in &plan.folds {
        let Some(replacement) = const_to_expression(&Const::Str(text.clone()), *span, &*cx) else {
            return false;
        };
        if !folds.insert(*span, replacement) {
            return false;
        }
    }
    let wanted = folds.len();
    let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != wanted {
        cx.note(format!("js-confuser string machinery kept: {applied} of {wanted} concealed strings could be put in place"), None);
        return false;
    }
    let mut removals = StmtEdits::new();
    for span in removals_list {
        removals.remove(*span);
    }
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(removed);
    if drop_vars && !plan.dead_vars.is_empty() {
        let dropped = drop_declarators(program, &plan.dead_vars);
        if dropped > 0 {
            *cx.scoping_mut() = untwist_core::semantic::build_scoping(program).scoping;
            cx.binding_moved();
            cx.changed_by(dropped);
        }
    }
    true
}

impl<'a> Pass<'a> for DecodeStrings {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if self.settled {
            return;
        }
        let plan = match analyse(program, cx.scoping(), cx.opts.nested_root) {
            Ok(plan)
                if plan.folds.is_empty()
                    && plan.block_removals.is_empty()
                    && plan.top_removals.is_empty() =>
            {
                self.settled = true;
                return;
            }
            Ok(plan) => plan,
            Err(reason) => {
                self.settled = true;
                cx.note(
                    format!("js-confuser string concealing kept: {reason}"),
                    None,
                );
                return;
            }
        };
        if plan.folds.is_empty() {
            self.settled = true;
        }
        let replacements = plan
            .folds
            .iter()
            .filter_map(|(span, text)| {
                Some((
                    *span,
                    const_to_expression(&Const::Str(text.clone()), *span, &*cx)?,
                ))
            })
            .chain(plan.array_reads.iter().filter_map(|(span, value)| {
                let replacement = match value {
                    Const::Undefined => Some(Expression::new_void_0(*span, &*cx)),
                    other => const_to_expression(other, *span, &*cx),
                };
                Some((*span, replacement?))
            }))
            .collect();
        let Some(judged) = judge_replaced(
            program,
            cx,
            replacements,
            (
                match plan.evaluated {
                    true => &EVALUATED_RELIED_ON,
                    false => &RELIED_ON,
                },
                &plan.checked,
            ),
            &plan.symbols,
        ) else {
            return;
        };
        if !cx.opts.unsafe_transforms && !judged.intact {
            cx.note("js-confuser string concealing kept: the program may replace a standard built-in the decoder uses (String, Array, Object, globalThis, TextDecoder, Uint8Array)", None);
            return;
        }
        if judged.reach.reach.is_some() || judged.reach.reach_by_code.is_some() {
            cx.note("js-confuser string concealing kept: code may reach the machinery's names through the global object or code text", None);
            return;
        }
        let mut removals = plan.block_removals.clone();
        let observed = names_observed(&judged.reach);
        if observed {
            cx.note(
                "js-confuser string machinery: code may observe the global object's names, so its top-level string machinery is kept",
                judged.reach.observed,
            );
        } else {
            removals.extend(plan.top_removals.iter().copied());
            if judged.reach.outside_premise.is_some() && !plan.top_removals.is_empty() {
                cx.note("js-confuser string machinery: the machinery was dropped under the outside-code premise", None);
            }
        }
        if !plan.folds.is_empty() {
            cx.note(
                format!(
                    "{} js-confuser concealed string(s) decoded",
                    plan.folds.len()
                ),
                None,
            );
        }
        if let Some(kept) = plan.kept {
            cx.note(format!("js-confuser string machinery kept: {kept}"), None);
        }
        if !apply(
            &plan,
            (&removals, !observed && plan.kept.is_none()),
            program,
            cx,
        ) {
            self.settled = true;
        }
    }
}
