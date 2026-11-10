use oxc_ast::ast::*;
use oxc_ast_visit::{walk_mut, VisitMut};
use oxc_span::{GetSpanMut, Span};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::source_map::origins::{is_recording, note_respanned};

pub const SYNTHETIC_BASE: u32 = 1 << 31;
const SYNTHETIC_MASK: u32 = SYNTHETIC_BASE - 1;

pub struct SpanCounter {
    next: AtomicU64,
}

impl Default for SpanCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl SpanCounter {
    pub const fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
        }
    }

    pub fn starting_at(value: u64) -> Self {
        Self {
            next: AtomicU64::new(value),
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.next.load(Ordering::Relaxed) > SYNTHETIC_MASK as u64
    }

    pub fn reset(&self) {
        self.next.store(0, Ordering::Relaxed);
    }

    fn tick(&self) -> Option<u32> {
        let value = self.next.fetch_add(1, Ordering::Relaxed);
        (value <= SYNTHETIC_MASK as u64).then_some(SYNTHETIC_BASE | (value as u32))
    }
}

thread_local! {
    static COUNTER: SpanCounter = const { SpanCounter::new() };
}

pub fn reset_synthetic_spans() {
    COUNTER.with(SpanCounter::reset);
}

pub fn synthetic_spans_exhausted() -> bool {
    COUNTER.with(SpanCounter::is_exhausted)
}

pub fn fresh_span_with(counter: &SpanCounter) -> Option<Span> {
    let start = counter.tick()?;
    let end = counter.tick()?;
    (start < end).then(|| Span::new(start, end))
}

pub fn fresh_span() -> Option<Span> {
    COUNTER.with(fresh_span_with)
}

pub fn fresh_spans(count: usize) -> Option<Vec<Span>> {
    (0..count).map(|_| fresh_span()).collect()
}

struct Fresh<'c> {
    counter: &'c SpanCounter,
    exhausted: bool,
    pairs: Option<Vec<(Span, Span)>>,
}

impl<'c> Fresh<'c> {
    fn new(counter: &'c SpanCounter) -> Self {
        Self {
            counter,
            exhausted: false,
            pairs: is_recording().then(Vec::new),
        }
    }

    fn finish(self) -> bool {
        if let Some(pairs) = &self.pairs {
            note_respanned(pairs);
        }
        !self.exhausted
    }
}

impl Fresh<'_> {
    fn stamp(&mut self, span: &mut Span, start: Option<u32>) {
        let (Some(start), Some(end)) = (start, self.counter.tick()) else {
            self.exhausted = true;
            return;
        };
        if start >= end {
            self.exhausted = true;
            return;
        }
        let fresh = Span::new(start, end);
        if let Some(pairs) = self.pairs.as_mut() {
            pairs.push((fresh, *span));
        }
        *span = fresh;
    }
}

impl<'a> VisitMut<'a> for Fresh<'_> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        let start = self.counter.tick();
        walk_mut::walk_expression(self, it);
        self.stamp(it.span_mut(), start);
    }

    fn visit_statement(&mut self, it: &mut Statement<'a>) {
        let start = self.counter.tick();
        walk_mut::walk_statement(self, it);
        self.stamp(it.span_mut(), start);
    }
}

pub fn fresh_spans_expression_with(counter: &SpanCounter, expr: &mut Expression<'_>) -> bool {
    let mut fresh = Fresh::new(counter);
    fresh.visit_expression(expr);
    fresh.finish()
}

pub fn fresh_spans_statement_with(counter: &SpanCounter, stmt: &mut Statement<'_>) -> bool {
    let mut fresh = Fresh::new(counter);
    fresh.visit_statement(stmt);
    fresh.finish()
}

pub fn fresh_spans_expression(expr: &mut Expression<'_>) -> bool {
    COUNTER.with(|counter| fresh_spans_expression_with(counter, expr))
}

pub fn fresh_spans_statement(stmt: &mut Statement<'_>) -> bool {
    COUNTER.with(|counter| fresh_spans_statement_with(counter, stmt))
}
