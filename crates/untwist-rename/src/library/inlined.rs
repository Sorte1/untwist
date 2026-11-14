use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use oxc_semantic::Scoping;

use super::shape::{expanded_shape, ExactShape, Shaped, Unit};
use super::table::{self, Table};
use super::{Candidate, Merged, Slot, UnitRecord};

fn holds_inlined(unit: Span, inlined: &[Span]) -> bool {
    inlined
        .iter()
        .any(|span| *span != unit && unit.start <= span.start && span.end <= unit.end)
}

pub(super) fn expanded_probes<'s>(
    table: &'s Table,
    units: &'s [Unit<'_, '_>],
    shaped: &'s Shaped<'_, '_>,
    scoping: &'s Scoping,
) -> impl Iterator<Item = (usize, ExactShape)> + 's {
    units
        .iter()
        .enumerate()
        .filter(|(_, unit)| holds_inlined(unit.span, &shaped.inlined))
        .map(|(unit_index, unit)| (unit_index, expanded_shape(unit, scoping)))
        .filter(|(_, shape)| table.has_loose(table::loose_key(shape.loose)))
}

pub(super) fn expanded_records<'s>(
    shaped: &'s Shaped<'_, '_>,
    scoping: &'s Scoping,
) -> impl Iterator<Item = UnitRecord> + 's {
    let name = |symbol: &SymbolId| scoping.symbol_name(*symbol).to_string();
    shaped
        .units
        .iter()
        .filter(|unit| holds_inlined(unit.span, &shaped.inlined))
        .map(move |unit| {
            let shape = expanded_shape(unit, scoping);
            UnitRecord {
                loose: shape.loose,
                exact: shape.exact,
                specific: shape.specific,
                kind: unit.kind,
                span: unit.span,
                binding: unit.binding.as_ref().map(name),
                inner: shape.inner.iter().map(name).collect(),
                outer: shape.outer.iter().map(name).collect(),
            }
        })
}

fn deduplicated(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let proven: FxHashSet<usize> = candidates
        .iter()
        .filter(|candidate| !candidate.shape.inlined.is_empty())
        .map(|candidate| candidate.unit_index)
        .collect();
    candidates
        .into_iter()
        .filter(|candidate| {
            !candidate.shape.inlined.is_empty() || !proven.contains(&candidate.unit_index)
        })
        .collect()
}

fn labels(merged: &Merged) -> impl Iterator<Item = &str> {
    merged.groups.iter().map(|group| group.label.as_str())
}

fn shares_label(outer: &Merged, inner: &Merged) -> bool {
    labels(outer).any(|label| labels(inner).any(|other| other == label))
}

fn proven(candidate: &Candidate, kept: &[(Span, &Candidate)]) -> bool {
    candidate.shape.inlined.iter().all(|(_, span)| {
        kept.iter()
            .any(|(unit, inner)| unit == span && shares_label(&candidate.merged, &inner.merged))
    })
}

fn forget(slots: &mut [Slot], symbols: &[SymbolId], symbol: SymbolId) {
    if let Some(index) = symbols.iter().position(|&known| known == symbol) {
        if let Some(slot) = slots.get_mut(index) {
            *slot = Slot::Unknown;
        }
    }
}

fn unnamed(mut candidate: Candidate) -> Candidate {
    let ExactShape {
        inner,
        outer,
        inlined,
        ..
    } = &candidate.shape;
    for (flag, _) in inlined {
        forget(&mut candidate.merged.outer, outer, *flag);
        forget(&mut candidate.merged.inner, inner, *flag);
    }
    candidate
}

pub(super) fn settle(candidates: Vec<Candidate>, units: &[Unit<'_, '_>]) -> Vec<Candidate> {
    let mut dropped: FxHashSet<usize> = FxHashSet::default();
    loop {
        let kept: Vec<(Span, &Candidate)> = candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| !dropped.contains(index))
            .map(|(_, candidate)| (units[candidate.unit_index].span, candidate))
            .collect();
        let failing: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter(|(index, candidate)| !dropped.contains(index) && !proven(candidate, &kept))
            .map(|(index, _)| index)
            .collect();
        if failing.is_empty() {
            break;
        }
        dropped.extend(failing);
    }
    let kept: Vec<Candidate> = candidates
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !dropped.contains(index))
        .map(|(_, candidate)| unnamed(candidate))
        .collect();
    vouched(deduplicated(kept), units)
}

fn vouched(mut candidates: Vec<Candidate>, units: &[Unit<'_, '_>]) -> Vec<Candidate> {
    loop {
        let spans: Vec<Span> = candidates
            .iter()
            .filter(|candidate| candidate.strong)
            .flat_map(|candidate| candidate.shape.inlined.iter().map(|(_, span)| *span))
            .collect();
        let mut changed = false;
        for candidate in candidates.iter_mut().filter(|candidate| !candidate.strong) {
            if spans.contains(&units[candidate.unit_index].span) {
                candidate.strong = true;
                changed = true;
            }
        }
        if !changed {
            return candidates;
        }
    }
}
