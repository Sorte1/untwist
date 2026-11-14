use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::collect::{DeclKind, Facts};
use super::regions::Regions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    Installer(usize),
    Unused(SymbolId),
}

pub struct Dead<'d> {
    pub facts: &'d Facts,
    pub pinned: &'d FxHashSet<SymbolId>,
    pub eligible: &'d FxHashSet<SymbolId>,
    pub helpers: &'d FxHashSet<SymbolId>,
    counts: FxHashMap<SymbolId, usize>,
    queue: Vec<(Span, Origin)>,
    pub regions: Regions,
    origins: FxHashMap<u32, Origin>,
}

impl<'d> Dead<'d> {
    pub fn new(
        facts: &'d Facts,
        pinned: &'d FxHashSet<SymbolId>,
        eligible: &'d FxHashSet<SymbolId>,
        helpers: &'d FxHashSet<SymbolId>,
    ) -> Self {
        let mut counts: FxHashMap<SymbolId, usize> = FxHashMap::default();
        for site in &facts.refs {
            *counts.entry(site.symbol).or_default() += 1;
        }
        Dead {
            facts,
            pinned,
            eligible,
            helpers,
            counts,
            queue: Vec::new(),
            regions: Regions::default(),
            origins: FxHashMap::default(),
        }
    }

    pub fn remove(&mut self, span: Span, origin: Origin) {
        self.queue.push((span, origin));
    }

    pub fn drain(&mut self) {
        while let Some((span, origin)) = self.queue.pop() {
            let Some(subsumed) = self.regions.insert(span) else {
                continue;
            };
            for inner in &subsumed {
                self.origins.remove(&inner.start);
            }
            self.origins.insert(span.start, origin);
            let facts = self.facts;
            for site in facts.refs_in(span) {
                if subsumed
                    .iter()
                    .any(|inner| inner.start <= site.start && site.start < inner.end)
                {
                    continue;
                }
                self.release(site.symbol, origin);
            }
        }
    }

    pub fn seed_unused(&mut self) {
        let mut unused: Vec<SymbolId> = self
            .eligible
            .iter()
            .filter(|symbol| self.counts.get(symbol).is_none_or(|count| *count == 0))
            .copied()
            .collect();
        unused.sort_unstable();
        for symbol in unused {
            self.counts.insert(symbol, 1);
            self.release(symbol, Origin::Unused(symbol));
        }
    }

    fn release(&mut self, symbol: SymbolId, origin: Origin) {
        let Some(count) = self.counts.get_mut(&symbol) else {
            return;
        };
        *count = count.saturating_sub(1);
        if *count > 0 || !self.eligible.contains(&symbol) {
            return;
        }
        let Some(decl) = self.facts.decls.get(&symbol) else {
            return;
        };
        let removable = !decl.global
            && match decl.kind {
                DeclKind::Function => true,
                DeclKind::Var { pure, call_of } => {
                    pure || call_of.is_some_and(|callee| self.helpers.contains(&callee))
                }
            };
        if removable && !self.pinned.contains(&symbol) {
            self.queue.push((decl.region, origin));
        }
    }

    pub fn origin_at(&self, at: u32) -> Option<Origin> {
        let start = self
            .regions
            .enclosing_start(Span::new(at, at.saturating_add(1)))?;
        self.origins.get(&start).copied()
    }

    pub fn origin_over(&self, span: Span) -> Option<Origin> {
        let start = self.regions.enclosing_start(span)?;
        self.origins.get(&start).copied()
    }
}
