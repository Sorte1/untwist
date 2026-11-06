use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};

use super::{Walk, PARAMS, REPLACED, SEEN};

pub(super) enum Relink {
    Held {
        id: ReferenceId,
        at: u32,
        scope: ScopeId,
        name: String,
    },
    Fresh(usize),
}

const DEAD_SHARE: usize = 4;

pub(super) struct Dead {
    pub(super) scopes: Vec<ScopeId>,
    pub(super) symbols: Vec<SymbolId>,
    pub(super) bound: Vec<SymbolId>,
    pub(super) relinks: Vec<Relink>,
}

pub(super) fn over_share(dead: usize, len: usize) -> bool {
    dead > 0 && dead.saturating_mul(DEAD_SHARE) > len
}

impl Dead {
    pub(super) fn of(walk: &Walk<'_, '_>) -> Option<Self> {
        let scoping = walk.scoping;
        let scopes: Vec<ScopeId> = walk.scopes.unclaimed().map(ScopeId::new).collect();
        let symbols: Vec<SymbolId> = walk.symbols.unclaimed().map(SymbolId::new).collect();
        if over_share(
            scopes.len() + symbols.len(),
            scoping.scopes_len() + scoping.symbols_len(),
        ) {
            return None;
        }
        let bound: Vec<SymbolId> = symbols
            .iter()
            .copied()
            .filter(|&symbol| {
                let scope = scoping.symbol_scope_id(symbol);
                scoping.get_binding(scope, scoping.symbol_ident(symbol)) == Some(symbol)
            })
            .collect();
        let entries: usize = scoping
            .iter_bindings()
            .map(|(_, bindings)| bindings.len())
            .sum();
        if entries != walk.symbols.count() + bound.len() {
            return None;
        }
        let relinks = walk.relinks(&symbols)?;
        Some(Self {
            scopes,
            symbols,
            bound,
            relinks,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.scopes.is_empty() && self.symbols.is_empty()
    }

    pub(super) fn unlink(&self, scoping: &mut Scoping) {
        let kept = if self.bound.is_empty() {
            0
        } else {
            mutants::kept_bindings()
        };
        for &symbol in self.bound.iter().skip(kept) {
            let scope = scoping.symbol_scope_id(symbol);
            let name = scoping.symbol_name(symbol).to_string();
            scoping.remove_binding(scope, Ident::from(name.as_str()));
        }
        for &symbol in &self.symbols {
            *scoping.symbol_flags_mut(symbol) = SymbolFlags::empty();
            scoping.clear_symbol_redeclarations(symbol);
        }
        scoping.set_symbol_names(&self.symbols, Ident::from(""));
        for &scope in &self.scopes {
            *scoping.scope_flags_mut(scope) = ScopeFlags::empty();
        }
    }
}

impl Walk<'_, '_> {
    fn relinks(&self, dead: &[SymbolId]) -> Option<Vec<Relink>> {
        let scoping = self.scoping;
        let mut relinks = Vec::new();
        for &symbol in dead {
            for &id in scoping.get_resolved_reference_ids(symbol) {
                let Some(slot) = self.slots.get(id.index()) else {
                    continue;
                };
                if slot.state & (SEEN | REPLACED) != SEEN {
                    continue;
                }
                if slot.state & PARAMS != 0 {
                    return None;
                }
                relinks.push(Relink::Held {
                    id,
                    at: slot.at,
                    scope: scoping.get_reference(id).scope_id(),
                    name: scoping.symbol_name(symbol).to_string(),
                });
            }
        }
        for (index, fresh) in self.fresh.iter().enumerate() {
            let Some(symbol) = fresh.symbol else {
                continue;
            };
            if self.symbols.is_claimed(symbol.index()) {
                continue;
            }
            if fresh.params {
                return None;
            }
            relinks.push(Relink::Fresh(index));
        }
        Some(relinks)
    }
}

#[cfg(not(feature = "test-hooks"))]
mod mutants {
    pub(super) fn kept_bindings() -> usize {
        0
    }
}

#[cfg(feature = "test-hooks")]
pub(super) mod mutants {
    use std::sync::atomic::{AtomicBool, Ordering};

    static KEEP_ONE: AtomicBool = AtomicBool::new(false);

    pub fn keep_one_dead_binding_for_tests(on: bool) {
        KEEP_ONE.store(on, Ordering::SeqCst);
    }

    pub(super) fn kept_bindings() -> usize {
        usize::from(KEEP_ONE.swap(false, Ordering::SeqCst))
    }
}
