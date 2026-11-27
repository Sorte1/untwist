use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use rustc_hash::FxHashMap;

#[derive(Default)]
pub(super) struct Expected {
    by_reference: FxHashMap<Span, Option<Span>>,
}

impl Expected {
    pub(super) fn insert(&mut self, reference: Span, declaration: Option<Span>) {
        self.by_reference.insert(reference, declaration);
    }

    pub(super) fn is_empty(&self) -> bool {
        self.by_reference.is_empty()
    }
}

pub(super) type Spans = FxHashMap<Span, oxc_syntax::symbol::SymbolId>;

struct Record<'s, 'e> {
    scoping: &'s Scoping,
    expected: &'e mut Expected,
    skip: &'s [oxc_syntax::symbol::SymbolId],
    spans: Option<&'s Spans>,
}

fn declaration(scoping: &Scoping, it: &IdentifierReference<'_>) -> Option<Span> {
    let reference = scoping.get_reference(it.reference_id.get()?);
    reference
        .symbol_id()
        .map(|symbol| scoping.symbol_span(symbol))
}

impl<'a> VisitJs<'a> for Record<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = match self.spans {
            Some(spans) => spans.get(&it.span).copied(),
            None => it
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id()),
        };
        if symbol.is_some_and(|s| self.skip.contains(&s)) || it.name == "arguments" {
            return;
        }
        let declared = match self.spans {
            Some(_) => symbol.map(|s| self.scoping.symbol_span(s)),
            None => declaration(self.scoping, it),
        };
        self.expected.insert(it.span, declared);
    }
}

pub(super) fn record_function(
    function: &Function<'_>,
    scoping: &Scoping,
    expected: &mut Expected,
    skip: Option<oxc_syntax::symbol::SymbolId>,
) {
    let skip: Vec<oxc_syntax::symbol::SymbolId> = skip.into_iter().collect();
    let mut record = Record {
        scoping,
        expected,
        skip: &skip,
        spans: None,
    };
    walk_js::walk_function(
        &mut record,
        function,
        oxc_syntax::scope::ScopeFlags::Function,
    );
}

pub(super) fn record_statement(
    statement: &Statement<'_>,
    (scoping, spans): (&Scoping, &Spans),
    expected: &mut Expected,
    skip: &[oxc_syntax::symbol::SymbolId],
) {
    Record {
        scoping,
        expected,
        skip,
        spans: Some(spans),
    }
    .visit_statement(statement);
}

pub(super) fn record_expression(
    expression: &Expression<'_>,
    (scoping, spans): (&Scoping, &Spans),
    expected: &mut Expected,
    skip: &[oxc_syntax::symbol::SymbolId],
) {
    Record {
        scoping,
        expected,
        skip,
        spans: Some(spans),
    }
    .visit_expression(expression);
}

struct Verify<'s, 'e> {
    scoping: &'s Scoping,
    expected: &'e Expected,
    seen: usize,
    wrong: bool,
}

impl<'a> VisitJs<'a> for Verify<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(want) = self.expected.by_reference.get(&it.span) {
            self.seen += 1;
            self.wrong |= declaration(self.scoping, it) != *want;
        }
    }
}

pub(super) fn holds(program: &Program<'_>, scoping: &Scoping, expected: &Expected) -> bool {
    let mut verify = Verify {
        scoping,
        expected,
        seen: 0,
        wrong: false,
    };
    verify.visit_program(program);
    !verify.wrong && verify.seen == expected.by_reference.len()
}
