use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, walk_mut, VisitJs, VisitMut};
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::ast_util::respan::{fresh_spans_expression, SYNTHETIC_BASE};
use untwist_core::ast_util::search;

pub(super) fn sorted_spans(spans: impl Iterator<Item = Span>) -> Vec<Span> {
    let mut spans: Vec<Span> = spans.collect();
    spans.sort_unstable_by_key(|span| (span.start, span.end));
    spans.dedup();
    spans
}

pub(super) fn holds_target(targets: &[Span], span: Span) -> bool {
    if span.start >= SYNTHETIC_BASE {
        return true;
    }
    let first = targets.partition_point(|target| target.start < span.start);
    targets
        .get(first)
        .is_some_and(|target| target.end <= span.end)
}

pub(super) fn is_target(targets: &[Span], span: Span) -> bool {
    targets
        .binary_search_by_key(&(span.start, span.end), |target| (target.start, target.end))
        .is_ok()
}

struct Find<'p, 't> {
    targets: &'t [Span],
    found: FxHashMap<Span, &'p Expression<'p>>,
}

impl<'p> VisitJs<'p> for Find<'p, '_> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        if self.found.len() < self.targets.len() && holds_target(self.targets, it.span()) {
            walk_js::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'p>) {
        let span = it.span();
        if self.found.len() == self.targets.len() || !holds_target(self.targets, span) {
            return;
        }
        if is_target(self.targets, span) {
            let node: &'p Expression<'p> = self.alloc(it);
            self.found.entry(span).or_insert(node);
        }
        walk_js::walk_expression(self, it);
    }
}

pub(super) fn expressions_at<'p>(
    program: &'p Program<'p>,
    spans: impl Iterator<Item = Span>,
) -> FxHashMap<Span, &'p Expression<'p>> {
    let targets = sorted_spans(spans);
    if targets.is_empty() {
        return FxHashMap::default();
    }
    let mut find = Find {
        targets: &targets,
        found: FxHashMap::default(),
    };
    find.visit_program(program);
    let mut found = find.found;
    if found.len() < targets.len() {
        let missing: Vec<Span> = targets
            .iter()
            .copied()
            .filter(|span| !found.contains_key(span))
            .collect();
        let searched = search::expressions_at(program, &missing);
        for (span, node) in missing.into_iter().zip(searched) {
            if let Some(node) = node {
                found.insert(span, node);
            }
        }
    }
    found
}

#[derive(Default)]
pub(super) struct Edits<'a> {
    edits: FxHashMap<Span, Expression<'a>>,
}

impl<'a> Edits<'a> {
    pub(super) fn len(&self) -> usize {
        self.edits.len()
    }

    pub(super) fn insert(&mut self, span: Span, mut expr: Expression<'a>) -> bool {
        if span.start >= span.end || self.edits.contains_key(&span) {
            return false;
        }
        if !fresh_spans_expression(&mut expr) {
            return false;
        }
        self.edits.insert(span, expr);
        true
    }

    pub(super) fn apply(self, program: &mut Program<'a>, alloc: &'a Allocator) -> usize {
        if self.edits.is_empty() {
            return 0;
        }
        let targets = sorted_spans(self.edits.keys().copied());
        let mut applier = Applier {
            targets: &targets,
            edits: self.edits,
            applied: FxHashSet::default(),
            alloc,
            pruned: true,
        };
        applier.visit_program(program);
        if applier.applied.len() < applier.edits.len() {
            let applied = std::mem::take(&mut applier.applied);
            applier.edits.retain(|span, _| !applied.contains(span));
            applier.pruned = false;
            applier.visit_program(program);
            applier.applied.extend(applied);
        }
        applier.applied.len()
    }
}

struct Applier<'a, 't> {
    targets: &'t [Span],
    edits: FxHashMap<Span, Expression<'a>>,
    applied: FxHashSet<Span>,
    alloc: &'a Allocator,
    pruned: bool,
}

impl Applier<'_, '_> {
    fn may_hold(&self, span: Span) -> bool {
        !self.pruned || holds_target(self.targets, span)
    }
}

impl<'a> VisitMut<'a> for Applier<'a, '_> {
    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        if self.may_hold(it.span()) {
            walk_mut::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let span = it.span();
        if !self.may_hold(span) {
            return;
        }
        if let Some(replacement) = self.edits.get(&span) {
            *it = replacement.clone_in(self.alloc);
            self.applied.insert(span);
        }
        walk_mut::walk_expression(self, it);
    }
}
