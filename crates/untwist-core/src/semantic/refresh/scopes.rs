use oxc_syntax::scope::{ScopeFlags, ScopeId};

use super::{name_bit, Frame, Region, Walk};

const FIXABLE: ScopeFlags = ScopeFlags::StrictMode.union(ScopeFlags::DirectEval);

fn region_of(flags: ScopeFlags) -> Region {
    if flags.is_function() {
        Region::Function
    } else if flags.is_catch_clause() {
        Region::Catch
    } else {
        Region::Plain
    }
}

impl Walk<'_, '_> {
    pub(super) fn enter(&mut self, flags: ScopeFlags, cell: Option<ScopeId>) {
        let parent = self.frames.last();
        let strict = parent.map_or(ScopeFlags::empty(), |frame| {
            frame.expected & ScopeFlags::StrictMode
        });
        let parent_scope = parent.map(|frame| frame.scope);
        let parent_moved = parent.is_some_and(|frame| frame.moved);
        let catch_body =
            parent.is_some_and(|frame| frame.region == Region::Catch && !frame.in_params);
        let claimed = self.claim_scope(cell, parent_scope);
        let scope = claimed.map(|(scope, _)| scope);
        let names = scope.map_or(0, |scope| self.names_of(scope));
        let index = self.frames.len();
        if let Some(slot) = scope.and_then(|scope| self.frame_of.get_mut(scope.index())) {
            *slot = u32::try_from(index + 1).unwrap_or(0);
        }
        let outer = self.innermost;
        if flags.is_function() {
            self.innermost = Some(index);
        }
        self.frames.push(Frame {
            scope: scope.unwrap_or(ScopeId::new(0)),
            expected: flags | strict,
            direct_eval: false,
            region: region_of(flags),
            in_params: false,
            names,
            early: self.early.len(),
            outer,
            moved: parent_moved || claimed.is_some_and(|(_, fixed)| fixed),
        });
        match scope {
            Some(scope) if catch_body => self.settle_catch_params(scope),
            Some(_) => {}
            None => self.fail(),
        }
    }

    fn names_of(&self, scope: ScopeId) -> u64 {
        self.scoping
            .get_bindings(scope)
            .keys()
            .fold(0, |names, name| names | name_bit(*name))
    }

    fn claim_scope(
        &mut self,
        cell: Option<ScopeId>,
        parent: Option<ScopeId>,
    ) -> Option<(ScopeId, bool)> {
        let scope = cell.filter(|scope| scope.index() < self.scoping.scopes_len())?;
        if !self.scopes.claim(scope.index()) {
            return None;
        }
        let held = self.scoping.scope_parent_id(scope);
        if held == parent {
            return Some((scope, false));
        }
        let parent = held.and(parent)?;
        if mutants::misplaced() {
            return Some((scope, true));
        }
        self.parent_fixes.push((scope, parent));
        Some((scope, true))
    }

    pub(super) fn leave(&mut self) {
        let Some(frame) = self.frames.pop() else {
            return self.fail();
        };
        self.early.truncate(frame.early);
        if self.innermost == Some(self.frames.len()) {
            self.innermost = frame.outer;
        }
        if let Some(slot) = self.frame_of.get_mut(frame.scope.index()) {
            *slot = 0;
        }
        if self.failed {
            return;
        }
        let actual = if frame.direct_eval {
            frame.expected | ScopeFlags::DirectEval
        } else {
            frame.expected
        };
        let held = self.scoping.scope_flags(frame.scope);
        if held != actual {
            if !FIXABLE.contains(held.symmetric_difference(actual)) {
                return self.fail();
            }
            self.scope_flag_fixes.push((frame.scope, actual));
        }
        if let (true, Some(parent)) = (frame.direct_eval, self.frames.last_mut()) {
            parent.direct_eval = true;
        }
    }

    pub(super) fn direct_eval(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.direct_eval = true;
        }
    }

    pub(super) fn current_scope(&self) -> Option<ScopeId> {
        self.frames.last().map(|frame| frame.scope)
    }

    fn settle_catch_params(&mut self, body: ScopeId) {
        let Some(catch) = self.frames.len().checked_sub(2) else {
            return self.fail();
        };
        let catch = &self.frames[catch];
        let (catch_scope, start) = (catch.scope, catch.early);
        if !self.scoping.get_bindings(catch_scope).is_empty() {
            return self.fail();
        }
        let settled = self.early[start..].iter().all(|&symbol| {
            self.scoping.symbol_scope_id(symbol) == body
                && self
                    .scoping
                    .get_binding(body, self.scoping.symbol_ident(symbol))
                    == Some(symbol)
        });
        if !settled {
            self.fail();
        }
    }
}

#[cfg(not(feature = "test-hooks"))]
mod mutants {
    pub(super) fn misplaced() -> bool {
        false
    }
}

#[cfg(feature = "test-hooks")]
pub(super) mod mutants {
    use std::sync::atomic::{AtomicBool, Ordering};

    static MISPLACE_ONE: AtomicBool = AtomicBool::new(false);

    pub fn misplace_one_scope_for_tests(on: bool) {
        MISPLACE_ONE.store(on, Ordering::SeqCst);
    }

    pub(super) fn misplaced() -> bool {
        MISPLACE_ONE.swap(false, Ordering::SeqCst)
    }
}
