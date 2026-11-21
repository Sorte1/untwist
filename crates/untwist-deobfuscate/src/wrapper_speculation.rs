use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
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

struct ValueReads<'s> {
    scoping: &'s Scoping,
    symbols: &'s FxHashSet<SymbolId>,
    found: Vec<(SymbolId, Span)>,
    passed: FxHashSet<SymbolId>,
}

impl ValueReads<'_> {
    fn symbol(&self, it: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = self.scoping.get_reference(it.reference_id.get()?);
        if reference.is_write() || !reference.is_read() {
            return None;
        }
        reference
            .symbol_id()
            .filter(|symbol| self.symbols.contains(symbol))
    }
}

impl<'a> VisitJs<'a> for ValueReads<'_> {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        for argument in &it.arguments {
            if let Argument::Identifier(id) = argument {
                if let Some(symbol) = self.symbol(id) {
                    self.passed.insert(symbol);
                }
            }
        }
        match &it.callee {
            Expression::Identifier(_) => {
                for argument in &it.arguments {
                    self.visit_argument(argument);
                }
            }
            _ => walk_js::walk_call_expression(self, it),
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = self.symbol(it) {
            self.found.push((symbol, it.span));
        }
    }
}

fn branched_value_reads(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &FxHashSet<SymbolId>,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let mut reads = ValueReads {
        scoping,
        symbols,
        found: Vec::new(),
        passed: FxHashSet::default(),
    };
    reads.visit_program(program);
    let spans: FxHashSet<Span> = reads.found.iter().map(|(_, span)| *span).collect();
    let branched = in_branches(program, scoping, &spans);
    let mut held: FxHashMap<SymbolId, Vec<Span>> = FxHashMap::default();
    for (symbol, span) in &reads.found {
        held.entry(*symbol).or_default().push(*span);
    }
    for (symbol, span) in &reads.found {
        if !branched.contains(span) || reads.passed.contains(symbol) {
            held.remove(symbol);
        }
    }
    held
}

pub(crate) fn declared_by_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_flags(symbol).is_function()
        && scoping
            .scope_flags(scoping.symbol_scope_id(symbol))
            .is_var()
}

fn writes_outside_homes(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    homes: &FxHashMap<SymbolId, Span>,
) -> Vec<(SymbolId, Span)> {
    let mut writes = Writes {
        scoping,
        symbols: candidates,
        found: Vec::new(),
    };
    writes.visit_program(program);
    writes
        .found
        .into_iter()
        .filter(|(symbol, span)| {
            homes
                .get(symbol)
                .is_none_or(|home| span.start < home.start || home.end < span.end)
        })
        .collect()
}

pub(crate) fn guarded_writes_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    homes: &FxHashMap<SymbolId, Span>,
) -> FxHashSet<SymbolId> {
    let outside = writes_outside_homes(program, scoping, candidates, homes);
    let spans: FxHashSet<Span> = outside.iter().map(|(_, span)| *span).collect();
    let guards = crate::dead_guards::enclosing_guards(program, &spans);
    let mut held: FxHashSet<SymbolId> = outside.iter().map(|(symbol, _)| *symbol).collect();
    for (symbol, span) in &outside {
        if guards.get(span).is_none_or(Vec::is_empty) {
            held.remove(symbol);
        }
    }
    held
}

pub(crate) fn branched_writes_outside(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &FxHashSet<SymbolId>,
    homes: &FxHashMap<SymbolId, Span>,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let outside = writes_outside_homes(program, scoping, candidates, homes);
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
) -> FxHashMap<SymbolId, Vec<Span>> {
    let candidates: FxHashSet<SymbolId> = unstable
        .iter()
        .map(|shape| shape.symbol)
        .filter(|symbol| declared_by_function(scoping, *symbol))
        .collect();
    let admitted = match candidates.is_empty() {
        true => FxHashMap::default(),
        false => branched_writes_outside(program, scoping, &candidates, &FxHashMap::default()),
    };
    for shape in unstable {
        match admitted.contains_key(&shape.symbol) {
            true => found.push(shape),
            false => rejected.push((shape.statement, notes::WRAPPER_BINDING_UNSTABLE)),
        }
    }
    admitted
}

pub(crate) fn defer(
    (program, scoping): (&Program<'_>, &Scoping),
    found: &[WrapperShape],
    admitted: &FxHashMap<SymbolId, Vec<Span>>,
    speculating: bool,
) -> FxHashMap<SymbolId, Vec<Span>> {
    let symbols: FxHashSet<SymbolId> = found.iter().map(|shape| shape.symbol).collect();
    let mut held = match speculating && !symbols.is_empty() {
        true => branched_value_reads(program, scoping, &symbols),
        false => FxHashMap::default(),
    };
    for (symbol, writes) in admitted {
        if symbols.contains(symbol) {
            held.entry(*symbol)
                .or_default()
                .extend(writes.iter().copied());
        }
    }
    let mut spans: Vec<Span> = Vec::new();
    for shape in found {
        if admitted.contains_key(&shape.symbol) {
            spans.push(scoping.symbol_span(shape.symbol));
        }
        if let Some(references) = held.get(&shape.symbol) {
            spans.extend(references.iter().copied());
        }
    }
    crate::legacy_reach::defer_escapes(&spans);
    held
}
