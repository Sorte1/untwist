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

struct Record<'s, 'e> {
    scoping: &'s Scoping,
    expected: &'e mut Expected,
    skip: Option<oxc_syntax::symbol::SymbolId>,
}

fn declaration(scoping: &Scoping, it: &IdentifierReference<'_>) -> Option<Span> {
    let reference = scoping.get_reference(it.reference_id.get()?);
    reference
        .symbol_id()
        .map(|symbol| scoping.symbol_span(symbol))
}

impl<'a> VisitJs<'a> for Record<'_, '_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        if (symbol.is_some() && symbol == self.skip) || it.name == "arguments" {
            return;
        }
        self.expected.insert(it.span, declaration(self.scoping, it));
    }
}

pub(super) fn record_function(
    function: &Function<'_>,
    scoping: &Scoping,
    expected: &mut Expected,
    skip: Option<oxc_syntax::symbol::SymbolId>,
) {
    let mut record = Record {
        scoping,
        expected,
        skip,
    };
    walk_js::walk_function(
        &mut record,
        function,
        oxc_syntax::scope::ScopeFlags::Function,
    );
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
