use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;

const FILTER_WORDS: usize = 16;
const FILTER_SHIFT: u32 = 54;
const SPREAD: u64 = 0x9e37_79b9_7f4a_7c15;

fn filter_bit(span: Span) -> (usize, u64) {
    let key = (u64::from(span.start) << 32) | u64::from(span.end);
    let hashed = key.wrapping_mul(SPREAD) >> FILTER_SHIFT;
    ((hashed >> 6) as usize, 1u64 << (hashed & 63))
}

const NO_SLOT: usize = usize::MAX;

struct Many<'p, T: 'p> {
    filter: [u64; FILTER_WORDS],
    heads: FxHashMap<Span, usize>,
    next: Vec<usize>,
    found: Vec<Option<&'p T>>,
}

impl<'p, T> Many<'p, T> {
    fn new(spans: &[Span]) -> Self {
        let mut filter = [0u64; FILTER_WORDS];
        let mut heads: FxHashMap<Span, usize> = FxHashMap::default();
        let mut next = vec![NO_SLOT; spans.len()];
        for (slot, span) in spans.iter().enumerate().rev() {
            let (word, bit) = filter_bit(*span);
            if let Some(entry) = filter.get_mut(word) {
                *entry |= bit;
            }
            if let (Some(previous), Some(link)) = (heads.insert(*span, slot), next.get_mut(slot)) {
                *link = previous;
            }
        }
        Self {
            filter,
            heads,
            next,
            found: vec![None; spans.len()],
        }
    }

    fn may_want(&self, span: Span) -> bool {
        let (word, bit) = filter_bit(span);
        self.filter.get(word).is_some_and(|entry| entry & bit != 0)
    }

    fn offer(&mut self, span: Span, node: &'p T) {
        if !self.may_want(span) {
            return;
        }
        let mut slot = self.heads.remove(&span).unwrap_or(NO_SLOT);
        while let Some(found) = self.found.get_mut(slot) {
            *found = Some(node);
            slot = self.next.get(slot).copied().unwrap_or(NO_SLOT);
        }
    }
}

impl<'p, T> Many<'p, T> {
    fn is_done(&self) -> bool {
        self.heads.is_empty()
    }
}

struct ExpressionsSearch<'p> {
    wanted: Many<'p, Expression<'p>>,
}

impl<'p> VisitJs<'p> for ExpressionsSearch<'p> {
    fn visit_expression(&mut self, it: &Expression<'p>) {
        if self.wanted.is_done() {
            return;
        }
        let node: &'p Expression<'p> = self.alloc(it);
        self.wanted.offer(it.span(), node);
        walk_js::walk_expression(self, it);
    }
}

struct StatementsSearch<'p> {
    wanted: Many<'p, Statement<'p>>,
}

impl<'p> VisitJs<'p> for StatementsSearch<'p> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        if self.wanted.is_done() {
            return;
        }
        let node: &'p Statement<'p> = self.alloc(it);
        self.wanted.offer(it.span(), node);
        walk_js::walk_statement(self, it);
    }
}

pub fn expressions_at<'p>(
    program: &'p Program<'p>,
    spans: &[Span],
) -> Vec<Option<&'p Expression<'p>>> {
    if spans.is_empty() {
        return Vec::new();
    }
    let mut search = ExpressionsSearch {
        wanted: Many::new(spans),
    };
    search.visit_program(program);
    search.wanted.found
}

pub fn statements_at<'p>(
    program: &'p Program<'p>,
    spans: &[Span],
) -> Vec<Option<&'p Statement<'p>>> {
    match spans {
        [] => return Vec::new(),
        [span] => return vec![statement_at(program, *span)],
        _ => {}
    }
    let mut search = StatementsSearch {
        wanted: Many::new(spans),
    };
    search.visit_program(program);
    search.wanted.found
}

struct ExpressionSearch<'p> {
    span: Span,
    found: Option<&'p Expression<'p>>,
}

impl<'p> VisitJs<'p> for ExpressionSearch<'p> {
    fn visit_expression(&mut self, it: &Expression<'p>) {
        if self.found.is_some() {
            return;
        }
        if it.span() == self.span {
            self.found = Some(self.alloc(it));
            return;
        }
        walk_js::walk_expression(self, it);
    }
}

struct StatementSearch<'p> {
    span: Span,
    found: Option<&'p Statement<'p>>,
}

impl<'p> VisitJs<'p> for StatementSearch<'p> {
    fn visit_statement(&mut self, it: &Statement<'p>) {
        if self.found.is_some() {
            return;
        }
        if it.span() == self.span {
            self.found = Some(self.alloc(it));
            return;
        }
        walk_js::walk_statement(self, it);
    }
}

pub fn expression_at<'p>(program: &'p Program<'p>, span: Span) -> Option<&'p Expression<'p>> {
    let mut search = ExpressionSearch { span, found: None };
    search.visit_program(program);
    search.found
}

pub fn statement_at<'p>(program: &'p Program<'p>, span: Span) -> Option<&'p Statement<'p>> {
    let mut search = StatementSearch { span, found: None };
    search.visit_program(program);
    search.found
}
