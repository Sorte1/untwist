use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::branch_reach::in_branches;
use crate::notes;
use crate::wrapper_calls::WrapperShape;

struct Writes<'s> {
    scoping: &'s Scoping,
    symbols: &'s FxHashSet<SymbolId>,
    found: Vec<(SymbolId, Span)>,
}

impl<'a> VisitJs<'a> for Writes<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(reference) = it.reference_id.get() else {
            return;
        };
        let reference = self.scoping.get_reference(reference);
        if let Some(symbol) = reference.symbol_id() {
            if reference.is_write() && self.symbols.contains(&symbol) {
                self.found.push((symbol, it.span));
            }
        }
    }
}

pub(crate) fn declared_by_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_flags(symbol).is_function()
        && scoping
            .scope_flags(scoping.symbol_scope_id(symbol))
            .is_var()
}

fn branched_writes_only(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
) -> FxHashSet<SymbolId> {
    branched_writes_outside(program, scoping, candidates, &FxHashMap::default())
        .into_keys()
        .collect()
}

pub(crate) fn branched_writes_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    homes: &FxHashMap<SymbolId, Span>,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let mut writes = Writes {
        scoping,
        symbols: candidates,
        found: Vec::new(),
    };
    writes.visit_program(program);
    let outside: Vec<(SymbolId, Span)> = writes
        .found
        .into_iter()
        .filter(|(symbol, span)| {
            homes
                .get(symbol)
                .is_none_or(|home| span.start < home.start || home.end < span.end)
        })
        .collect();
    let spans: FxHashSet<Span> = outside.iter().map(|(_, span)| *span).collect();
    let branched = in_branches(program, scoping, &spans);
    let mut held: FxHashMap<SymbolId, Vec<Span>> = FxHashMap::default();
    for (symbol, span) in &outside {
        held.entry(*symbol).or_default().push(*span);
    }
    for (symbol, span) in &outside {
        if !branched.contains(span) {
            held.remove(symbol);
        }
    }
    held
}

pub(crate) fn admit(
    program: &Program<'_>,
    scoping: &Scoping,
    unstable: Vec<WrapperShape>,
    (found, rejected): (&mut Vec<WrapperShape>, &mut Vec<(Span, &'static str)>),
) -> FxHashSet<SymbolId> {
    let candidates: FxHashSet<SymbolId> = unstable
        .iter()
        .map(|shape| shape.symbol)
        .filter(|symbol| declared_by_function(scoping, *symbol))
        .collect();
    let admitted = match candidates.is_empty() {
        true => FxHashSet::default(),
        false => branched_writes_only(program, scoping, &candidates),
    };
    for shape in unstable {
        match admitted.contains(&shape.symbol) {
            true => found.push(shape),
            false => rejected.push((shape.statement, notes::WRAPPER_BINDING_UNSTABLE)),
        }
    }
    admitted
}

pub(crate) fn defer(scoping: &Scoping, found: &[WrapperShape], admitted: &FxHashSet<SymbolId>) {
    let spans: Vec<Span> = found
        .iter()
        .filter(|shape| admitted.contains(&shape.symbol))
        .map(|shape| scoping.symbol_span(shape.symbol))
        .collect();
    crate::legacy_reach::defer_escapes(&spans);
}
