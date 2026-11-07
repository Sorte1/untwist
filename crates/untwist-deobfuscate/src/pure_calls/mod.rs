mod eval;
mod plan;
mod scan;
mod stateful;
mod uses;

use oxc_allocator::GetAllocator;
use oxc_ast::ast::Program;
use untwist_core::ast_util::edit::{ExprEdits, StmtEdits};
use untwist_core::pass::{Cx, Interest, Pass};
use untwist_core::semantic::{assumed_hazard, scope_hazard};
use untwist_eval::fold::const_to_expression;

use plan::Plan;

pub const PASS: &str = "fold-pure-calls";

#[derive(Default)]
pub struct FoldPureCalls;

pub fn boxed<'a>() -> Box<dyn Pass<'a> + 'a> {
    Box::new(FoldPureCalls)
}

fn apply_stateful<'a>(plan: stateful::Plan, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
    let mut folds = ExprEdits::new();
    for (span, value) in &plan.folds {
        let Some(replacement) = const_to_expression(value, *span, &*cx) else {
            return;
        };
        if !folds.insert(*span, replacement) {
            return;
        }
    }
    let mut removals = StmtEdits::new();
    for span in &plan.removals {
        if !removals.remove(*span) {
            return;
        }
    }
    let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != plan.folds.len() {
        return;
    }
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(removed);
}

fn apply<'a>(plan: Plan, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
    let mut folds = ExprEdits::new();
    for (span, value) in &plan.folds {
        let Some(replacement) = const_to_expression(value, *span, &*cx) else {
            continue;
        };
        folds.insert(*span, replacement);
    }
    let wanted = folds.len();
    let applied = folds.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(applied);
    if applied != plan.folds.len() || wanted != applied || plan.removals.is_empty() {
        return;
    }
    let mut removals = StmtEdits::new();
    for span in &plan.removals {
        removals.remove(*span);
    }
    let removed = removals.apply(program, cx.allocator(), cx.scoping_mut());
    cx.changed_by(removed);
}

impl<'a> Pass<'a> for FoldPureCalls {
    fn name(&self) -> &'static str {
        PASS
    }

    fn interest(&self) -> Interest {
        Interest::ENTER_PROGRAM
    }

    fn requires_extended(&self) -> bool {
        true
    }

    fn enter_program(&mut self, program: &mut Program<'a>, cx: &mut Cx<'a, '_>) {
        if !cx.is_program_unchanged_this_traversal() {
            return;
        }
        let scoping = cx.scoping();
        if scope_hazard(scoping)
            .or_else(|| assumed_hazard(scoping))
            .is_some()
        {
            return;
        }
        if let Some(plan) = plan::plan(program, scoping, cx.opts) {
            apply(plan, program, cx);
            return;
        }
        if let Some(plan) = stateful::plan(program, scoping, cx.opts) {
            apply_stateful(plan, program, cx);
        }
    }
}
