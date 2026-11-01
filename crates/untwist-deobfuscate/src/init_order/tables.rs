use std::cell::OnceCell;
use std::cmp::Reverse;
use std::collections::binary_heap::{BinaryHeap, PeekMut};
use std::hash::BuildHasher;

use hashbrown::hash_table::{Entry, HashTable};
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxBuildHasher;

pub(super) const NONE: u32 = u32::MAX;
const RUN_DIVISOR: usize = 64;

type RunHead = Reverse<((u32, u32, u32), usize)>;

pub(super) fn id(len: usize) -> Option<u32> {
    u32::try_from(len).ok().filter(|id| *id != NONE)
}

pub(super) fn at(id: u32) -> usize {
    id as usize
}

pub(super) fn some(id: u32) -> Option<u32> {
    (id != NONE).then_some(id)
}

fn span_hash(span: Span) -> u64 {
    FxBuildHasher.hash_one(span)
}

#[derive(Default)]
pub(super) struct SpanSlots {
    spans: Vec<Span>,
    table: HashTable<u32>,
}

impl SpanSlots {
    pub(super) fn with_capacity(capacity: usize) -> Self {
        Self {
            spans: Vec::with_capacity(capacity),
            table: HashTable::with_capacity(capacity),
        }
    }

    #[cfg(test)]
    pub(super) fn capacity(&self) -> usize {
        self.spans.capacity()
    }

    pub(super) fn len(&self) -> usize {
        self.spans.len()
    }

    #[cfg(debug_assertions)]
    pub(super) fn spans(&self) -> &[Span] {
        &self.spans
    }

    #[inline(always)]
    pub(super) fn insert(&mut self, span: Span) -> Option<u32> {
        let ordinal = id(self.spans.len())?;
        self.spans.push(span);
        let Self { spans, table } = self;
        let same = |stored: &u32| spans.get(at(*stored)) == Some(&span);
        let rehash = |stored: &u32| spans.get(at(*stored)).map_or(0, |span| span_hash(*span));
        match table.entry(span_hash(span), same, rehash) {
            Entry::Occupied(mut entry) => *entry.get_mut() = ordinal,
            Entry::Vacant(entry) => {
                entry.insert(ordinal);
            }
        }
        Some(ordinal)
    }

    pub(super) fn get(&self, span: Span) -> Option<u32> {
        self.table
            .find(span_hash(span), |stored| {
                self.spans.get(at(*stored)) == Some(&span)
            })
            .copied()
    }
}

#[derive(Clone, Copy, Hash)]
pub(super) struct Site {
    pub(super) frame: u32,
    #[cfg(test)]
    pub(super) functions: u32,
    pub(super) value: u32,
    next: u32,
    start: u32,
    end: u32,
}

#[derive(Default)]
pub(super) struct Sites {
    records: Vec<Site>,
    by_span: OnceCell<Vec<u32>>,
    first: Vec<u32>,
    last: Vec<u32>,
}

impl Sites {
    pub(super) fn with_capacity(references: usize, symbols: usize) -> Self {
        Self {
            records: Vec::with_capacity(references),
            by_span: OnceCell::new(),
            first: vec![NONE; symbols],
            last: vec![NONE; symbols],
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.records.len()
    }

    #[cfg(debug_assertions)]
    pub(super) fn content(&self) -> (&[Site], &[u32]) {
        (&self.records, &self.first)
    }

    pub(super) fn get(&self, site: u32) -> Option<Site> {
        self.records.get(at(site)).copied()
    }

    #[inline(always)]
    pub(super) fn push(
        &mut self,
        frame: u32,
        functions: u32,
        value: u32,
        span: Span,
        symbol: Option<SymbolId>,
    ) -> Option<()> {
        let site = id(self.records.len())?;
        #[cfg(not(test))]
        let _ = functions;
        self.records.push(Site {
            frame,
            #[cfg(test)]
            functions,
            value,
            next: NONE,
            start: span.start,
            end: span.end,
        });
        if let Some(symbol) = symbol {
            self.link(symbol.index(), site);
        }
        Some(())
    }

    #[inline(always)]
    fn link(&mut self, symbol: usize, site: u32) {
        if self.last.len() <= symbol {
            self.first.resize(symbol.saturating_add(1), NONE);
            self.last.resize(symbol.saturating_add(1), NONE);
        }
        let previous = self.last.get(symbol).copied().and_then(some);
        let slot = match previous {
            Some(previous) => self
                .records
                .get_mut(at(previous))
                .map(|site| &mut site.next),
            None => self.first.get_mut(symbol),
        };
        if let Some(slot) = slot {
            *slot = site;
        }
        if let Some(last) = self.last.get_mut(symbol) {
            *last = site;
        }
    }

    pub(super) fn of_symbol(&self, symbol: SymbolId) -> impl Iterator<Item = u32> + '_ {
        let first = self.first.get(symbol.index()).copied().and_then(some);
        std::iter::successors(first, |site| {
            self.records
                .get(at(*site))
                .and_then(|record| some(record.next))
        })
    }

    fn key(&self, site: u32) -> (u32, u32, u32) {
        self.records
            .get(at(site))
            .map_or((NONE, NONE, site), |record| {
                (record.start, record.end, site)
            })
    }

    pub(super) fn seal(&mut self) {
        self.last = Vec::new();
    }

    pub(super) fn forget_span_index(&mut self) {
        self.by_span.take();
    }

    fn by_span(&self) -> &[u32] {
        self.by_span.get_or_init(|| self.span_order())
    }

    fn span_order(&self) -> Vec<u32> {
        let limit = self.records.len() / RUN_DIVISOR + 1;
        match self.descents(limit) {
            Some(0) => self.identity(),
            Some(_) => self.merged(&self.run_starts()),
            None => self.sorted(),
        }
    }

    fn identity(&self) -> Vec<u32> {
        let mut by_span = Vec::with_capacity(self.records.len());
        by_span.extend(0..u32::try_from(self.records.len()).unwrap_or(NONE));
        by_span
    }

    fn sorted(&self) -> Vec<u32> {
        let mut by_span = self.identity();
        by_span.sort_unstable_by_key(|site| self.key(*site));
        by_span
    }

    fn descends(pair: &[Site]) -> bool {
        let [before, after] = pair else {
            return false;
        };
        (after.start, after.end) < (before.start, before.end)
    }

    fn descents(&self, limit: usize) -> Option<usize> {
        let mut count = 0usize;
        for pair in self.records.windows(2) {
            if Self::descends(pair) {
                count += 1;
                if count >= limit {
                    return None;
                }
            }
        }
        Some(count)
    }

    fn run_starts(&self) -> Vec<u32> {
        let mut starts = vec![0];
        for (index, pair) in self.records.windows(2).enumerate() {
            if Self::descends(pair) {
                starts.push(id(index.saturating_add(1)).unwrap_or(NONE));
            }
        }
        starts
    }

    fn merged(&self, starts: &[u32]) -> Vec<u32> {
        let len = self.records.len();
        let mut by_span = Vec::with_capacity(len);
        let end_of = |run: usize| {
            starts
                .get(run.saturating_add(1))
                .map_or(len, |start| at(*start))
        };
        let mut heap: BinaryHeap<RunHead> = starts
            .iter()
            .enumerate()
            .map(|(run, start)| Reverse((self.key(*start), run)))
            .collect();
        while let Some(mut top) = heap.peek_mut() {
            let Reverse(((_, _, site), run)) = *top;
            by_span.push(site);
            let next = site.saturating_add(1);
            if at(next) < end_of(run) {
                *top = Reverse((self.key(next), run));
            } else {
                PeekMut::pop(top);
            }
        }
        by_span
    }

    pub(super) fn at_span(&self, span: Span) -> Option<u32> {
        let target = (span.start, span.end);
        let by_span = self.by_span();
        let end = by_span.partition_point(|site| {
            let (start, end, _) = self.key(*site);
            (start, end) <= target
        });
        let site = *by_span.get(end.checked_sub(1)?)?;
        let record = self.records.get(at(site))?;
        (record.start == span.start && record.end == span.end).then_some(site)
    }
}

#[derive(Default)]
pub(super) struct SymbolSet {
    members: Vec<bool>,
}

impl SymbolSet {
    pub(super) fn with_capacity(symbols: usize) -> Self {
        Self {
            members: vec![false; symbols],
        }
    }

    pub(super) fn insert(&mut self, symbol: SymbolId) -> bool {
        let index = symbol.index();
        if self.members.len() <= index {
            self.members.resize(index.saturating_add(1), false);
        }
        self.members
            .get_mut(index)
            .is_some_and(|member| !std::mem::replace(member, true))
    }

    pub(super) fn contains(&self, symbol: SymbolId) -> bool {
        self.members.get(symbol.index()).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(index: usize) -> SymbolId {
        SymbolId::from_usize(index)
    }

    #[test]
    fn an_index_that_does_not_fit_u32_below_none_has_no_id() {
        assert_eq!(id(0), Some(0));
        assert_eq!(id(u32::MAX as usize - 1), Some(u32::MAX - 1));
        assert_eq!(id(u32::MAX as usize), None);
        assert_eq!(id(usize::MAX), None);
    }

    #[test]
    fn a_repeated_statement_span_answers_with_its_latest_ordinal() {
        let mut slots = SpanSlots::default();
        let (a, b) = (Span::new(0, 4), Span::new(5, 9));
        assert_eq!(slots.insert(a), Some(0));
        assert_eq!(slots.insert(b), Some(1));
        assert_eq!(slots.get(a), Some(0));
        assert_eq!(slots.insert(a), Some(2));
        assert_eq!(slots.get(a), Some(2));
        assert_eq!(slots.get(b), Some(1));
        assert_eq!(slots.get(Span::new(0, 5)), None);
        assert_eq!(slots.len(), 3);
    }

    #[test]
    fn a_repeated_reference_span_answers_with_its_latest_site() {
        let mut sites = Sites::with_capacity(0, 0);
        let spans = [
            Span::new(9, 10),
            Span::new(2, 3),
            Span::new(9, 10),
            Span::new(2, 4),
            Span::new(0, 1),
        ];
        for span in spans {
            assert_eq!(sites.push(0, 0, NONE, span, None), Some(()));
        }
        sites.seal();
        assert_eq!(sites.at_span(Span::new(9, 10)), Some(2));
        assert_eq!(sites.at_span(Span::new(2, 3)), Some(1));
        assert_eq!(sites.at_span(Span::new(2, 4)), Some(3));
        assert_eq!(sites.at_span(Span::new(0, 1)), Some(4));
        assert_eq!(sites.at_span(Span::new(2, 2)), None);
        assert_eq!(sites.at_span(Span::new(11, 12)), None);
    }

    struct Rng(u64);

    impl Rng {
        fn below(&mut self, bound: u32) -> u32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            u32::try_from(self.0 % u64::from(bound.max(1))).unwrap()
        }
    }

    fn walk_spans(rng: &mut Rng, count: usize, disorder: u32) -> Vec<Span> {
        let mut at = 0;
        let mut spans = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(last) = spans.last().copied().filter(|_| rng.below(9) == 0) {
                spans.push(last);
                continue;
            }
            let start = if rng.below(disorder) == 0 {
                rng.below(at + 8) | (rng.below(2) << 31)
            } else {
                at += 1 + rng.below(3);
                at
            };
            spans.push(Span::new(start, start + rng.below(4)));
        }
        spans
    }

    fn model_by_span(spans: &[Span]) -> Vec<u32> {
        let mut keyed: Vec<(u32, u32, u32)> = (0..spans.len())
            .map(|site| (spans[site].start, spans[site].end, site as u32))
            .collect();
        keyed.sort();
        keyed.into_iter().map(|(_, _, site)| site).collect()
    }

    fn assert_sealed_as_the_model(rng: &mut Rng, spans: &[Span]) {
        let mut sites = Sites::with_capacity(spans.len(), 0);
        for span in spans {
            assert_eq!(sites.push(0, 0, NONE, *span, None), Some(()));
        }
        sites.seal();
        let latest = |span: Span| (0..spans.len()).rev().find(|site| spans[*site] == span);
        for _ in 0..8 {
            let start = rng.below(64);
            let span = Span::new(start, start + rng.below(4));
            assert_eq!(sites.at_span(span), latest(span).map(|site| site as u32));
        }
        for span in spans {
            assert_eq!(sites.at_span(*span), latest(*span).map(|site| site as u32));
        }
        assert_eq!(
            sites.by_span(),
            model_by_span(spans).as_slice(),
            "{spans:?}"
        );
    }

    #[test]
    fn sealed_sites_order_and_answer_as_todays_sort() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for count in [0, 1, 2, 3, 7, 64, 200, 1000] {
            for disorder in [1, 2, 9, 70, 400, u32::MAX] {
                for _ in 0..12 {
                    let spans = walk_spans(&mut rng, count, disorder);
                    assert_sealed_as_the_model(&mut rng, &spans);
                }
            }
        }
    }

    #[test]
    fn a_symbol_lists_its_sites_in_the_order_they_were_seen() {
        let mut sites = Sites::with_capacity(0, 1);
        let span = Span::new(0, 1);
        for owner in [3, 0, 3, 7, 3, 0] {
            assert_eq!(sites.push(0, 0, NONE, span, Some(symbol(owner))), Some(()));
        }
        assert_eq!(sites.push(0, 0, NONE, span, None), Some(()));
        sites.seal();
        let listed = |index| sites.of_symbol(symbol(index)).collect::<Vec<u32>>();
        assert_eq!(listed(3), vec![0, 2, 4]);
        assert_eq!(listed(0), vec![1, 5]);
        assert_eq!(listed(7), vec![3]);
        assert!(listed(5).is_empty());
        assert!(listed(100).is_empty());
    }

    #[test]
    fn a_symbol_set_reports_whether_an_insert_was_new() {
        let mut set = SymbolSet::with_capacity(2);
        assert!(!set.contains(symbol(9)));
        assert!(set.insert(symbol(9)));
        assert!(!set.insert(symbol(9)));
        assert!(set.contains(symbol(9)));
        assert!(set.insert(symbol(1)));
        assert!(!set.contains(symbol(0)));
    }
}
