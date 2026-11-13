use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::shape::{ExactShape, Unit};
use super::{Candidate, Merged, Slot};

fn labels(merged: &Merged) -> impl Iterator<Item = &str> {
    std::iter::once(merged.label.as_str()).chain(merged.also.iter().map(|same| same.label.as_str()))
}

fn shares_label(outer: &Merged, inner: &Merged) -> bool {
    labels(outer).any(|label| labels(inner).any(|other| other == label))
}

fn proven(candidate: &Candidate, kept: &[(Span, &Candidate)]) -> bool {
    candidate.shape.inlined.iter().all(|(_, span)| {
        kept.iter().any(|(unit, inner)| {
            unit == span && shares_label(&candidate.merged, &inner.merged)
        })
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
    candidates
        .into_iter()
        .enumerate()
        .filter(|(index, _)| !dropped.contains(index))
        .map(|(_, candidate)| unnamed(candidate))
        .collect()
}
