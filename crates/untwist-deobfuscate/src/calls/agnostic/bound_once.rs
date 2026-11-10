use std::cell::RefCell;

use oxc_semantic::Scoping;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::bindings::block_function_names;

const UNKNOWN: u8 = 0;
const HOLDS: u8 = 1;
const FAILS: u8 = 2;

pub(super) struct Stable<'s> {
    bound: BoundOnce<'s>,
    known: RefCell<Vec<u8>>,
}

impl Stable<'_> {
    pub(super) fn holds(&self, symbol: SymbolId) -> bool {
        let index = symbol.index();
        match self.known.borrow().get(index).copied() {
            Some(HOLDS) => return true,
            Some(FAILS) | None => return false,
            Some(_) => {}
        }
        let found = self.bound.holds(symbol);
        if let Some(slot) = self.known.borrow_mut().get_mut(index) {
            *slot = if found { HOLDS } else { FAILS };
        }
        found
    }
}

pub(super) struct BoundOnce<'s> {
    scoping: &'s Scoping,
    block_functions: FxHashSet<&'s str>,
    has_with: bool,
    eval_judged: bool,
}

impl<'s> BoundOnce<'s> {
    pub(super) fn new(scoping: &'s Scoping) -> Self {
        let has_with = scoping
            .scope_descendants_from_root()
            .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With));
        Self {
            scoping,
            block_functions: block_function_names(scoping),
            has_with,
            eval_judged: false,
        }
    }

    pub(super) fn with_judged_eval(scoping: &'s Scoping) -> Self {
        Self {
            eval_judged: true,
            ..Self::new(scoping)
        }
    }

    pub(super) fn holds(&self, symbol: SymbolId) -> bool {
        let scoping = self.scoping;
        let declared = scoping.symbol_scope_id(symbol);
        scoping.symbol_redeclarations(symbol).is_empty()
            && !scoping
                .get_resolved_references(symbol)
                .any(|reference| reference.is_write())
            && !self.block_functions.contains(scoping.symbol_name(symbol))
            && (self.eval_judged || !scoping.scope_flags(declared).contains_direct_eval())
            && !(self.has_with && self.seen_through_with(symbol))
    }

    pub(super) fn table(self) -> Stable<'s> {
        let symbols = self.scoping.symbols_len();
        Stable {
            bound: self,
            known: RefCell::new(vec![UNKNOWN; symbols]),
        }
    }

    fn seen_through_with(&self, symbol: SymbolId) -> bool {
        let scoping = self.scoping;
        let declared = scoping.symbol_scope_id(symbol);
        scoping.get_resolved_references(symbol).any(|reference| {
            scoping
                .scope_ancestors(reference.scope_id())
                .take_while(|scope| *scope != declared)
                .any(|scope| scoping.scope_flags(scope).contains(ScopeFlags::With))
        })
    }
}
