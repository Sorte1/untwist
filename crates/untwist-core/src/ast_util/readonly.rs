use std::hash::{Hash, Hasher};

use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::prop::member_parts;
use crate::semantic::is_constant_binding;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PropRead {
    pub member_span: Span,
    pub object_span: Span,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObjectUses {
    pub reads: Vec<PropRead>,
    pub write_spans: Vec<Span>,
    pub opaque_spans: Vec<Span>,
    pub writes: u32,
    pub opaque: u32,
    pub keyed: u32,
}

impl Hash for ObjectUses {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.reads.hash(state);
        (self.writes as usize).hash(state);
        (self.opaque as usize).hash(state);
        self.write_spans.hash(state);
        self.opaque_spans.hash(state);
    }
}

pub fn is_readonly_object(uses: &ObjectUses) -> bool {
    uses.writes == 0 && uses.opaque == 0 && !uses.reads.is_empty()
}

pub fn is_readonly_but_keyed(uses: &ObjectUses) -> bool {
    uses.writes == 0 && uses.opaque == uses.keyed && (uses.keyed > 0 || !uses.reads.is_empty())
}

pub fn is_readonly_object_outside(uses: &ObjectUses, dead: &[Span]) -> bool {
    if uses.reads.is_empty() {
        return false;
    }
    let inside = |span: &Span| {
        dead.iter()
            .any(|range| range.start <= span.start && span.end <= range.end)
    };
    uses.write_spans.iter().all(inside) && uses.opaque_spans.iter().all(inside)
}

pub fn constant_bindings(scoping: &Scoping, symbols: &FxHashSet<SymbolId>) -> FxHashSet<SymbolId> {
    symbols
        .iter()
        .copied()
        .filter(|symbol| is_constant_binding(scoping, *symbol))
        .collect()
}

pub fn readonly_uses(uses: SymbolUses) -> FxHashMap<SymbolId, ObjectUses> {
    uses_where(uses, is_readonly_but_keyed)
}

fn uses_where(uses: SymbolUses, keep: fn(&ObjectUses) -> bool) -> FxHashMap<SymbolId, ObjectUses> {
    uses.uses
        .into_iter()
        .filter(|(_, uses)| keep(uses))
        .collect()
}

pub fn readonly_bindings_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &FxHashSet<SymbolId>,
) -> FxHashMap<SymbolId, ObjectUses> {
    let constant = constant_bindings(scoping, symbols);
    if constant.is_empty() {
        return FxHashMap::default();
    }
    uses_where(
        scan_symbol_uses(program, scoping, &constant),
        is_readonly_object,
    )
}

pub fn readonly_binding_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    symbol: SymbolId,
) -> Option<ObjectUses> {
    let mut symbols = FxHashSet::default();
    symbols.insert(symbol);
    readonly_bindings_uses(program, scoping, &symbols).remove(&symbol)
}

fn assignment_target_expression<'k, 'a>(
    target: &'k AssignmentTarget<'a>,
) -> Option<&'k Expression<'a>> {
    Some(target.as_member_expression()?.as_expression())
}

fn simple_assignment_target_expression<'k, 'a>(
    target: &'k SimpleAssignmentTarget<'a>,
) -> Option<&'k Expression<'a>> {
    Some(target.as_member_expression()?.as_expression())
}

#[inline(always)]
fn plain_member_object<'k, 'a>(expr: &'k Expression<'a>) -> Option<&'k Expression<'a>> {
    match expr {
        Expression::StaticMemberExpression(member) if !member.optional => Some(&member.object),
        Expression::ComputedMemberExpression(member) if !member.optional => Some(&member.object),
        _ => None,
    }
}

pub struct UsesScan<'s> {
    scoping: &'s Scoping,
    symbols: &'s FxHashSet<SymbolId>,
    seen: FxHashMap<SymbolId, FxHashSet<Span>>,
    uses: FxHashMap<SymbolId, ObjectUses>,
    bare: FxHashMap<SymbolId, Vec<Span>>,
    all: FxHashMap<SymbolId, Vec<Span>>,
    missed: usize,
}

fn address_of(id: &IdentifierReference<'_>) -> usize {
    std::ptr::from_ref(id) as usize
}

impl<'s> UsesScan<'s> {
    pub fn new(scoping: &'s Scoping, symbols: &'s FxHashSet<SymbolId>) -> Self {
        Self {
            scoping,
            symbols,
            seen: FxHashMap::default(),
            uses: FxHashMap::default(),
            bare: FxHashMap::default(),
            all: FxHashMap::default(),
            missed: 0,
        }
    }

    #[inline(always)]
    fn resolve(&self, id: &IdentifierReference<'_>) -> Option<SymbolId> {
        let symbol = self
            .scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()?;
        self.symbols.contains(&symbol).then_some(symbol)
    }

    fn target_of(&mut self, expr: &Expression<'_>) -> Option<(SymbolId, Span)> {
        let Expression::Identifier(id) = expr else {
            return None;
        };
        let Some(symbol) = self.resolve(id) else {
            self.missed = address_of(id);
            return None;
        };
        Some((symbol, id.span))
    }

    #[inline(always)]
    fn bare_reference(&mut self, id: &IdentifierReference<'_>) {
        if address_of(id) == self.missed {
            return;
        }
        if let Some(symbol) = self.resolve(id) {
            self.push_bare(symbol, id.span);
        }
    }

    #[cold]
    fn push_bare(&mut self, symbol: SymbolId, span: Span) {
        self.bare.entry(symbol).or_default().push(span);
        self.all.entry(symbol).or_default().push(span);
    }

    #[inline(always)]
    fn member_of_target(&mut self, expr: &Expression<'_>) -> Option<(SymbolId, Span, String)> {
        let (symbol, span) = self.target_of(plain_member_object(expr)?)?;
        let (_, name) = member_parts(expr)?;
        self.seen.entry(symbol).or_default().insert(span);
        Some((symbol, span, name.into_owned()))
    }

    fn record_write(&mut self, expr: &Expression<'_>) -> bool {
        let Some((symbol, _, _)) = self.member_of_target(expr) else {
            return false;
        };
        let entry = self.uses.entry(symbol).or_default();
        entry.writes = entry.writes.saturating_add(1);
        entry.write_spans.push(expr.span());
        true
    }

    #[inline(always)]
    pub fn descends_into(&mut self, it: &Expression<'_>) -> bool {
        match it {
            Expression::AssignmentExpression(assignment) => {
                if let Some(target) = assignment_target_expression(&assignment.left) {
                    self.record_write(target);
                }
            }
            Expression::UpdateExpression(update) => {
                if let Some(target) = simple_assignment_target_expression(&update.argument) {
                    self.record_write(target);
                }
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                if self.record_write(&unary.argument) {
                    return false;
                }
            }
            Expression::Identifier(id) => {
                self.bare_reference(id);
                return false;
            }
            other => self.record_read(other),
        }
        true
    }

    #[inline(always)]
    fn record_read(&mut self, other: &Expression<'_>) {
        let Some(object) = plain_member_object(other) else {
            return;
        };
        let Some((symbol, object_span)) = self.target_of(object) else {
            return;
        };
        match member_parts(other) {
            Some((_, name)) => self.push_read(symbol, other.span(), object_span, name.into_owned()),
            None => {
                let entry = self.uses.entry(symbol).or_default();
                entry.keyed = entry.keyed.saturating_add(1);
            }
        }
    }

    fn push_read(&mut self, symbol: SymbolId, member_span: Span, object_span: Span, name: String) {
        self.seen.entry(symbol).or_default().insert(object_span);
        self.uses.entry(symbol).or_default().reads.push(PropRead {
            member_span,
            object_span,
            name,
        });
    }

    #[inline(always)]
    pub fn reference(&mut self, it: &IdentifierReference<'_>) {
        if let Some(symbol) = self.resolve(it) {
            self.all.entry(symbol).or_default().push(it.span);
        }
    }

    pub fn finish(self) -> SymbolUses {
        let Self {
            symbols,
            seen,
            mut uses,
            bare,
            all,
            ..
        } = self;
        for symbol in symbols {
            let entry = uses.entry(*symbol).or_default();
            let accounted = seen.get(symbol);
            entry.opaque_spans = all
                .get(symbol)
                .map(|spans| {
                    spans
                        .iter()
                        .filter(|span| !accounted.is_some_and(|seen| seen.contains(span)))
                        .copied()
                        .collect()
                })
                .unwrap_or_default();
            entry.opaque = u32::try_from(entry.opaque_spans.len()).unwrap_or(u32::MAX);
        }
        SymbolUses {
            uses,
            references: bare,
        }
    }
}

impl<'a> VisitJs<'a> for UsesScan<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.descends_into(it) {
            walk_js::walk_expression(self, it);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.reference(it);
    }
}

#[derive(Default)]
pub struct SymbolUses {
    pub uses: FxHashMap<SymbolId, ObjectUses>,
    pub references: FxHashMap<SymbolId, Vec<Span>>,
}

impl SymbolUses {
    pub fn of(&self, symbol: SymbolId) -> Option<&ObjectUses> {
        self.uses.get(&symbol)
    }

    pub fn references_of(&self, symbol: SymbolId) -> &[Span] {
        self.references.get(&symbol).map_or(&[], Vec::as_slice)
    }
}

pub fn scan_symbol_uses(
    program: &Program<'_>,
    scoping: &Scoping,
    symbols: &FxHashSet<SymbolId>,
) -> SymbolUses {
    let mut scan = UsesScan::new(scoping, symbols);
    scan.visit_program(program);
    scan.finish()
}

pub fn scan_object_uses(program: &Program<'_>, scoping: &Scoping, symbol: SymbolId) -> ObjectUses {
    let mut symbols = FxHashSet::default();
    symbols.insert(symbol);
    scan_symbol_uses(program, scoping, &symbols)
        .uses
        .remove(&symbol)
        .unwrap_or_default()
}
