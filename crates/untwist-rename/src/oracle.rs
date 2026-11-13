use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use untwist_core::host_globals::is_host_global_property;
use untwist_core::semantic::named_by_another_input;

use crate::collect::Collected;
use crate::collisions::{Reason, Tally};
use crate::generate::{uid_candidate, uid_candidates, UID_CANDIDATES};
use crate::holders::{within, Holder, HolderIndex, ScopeTree};
use crate::plan::RenamePlan;
use crate::reserved::{is_reserved, Excluded};

const ANCESTOR_SCAN_LIMIT: usize = 32;

const _: () = assert!(UID_CANDIDATES <= u16::MAX as usize);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OuterRule {
    ProgramWide,
    #[default]
    UsedEnclosing,
}

impl OuterRule {
    pub fn needs_program_references(self) -> bool {
        self == OuterRule::ProgramWide
    }
}

pub struct NameOracle<'s> {
    scoping: &'s Scoping,
    collected: &'s Collected,
    excluded: Excluded,
    rule: OuterRule,
    kept: Option<Excluded>,
    tree: ScopeTree,
    holders: HolderIndex,
    names: FxHashMap<SymbolId, String>,
    handed_out: FxHashSet<String>,
    uid_cursors: FxHashMap<Box<str>, u16>,
    pending: FxHashSet<SymbolId>,
    assigned: Vec<SymbolId>,
    tally: Tally,
}

impl<'s> NameOracle<'s> {
    pub fn new(scoping: &'s Scoping, collected: &'s Collected) -> Self {
        Self::with_kept(scoping, collected, collected.rule, None)
    }

    pub fn keeping(scoping: &'s Scoping, collected: &'s Collected, kept: Excluded) -> Self {
        Self::with_kept(scoping, collected, collected.rule, Some(kept))
    }

    fn with_kept(
        scoping: &'s Scoping,
        collected: &'s Collected,
        rule: OuterRule,
        kept: Option<Excluded>,
    ) -> Self {
        let tree = ScopeTree::new(scoping);
        let holders = index_holders(scoping);
        let mut excluded = Excluded::from_scoping(scoping);
        for name in &collected.block_function_names {
            excluded.insert(name);
        }
        for name in &collected.eval_names {
            excluded.insert(name);
        }
        Self {
            scoping,
            collected,
            excluded,
            rule,
            kept,
            tree,
            holders,
            names: FxHashMap::default(),
            handed_out: FxHashSet::default(),
            uid_cursors: FxHashMap::default(),
            pending: FxHashSet::default(),
            assigned: Vec::new(),
            tally: Tally::default(),
        }
    }

    pub fn defer(&mut self, symbols: impl IntoIterator<Item = SymbolId>) {
        self.pending.extend(symbols);
    }

    fn honours_pending(&self) -> bool {
        self.rule == OuterRule::UsedEnclosing || self.kept.is_some()
    }

    fn is_pending(&self, symbol: SymbolId) -> bool {
        self.honours_pending() && self.pending.contains(&symbol)
    }

    pub fn is_assigned(&self, symbol: SymbolId) -> bool {
        self.names.contains_key(&symbol)
    }

    pub fn current_name(&self, symbol: SymbolId) -> &str {
        self.names
            .get(&symbol)
            .map(String::as_str)
            .unwrap_or_else(|| self.scoping.symbol_name(symbol))
    }

    pub fn is_available(&self, name: &str, symbol: SymbolId) -> bool {
        self.available(name, symbol, false)
    }

    #[doc(hidden)]
    pub fn is_available_by_climb_for_tests(&self, name: &str, symbol: SymbolId) -> bool {
        self.available(name, symbol, true)
    }

    #[inline]
    fn jsx_capital(&self, symbol: SymbolId, name: &str) -> Option<String> {
        let tags = &self.collected.jsx_tags;
        if tags.is_empty() || !tags.contains(&symbol) {
            return None;
        }
        capitalised(name)
    }

    fn available(&self, name: &str, symbol: SymbolId, by_climb: bool) -> bool {
        let capital = self.jsx_capital(symbol, name);
        let name = capital.as_deref().unwrap_or(name);
        if self.excluded.contains(name) || self.lands_on_host_property(name, symbol) {
            self.tally.note(Reason::Excluded);
            return false;
        }
        let conflicts = if by_climb {
            self.holder_conflicts_by_climb(name, symbol)
        } else {
            self.holder_conflicts(name, symbol)
        };
        if conflicts {
            self.tally.note(Reason::BoundInScope);
            return false;
        }
        let refused = match self.rule {
            OuterRule::ProgramWide => self.uid_refusal(name, symbol),
            OuterRule::UsedEnclosing => self
                .captures_a_free_symbol(name, symbol)
                .then_some(Reason::FreeReference),
        };
        match refused {
            Some(reason) => {
                self.tally.note(reason);
                false
            }
            None => true,
        }
    }

    fn lands_on_host_property(&self, name: &str, symbol: SymbolId) -> bool {
        self.collected.classic_script
            && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            && self.scoping.symbol_name(symbol) != name
            && (is_host_global_property(name) || named_by_another_input(name))
    }

    fn uid_refusal(&self, name: &str, symbol: SymbolId) -> Option<Reason> {
        if self.collected.labels.contains(name) {
            return Some(Reason::Label);
        }
        if self.handed_out.contains(name) {
            return Some(Reason::BoundInScope);
        }
        let free = match &self.kept {
            Some(kept) => !kept.contains(name),
            None => {
                !self.collected.program_references.contains(name)
                    || self.scoping.symbol_name(symbol) == name
            }
        };
        (!free).then_some(Reason::FreeReference)
    }

    fn captures_a_free_symbol(&self, name: &str, symbol: SymbolId) -> bool {
        let scope = self.scoping.symbol_scope_id(symbol);
        self.collected.free.get(&scope).is_some_and(|free| {
            free.iter().any(|&other| {
                other != symbol && !self.is_pending(other) && self.current_name(other) == name
            })
        })
    }

    fn is_live_holder(&self, held: &Holder, symbol: SymbolId) -> bool {
        held.symbol != symbol && !self.is_pending(held.symbol)
    }

    fn holder_conflicts(&self, name: &str, symbol: SymbolId) -> bool {
        let scope = self.scoping.symbol_scope_id(symbol);
        let (Some(first), Some(last)) = (self.tree.order(scope), self.tree.last(scope)) else {
            return self.holder_conflicts_by_climb(name, symbol);
        };
        let holders = self.holders.all(name);
        if holders.is_empty() {
            return false;
        }
        let live = |held: &&Holder| self.is_live_holder(held, symbol);
        if within(holders, first, first).iter().any(|held| live(&held)) {
            return true;
        }
        if within(holders, first.saturating_add(1), last)
            .iter()
            .filter(live)
            .any(|held| self.is_free_in(symbol, held.scope))
        {
            return true;
        }
        self.rule == OuterRule::ProgramWide && self.an_ancestor_holds(holders, scope, first, symbol)
    }

    fn an_ancestor_holds(
        &self,
        holders: &[Holder],
        scope: ScopeId,
        first: usize,
        symbol: SymbolId,
    ) -> bool {
        let before = holders
            .get(..holders.partition_point(|held| held.order() < first))
            .unwrap_or(&[]);
        if before.len() <= ANCESTOR_SCAN_LIMIT {
            return before.iter().any(|held| {
                self.tree.last(held.scope).is_some_and(|last| last >= first)
                    && self.is_live_holder(held, symbol)
            });
        }
        self.scoping
            .scope_ancestors(scope)
            .skip(1)
            .filter_map(|ancestor| self.tree.order(ancestor))
            .any(|order| {
                within(before, order, order)
                    .iter()
                    .any(|held| self.is_live_holder(held, symbol))
            })
    }

    fn holder_conflicts_by_climb(&self, name: &str, symbol: SymbolId) -> bool {
        let scope = self.scoping.symbol_scope_id(symbol);
        self.holders.all(name).iter().any(|held| {
            if held.symbol == symbol || self.is_pending(held.symbol) {
                return false;
            }
            if held.scope == scope {
                return true;
            }
            if self.scoping.scope_is_descendant_of(held.scope, scope) {
                return self.is_free_in(symbol, held.scope);
            }
            self.rule == OuterRule::ProgramWide
                && self.scoping.scope_is_descendant_of(scope, held.scope)
        })
    }

    fn is_free_in(&self, symbol: SymbolId, scope: ScopeId) -> bool {
        self.collected
            .free
            .get(&scope)
            .is_some_and(|free| free.contains(&symbol))
    }

    pub fn take(
        &mut self,
        symbol: SymbolId,
        candidates: impl Iterator<Item = String>,
    ) -> Option<String> {
        let found = candidates
            .into_iter()
            .find(|name| self.is_available(name, symbol))?;
        let chosen = self.jsx_capital(symbol, &found).unwrap_or(found);
        self.assign(symbol, chosen.clone());
        Some(chosen)
    }

    pub fn take_uid(&mut self, symbol: SymbolId, base: &str) -> Option<String> {
        let order = self.uid_probe_order(symbol, base);
        self.take(symbol, order)
    }

    #[doc(hidden)]
    pub fn take_uid_counting_probes_for_tests(
        &mut self,
        symbol: SymbolId,
        base: &str,
    ) -> (Option<String>, usize) {
        let mut probes = 0;
        let order = self.uid_probe_order(symbol, base).inspect(|_| probes += 1);
        let taken = self.take(symbol, order);
        (taken, probes)
    }

    fn uid_probe_order<'b>(
        &mut self,
        symbol: SymbolId,
        base: &'b str,
    ) -> impl Iterator<Item = String> + 'b {
        let (start, first, own) = if self.rule == OuterRule::ProgramWide {
            let (start, first) = self.first_not_refused_for_all(base);
            (start, first, self.own_name_before(symbol, base, start))
        } else {
            (0, uid_candidate(base, 0), None)
        };
        let rest =
            (start + 1..UID_CANDIDATES).filter_map(move |position| uid_candidate(base, position));
        own.into_iter().chain(first).chain(rest)
    }

    fn own_name_before(&self, symbol: SymbolId, base: &str, start: usize) -> Option<String> {
        if self.kept.is_some() {
            return None;
        }
        let own = self.scoping.symbol_name(symbol);
        let position = match own.strip_prefix(base)? {
            "" => 0,
            digits if !digits.starts_with('0') => digits.parse::<usize>().ok()?.checked_sub(1)?,
            _ => return None,
        };
        (position < start && uid_candidate(base, position).as_deref() == Some(own))
            .then(|| own.to_string())
    }

    #[doc(hidden)]
    pub fn take_uid_by_rescan_for_tests(&mut self, symbol: SymbolId, base: &str) -> Option<String> {
        self.take(symbol, uid_candidates(base))
    }

    fn first_not_refused_for_all(&mut self, base: &str) -> (usize, Option<String>) {
        let refusals = Refusals {
            handed_out: &self.handed_out,
            collected: self.collected,
            excluded: &self.excluded,
            kept: self.kept.as_ref(),
        };
        let (position, candidate) = match self.uid_cursors.get_mut(base) {
            Some(cursor) => {
                let (position, candidate) = refusals.first_after(base, *cursor);
                *cursor = position;
                (position, candidate)
            }
            None => {
                let (position, candidate) = refusals.first_after(base, 0);
                if position > 0 {
                    self.uid_cursors.insert(base.into(), position);
                }
                (position, candidate)
            }
        };
        (usize::from(position), candidate)
    }

    fn assign(&mut self, symbol: SymbolId, name: String) {
        self.pending.remove(&symbol);
        let scope = self.scoping.symbol_scope_id(symbol);
        let previous = self.current_name(symbol).to_string();
        self.holders.remove(&previous, scope, symbol);
        self.holders.insert(&name, Holder { scope, symbol });
        self.handed_out.insert(name.clone());
        if self.names.insert(symbol, name).is_none() {
            self.assigned.push(symbol);
        }
    }

    pub fn into_plan(self) -> RenamePlan {
        let Self {
            assigned,
            mut names,
            ..
        } = self;
        let mut plan = RenamePlan::new();
        for symbol in assigned {
            if let Some(name) = names.remove(&symbol) {
                plan.set(symbol, name);
            }
        }
        plan
    }
}

struct Refusals<'r> {
    handed_out: &'r FxHashSet<String>,
    collected: &'r Collected,
    excluded: &'r Excluded,
    kept: Option<&'r Excluded>,
}

impl Refusals<'_> {
    fn refuse_everyone(&self, name: &str) -> bool {
        self.handed_out.contains(name)
            || self.collected.labels.contains(name)
            || self.excluded.lists(name)
            || match self.kept {
                Some(kept) => kept.lists(name),
                None => self.collected.program_references.contains(name),
            }
    }

    fn first_after(&self, base: &str, from: u16) -> (u16, Option<String>) {
        let mut position = from;
        let mut candidate = uid_candidate(base, usize::from(position));
        while candidate
            .as_ref()
            .is_some_and(|name| self.refuse_everyone(name) || (position == 0 && is_reserved(name)))
        {
            position = position.saturating_add(1);
            candidate = uid_candidate(base, usize::from(position));
        }
        (position, candidate)
    }
}

fn capitalised(name: &str) -> Option<String> {
    if !name.as_bytes().first()?.is_ascii_lowercase() {
        return None;
    }
    let mut upper = name.to_string();
    upper[..1].make_ascii_uppercase();
    Some(upper)
}

fn index_holders(scoping: &Scoping) -> HolderIndex {
    let mut holders = HolderIndex::default();
    for (scope, bindings) in scoping.iter_bindings() {
        for (name, symbol) in bindings.iter() {
            holders.insert(
                name.as_str(),
                Holder {
                    scope,
                    symbol: *symbol,
                },
            );
        }
    }
    holders
}
