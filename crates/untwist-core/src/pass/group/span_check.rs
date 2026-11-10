use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use oxc_ast::ast::{Expression, Program, Statement};
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};

use super::super::interest::ExpressionKind;

static CHECK: AtomicBool = AtomicBool::new(false);

static VIOLATIONS: Mutex<Vec<(Span, String)>> = Mutex::new(Vec::new());

#[doc(hidden)]
pub fn check_shared_spans_for_tests(on: bool) {
    CHECK.store(on, Ordering::SeqCst);
}

#[doc(hidden)]
pub fn take_shared_span_violations_for_tests() -> Vec<String> {
    let mut violations = VIOLATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    std::mem::take(&mut *violations)
        .into_iter()
        .map(|(_, violation)| violation)
        .collect()
}

#[derive(Default)]
struct SharedSpans {
    enclosing: Vec<Option<(Span, ExpressionKind)>>,
    found: Vec<(Span, ExpressionKind, ExpressionKind)>,
}

impl<'a> VisitJs<'a> for SharedSpans {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        let span = it.span();
        let kind = ExpressionKind::of(it);
        if let Some(Some((outer, outer_kind))) = self.enclosing.last() {
            if *outer == span && !span.is_empty() && *outer_kind != kind {
                self.found.push((span, *outer_kind, kind));
            }
        }
        self.enclosing.push(Some((span, kind)));
        walk_js::walk_expression(self, it);
        self.enclosing.pop();
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        self.enclosing.push(None);
        walk_js::walk_statement(self, it);
        self.enclosing.pop();
    }
}

pub(super) fn check(program: &Program<'_>, after: impl FnOnce() -> String) {
    if !CHECK.load(Ordering::SeqCst) {
        return;
    }
    let mut scan = SharedSpans::default();
    scan.visit_program(program);
    if scan.found.is_empty() {
        return;
    }
    let after = after();
    let mut violations = VIOLATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    for (span, outer, inner) in scan.found {
        if violations.iter().any(|(seen, _)| *seen == span) {
            continue;
        }
        let violation = format!("after {after}: {outer:?} and its child {inner:?} share {span:?}");
        violations.push((span, violation));
    }
}
