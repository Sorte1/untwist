use oxc_ast::ast::IdentifierReference;
use oxc_str::Ident;
use oxc_syntax::reference::{ReferenceFlags, ReferenceId};
use oxc_syntax::scope::ScopeId;
use oxc_syntax::symbol::SymbolId;

use super::{name_bit, Frame, Fresh, Walk, NO_KEY, PARAMS, REPLACED, SEEN};

impl<'a> Walk<'_, 'a> {
    pub(super) fn refer(&mut self, it: &IdentifierReference<'a>) {
        let at = self.references;
        self.references += 1;
        let flags = self.take_flags();
        if self.failed {
            return;
        }
        let Some(scope) = self.current_scope() else {
            return self.fail();
        };
        self.prepare();
        match it.reference_id.get() {
            Some(id) if self.claim(id, at) => self.check(id, it, scope, flags, at),
            _ => {
                let symbol = self.resolve(it.name);
                self.renew(it, symbol, (scope, flags, at));
            }
        }
    }

    fn take_flags(&mut self) -> ReferenceFlags {
        if self.flags.is_empty() {
            ReferenceFlags::Read
        } else {
            std::mem::take(&mut self.flags)
        }
    }

    fn claim(&mut self, id: ReferenceId, at: u32) -> bool {
        let params = if self.params_open > 0 { PARAMS } else { 0 };
        match self.slots.get_mut(id.index()) {
            Some(slot) if slot.state & SEEN == 0 => {
                slot.state |= SEEN | params;
                slot.at = at;
                true
            }
            _ => false,
        }
    }

    fn check(
        &mut self,
        id: ReferenceId,
        it: &IdentifierReference<'a>,
        scope: ScopeId,
        flags: ReferenceFlags,
        at: u32,
    ) {
        let reference = self.scoping.get_reference(id);
        let held = reference.symbol_id();
        if held.is_some_and(|symbol| self.scoping.symbol_ident(symbol) != it.name) {
            return self.fail();
        }
        let same = reference.scope_id() == scope
            && self.still_resolves(it.name, held)
            && (held.is_some() || self.listed_as(id, it.name));
        if !same {
            if let Some(slot) = self.slots.get_mut(id.index()) {
                slot.state |= REPLACED;
            }
            let symbol = self.resolve(it.name);
            return self.renew(it, symbol, (scope, flags, at));
        }
        if reference.flags() != flags {
            self.flag_fixes.push((id, flags));
        }
    }

    fn listed_as(&self, id: ReferenceId, name: Ident<'_>) -> bool {
        let key = self
            .slots
            .get(id.index())
            .map_or(NO_KEY, |slot| slot.state & NO_KEY);
        self.keys
            .get(key as usize)
            .is_some_and(|listed| *listed == name)
    }

    fn renew(
        &mut self,
        it: &IdentifierReference<'a>,
        symbol: Option<SymbolId>,
        (scope, flags, at): (ScopeId, ReferenceFlags, u32),
    ) {
        let index = self.scoping.references_len() + self.fresh.len();
        if index >= ReferenceId::MAX_INDEX {
            return self.fail();
        }
        it.reference_id.set(Some(ReferenceId::new(index)));
        self.fresh.push(Fresh {
            at,
            name: it.name,
            symbol,
            scope,
            flags,
            params: self.params_open > 0,
        });
    }

    fn found_in(
        &self,
        frame: &Frame,
        early: &[SymbolId],
        name: Ident<'_>,
        bit: u64,
    ) -> Option<SymbolId> {
        let scoping = self.scoping;
        let found = if frame.in_params {
            early
                .iter()
                .rev()
                .copied()
                .find(|&symbol| scoping.symbol_ident(symbol) == name)
        } else if frame.names & bit == 0 {
            None
        } else {
            scoping.get_binding(frame.scope, name)
        };
        found.filter(|&symbol| scoping.symbol_flags(symbol).can_be_referenced_by_value())
    }

    fn resolve(&self, name: Ident<'_>) -> Option<SymbolId> {
        let bit = name_bit(name);
        let mut end = self.early.len();
        self.frames.iter().rev().find_map(|frame| {
            let early = &self.early[frame.early..end];
            end = frame.early;
            self.found_in(frame, early, name, bit)
        })
    }

    fn early_of(&self, index: usize) -> &[SymbolId] {
        let start = self.frames[index].early;
        let end = self
            .frames
            .get(index + 1)
            .map_or(self.early.len(), |next| next.early);
        &self.early[start..end]
    }

    fn function_binds(&self, above: usize, name: Ident<'_>, bit: u64) -> bool {
        let mut next = self.innermost;
        while let Some(index) = next.filter(|&index| index > above) {
            let frame = &self.frames[index];
            let binds = frame.names & bit != 0
                && self
                    .found_in(frame, self.early_of(index), name, bit)
                    .is_some();
            if binds {
                return true;
            }
            next = frame.outer;
        }
        false
    }

    fn frame_binds(&self, above: usize, name: Ident<'_>, bit: u64) -> bool {
        (above + 1..self.frames.len()).any(|index| {
            let frame = &self.frames[index];
            frame.names & bit != 0
                && self
                    .found_in(frame, self.early_of(index), name, bit)
                    .is_some()
        })
    }

    fn shadowed(&self, above: usize, name: Ident<'_>, bit: u64) -> bool {
        if self.frames.last().is_some_and(|frame| frame.moved) {
            self.frame_binds(above, name, bit)
        } else {
            self.function_binds(above, name, bit)
        }
    }

    fn still_resolves(&self, name: Ident<'_>, held: Option<SymbolId>) -> bool {
        let bit = name_bit(name);
        let Some(symbol) = held else {
            return !self.shadowed(0, name, bit);
        };
        let home = self.scoping.symbol_scope_id(symbol);
        let slot = self.frame_of.get(home.index()).copied().unwrap_or(0);
        let Some(index) = (slot as usize).checked_sub(1) else {
            return false;
        };
        if self.shadowed(index, name, bit) {
            return false;
        }
        let frame = &self.frames[index];
        !frame.in_params || self.found_in(frame, self.early_of(index), name, bit) == held
    }
}
