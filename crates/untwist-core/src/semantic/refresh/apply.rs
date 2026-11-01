use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::node::NodeId;
use oxc_syntax::reference::{Reference, ReferenceId};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::{Fresh, Semantics, Slot, Walked, LISTED, REPLACED, SEEN};

type Placed = (u32, ReferenceId);

fn kept(state: u32) -> bool {
    state & (SEEN | REPLACED) == SEEN
}

pub(super) fn apply(mut scoping: Scoping, mut walked: Walked<'_>) -> Option<Semantics> {
    for (id, flags) in &walked.flag_fixes {
        *scoping.get_reference_mut(*id).flags_mut() = *flags;
    }
    let (resolved, unresolved) = create_fresh(&mut scoping, &walked)?;
    relist_resolved(&mut scoping, &mut walked, resolved)?;
    relist_unresolved(&mut scoping, &mut walked, unresolved)?;
    let every_kept_listed = walked
        .slots
        .iter()
        .all(|slot| !kept(slot.state) || slot.state & LISTED != 0);
    every_kept_listed.then_some(Semantics {
        scoping,
        stats: walked.stats,
        patched: true,
    })
}

type FreshResolved = FxHashMap<SymbolId, Vec<Placed>>;
type FreshUnresolved = FxHashMap<String, Vec<Placed>>;

fn create_fresh(
    scoping: &mut Scoping,
    walked: &Walked<'_>,
) -> Option<(FreshResolved, FreshUnresolved)> {
    let start = walked.slots.len();
    let mut resolved: FreshResolved = FxHashMap::default();
    let mut unresolved: FreshUnresolved = FxHashMap::default();
    for (offset, fresh) in walked.fresh.iter().enumerate() {
        let id = scoping.create_reference(reference_of(fresh));
        if id.index() != start + offset {
            return None;
        }
        match fresh.symbol {
            Some(symbol) => resolved.entry(symbol).or_default().push((fresh.at, id)),
            None => unresolved
                .entry(fresh.name.as_str().to_string())
                .or_default()
                .push((fresh.at, id)),
        }
    }
    Some((resolved, unresolved))
}

fn reference_of(fresh: &Fresh<'_>) -> Reference {
    match fresh.symbol {
        Some(symbol) => {
            Reference::new_with_symbol_id(NodeId::DUMMY, symbol, fresh.scope, fresh.flags)
        }
        None => Reference::new(NodeId::DUMMY, fresh.scope, fresh.flags),
    }
}

fn mark_listed(slots: &mut [Slot], id: ReferenceId) -> Option<Option<u32>> {
    let slot = slots.get_mut(id.index())?;
    if !kept(slot.state) {
        return Some(None);
    }
    if slot.state & LISTED != 0 {
        return None;
    }
    slot.state |= LISTED;
    Some(Some(slot.at))
}

struct Verdict {
    rewrite: bool,
    skipped: Option<ReferenceId>,
}

fn survey(
    walked: &mut Walked<'_>,
    ids: &[ReferenceId],
    owner: impl Fn(ReferenceId) -> bool,
) -> Option<Verdict> {
    let mut verdict = Verdict {
        rewrite: false,
        skipped: None,
    };
    let mut last: Option<u32> = None;
    for &id in ids {
        let at = if let Some(at) = mark_listed(&mut walked.slots, id)? {
            if !owner(id) {
                return None;
            }
            at
        } else if verdict.skipped.is_none() && skip_this_deletion() {
            verdict.skipped = Some(id);
            u32::MAX
        } else {
            verdict.rewrite = true;
            continue;
        };
        verdict.rewrite |= last.is_some_and(|last| last >= at);
        last = Some(at);
    }
    Some(verdict)
}

fn live_of(walked: &Walked<'_>, ids: &[ReferenceId], skipped: Option<ReferenceId>) -> Vec<Placed> {
    ids.iter()
        .filter_map(|&id| {
            if Some(id) == skipped {
                return Some((u32::MAX, id));
            }
            let slot = *walked.slots.get(id.index())?;
            kept(slot.state).then_some((slot.at, id))
        })
        .collect()
}

fn relist_resolved(
    scoping: &mut Scoping,
    walked: &mut Walked<'_>,
    mut fresh: FreshResolved,
) -> Option<()> {
    let mut rewrites: Vec<(SymbolId, Vec<ReferenceId>, Vec<Placed>)> = Vec::new();
    for symbol in scoping.symbol_ids() {
        let ids = scoping.get_resolved_reference_ids(symbol);
        let owner = |id: ReferenceId| scoping.get_reference(id).symbol_id() == Some(symbol);
        let verdict = survey(walked, ids, owner)?;
        let added = fresh.remove(&symbol).unwrap_or_default();
        if verdict.rewrite || !added.is_empty() {
            let live = live_of(walked, ids, verdict.skipped);
            rewrites.push((symbol, ids.to_vec(), merged(live, added)));
        }
    }
    for (symbol, all, placed) in rewrites {
        for id in front_first(&all) {
            scoping.delete_resolved_reference(symbol, id);
        }
        for (_, id) in placed {
            scoping.add_resolved_reference(symbol, id);
        }
    }
    Some(())
}

fn front_first(all: &[ReferenceId]) -> impl Iterator<Item = ReferenceId> + '_ {
    all.split_first()
        .into_iter()
        .flat_map(|(&first, rest)| std::iter::once(first).chain(rest.iter().rev().copied()))
}

fn merged(mut live: Vec<Placed>, added: Vec<Placed>) -> Vec<Placed> {
    live.extend(added);
    live.sort_by_key(|placed| placed.0);
    live
}

fn relist_unresolved(
    scoping: &mut Scoping,
    walked: &mut Walked<'_>,
    mut fresh: FreshUnresolved,
) -> Option<()> {
    let mut rewrites: Vec<(String, Vec<ReferenceId>, Vec<Placed>)> = Vec::new();
    for (name, ids) in scoping.root_unresolved_references() {
        let owner = |id: ReferenceId| scoping.get_reference(id).symbol_id().is_none();
        let verdict = survey(walked, ids, owner)?;
        let added = fresh.remove(name.as_str()).unwrap_or_default();
        if !verdict.rewrite && added.is_empty() {
            continue;
        }
        let placed = merged(live_of(walked, ids, verdict.skipped), added);
        rewrites.push((name.as_str().to_string(), ids.to_vec(), placed));
    }
    rewrites.extend(
        fresh
            .into_iter()
            .map(|(name, added)| (name, Vec::new(), merged(Vec::new(), added))),
    );
    for (name, all, placed) in rewrites {
        let name = Ident::from(name.as_str());
        for id in front_first(&all) {
            scoping.delete_root_unresolved_reference(name, id);
        }
        for (_, id) in placed {
            scoping.add_root_unresolved_reference(name, id);
        }
    }
    Some(())
}

#[cfg(not(feature = "test-hooks"))]
fn skip_this_deletion() -> bool {
    false
}

#[cfg(feature = "test-hooks")]
static SKIP_ONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "test-hooks")]
pub fn skip_one_deletion_for_tests(on: bool) {
    SKIP_ONE.store(on, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "test-hooks")]
fn skip_this_deletion() -> bool {
    SKIP_ONE.swap(false, std::sync::atomic::Ordering::SeqCst)
}
