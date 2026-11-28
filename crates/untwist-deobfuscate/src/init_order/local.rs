use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use untwist_core::semantic::top_level_is_local;

use super::{try_scan_init_order, DeclarationSlot, InitOrder};
use crate::mutants::early_read_guards_dropped;

pub(crate) fn read_before_local_declaration(
    program: &Program<'_>,
    scoping: &Scoping,
    bound: &[(Span, SymbolId)],
    harmless: &FxHashSet<Span>,
) -> FxHashSet<SymbolId> {
    if bound.is_empty() || early_read_guards_dropped() {
        return FxHashSet::default();
    }
    let every = || bound.iter().map(|(_, symbol)| *symbol).collect();
    if !top_level_is_local(program) {
        return every();
    }
    let Some(order) = try_scan_init_order(program, scoping) else {
        return every();
    };
    if order.hazard || order.reentrant {
        return every();
    }
    let ignored = FxHashSet::default();
    bound
        .iter()
        .filter(|(statement, symbol)| {
            !order.declaration_slot(*statement).is_some_and(|slot| {
                order.symbol_is_initialised_before_use(slot, *symbol, &ignored)
                    || order.symbol_is_set_before_local_use(slot, *symbol, harmless)
            })
        })
        .map(|(_, symbol)| *symbol)
        .collect()
}

impl InitOrder {
    fn symbol_is_set_before_local_use(
        &self,
        declaration: DeclarationSlot,
        symbol: SymbolId,
        harmless: &FxHashSet<Span>,
    ) -> bool {
        self.sites
            .of_symbol(symbol)
            .all(|site| self.local_site_is_set(declaration, site, harmless))
    }

    fn local_site_is_set(
        &self,
        declaration: DeclarationSlot,
        site: u32,
        harmless: &FxHashSet<Span>,
    ) -> bool {
        let Some(record) = self.sites.get(site) else {
            return false;
        };
        let Some(entry) = self.frame_in_list(record.frame, declaration.list) else {
            return false;
        };
        let statement_is_safe = match entry.hoisted {
            None => entry.index > declaration.index,
            Some(hoisted) => {
                self.prefix_cannot_call(declaration)
                    || self.unreachable_before_local(declaration, hoisted, harmless)
            }
        };
        statement_is_safe
            || self
                .outermost_value(record.value, declaration.list)
                .is_some_and(|value| {
                    self.roots_cannot_call(declaration)
                        || !self.value_runs_before(value, declaration)
                })
    }

    fn unreachable_before_local(
        &self,
        declaration: DeclarationSlot,
        hoisted: SymbolId,
        harmless: &FxHashSet<Span>,
    ) -> bool {
        let mut seen: FxHashSet<SymbolId> = FxHashSet::default();
        let mut pending = vec![hoisted];
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            for site in self.sites.of_symbol(current) {
                let Some(record) = self.sites.get(site) else {
                    return false;
                };
                if harmless.contains(&record.span()) {
                    continue;
                }
                let Some(entry) = self.frame_in_list(record.frame, declaration.list) else {
                    return false;
                };
                match entry.hoisted {
                    Some(symbol) => pending.push(symbol),
                    None if entry.index > declaration.index => {}
                    None => return false,
                }
            }
        }
        true
    }
}
