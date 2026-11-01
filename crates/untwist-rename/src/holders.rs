use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Holder {
    pub scope: ScopeId,
    pub symbol: SymbolId,
}

impl Holder {
    pub fn order(&self) -> usize {
        self.scope.index()
    }
}

pub struct ScopeTree {
    last: Vec<u32>,
}

impl ScopeTree {
    pub fn new(scoping: &Scoping) -> Self {
        let last = preorder_last(scoping).unwrap_or_default();
        Self { last }
    }

    pub fn is_numbered(&self) -> bool {
        !self.last.is_empty()
    }

    pub fn order(&self, scope: ScopeId) -> Option<usize> {
        self.last.get(scope.index()).map(|_| scope.index())
    }

    pub fn last(&self, scope: ScopeId) -> Option<usize> {
        self.last
            .get(scope.index())
            .and_then(|last| usize::try_from(*last).ok())
    }
}

fn preorder_last(scoping: &Scoping) -> Option<Vec<u32>> {
    let len = scoping.scopes_len();
    let final_order = u32::try_from(len.checked_sub(1)?).ok()?;
    let mut last = vec![final_order; len];
    let mut open: Vec<ScopeId> = Vec::new();
    for (position, scope) in scoping.scope_descendants_from_root().enumerate() {
        if scope.index() != position {
            return None;
        }
        let parent = scoping.scope_parent_id(scope);
        while open.last().copied() != parent {
            let closed = open.pop()?;
            let slot = last.get_mut(closed.index())?;
            *slot = u32::try_from(position.checked_sub(1)?).ok()?;
        }
        open.push(scope);
    }
    Some(last)
}

#[derive(Default)]
pub struct HolderIndex {
    buckets: FxHashMap<String, Vec<Holder>>,
}

impl HolderIndex {
    pub fn insert(&mut self, name: &str, holder: Holder) {
        let Some(bucket) = self.buckets.get_mut(name) else {
            self.buckets.insert(name.to_string(), vec![holder]);
            return;
        };
        if bucket
            .last()
            .is_none_or(|held| held.order() <= holder.order())
        {
            bucket.push(holder);
            return;
        }
        let at = bucket.partition_point(|held| held.order() <= holder.order());
        bucket.insert(at, holder);
    }

    pub fn remove(&mut self, name: &str, scope: ScopeId, symbol: SymbolId) {
        let Some(bucket) = self.buckets.get_mut(name) else {
            return;
        };
        let mut at = bucket.partition_point(|held| held.order() < scope.index());
        while let Some(held) = bucket.get(at) {
            if held.scope != scope {
                break;
            }
            if held.symbol == symbol {
                bucket.remove(at);
            } else {
                at += 1;
            }
        }
    }

    pub fn all(&self, name: &str) -> &[Holder] {
        self.buckets.get(name).map_or(&[], Vec::as_slice)
    }
}

pub fn within(holders: &[Holder], first: usize, last: usize) -> &[Holder] {
    let start = holders.partition_point(|held| held.order() < first);
    let end = holders.partition_point(|held| held.order() <= last);
    holders.get(start..end).unwrap_or(&[])
}
