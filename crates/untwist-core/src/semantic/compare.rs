use std::cell::Cell;

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{BindingIdentifier, IdentifierReference, Program};
use oxc_ast_visit::VisitJs;
use oxc_semantic::{Scoping, SemanticBuilder, Stats};

use super::Semantics;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;

#[derive(Default)]
struct Cells {
    scopes: Vec<Option<ScopeId>>,
    bindings: Vec<Option<SymbolId>>,
    references: Vec<Option<ReferenceId>>,
}

impl<'a> VisitJs<'a> for Cells {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        self.scopes.push(scope_id.get());
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.bindings.push(it.symbol_id.get());
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.references.push(it.reference_id.get());
    }
}

fn cells_of(program: &Program<'_>) -> Cells {
    let mut cells = Cells::default();
    cells.visit_program(program);
    cells
}

struct Side<'s> {
    scoping: &'s Scoping,
    cells: Cells,
}

pub fn first_difference(program: &Program<'_>, held: &Scoping) -> Option<String> {
    differs(program, held, None)
}

pub fn patch_difference(program: &Program<'_>, held: &Semantics) -> Option<String> {
    differs(program, &held.scoping, Some(held.stats))
}

fn differs(program: &Program<'_>, held: &Scoping, stats: Option<Stats>) -> Option<String> {
    let alloc = Allocator::default();
    let copy = alloc.alloc(program.clone_in(&alloc));
    let built = SemanticBuilder::new().build(copy).semantic;
    let fresh_stats = built.stats();
    let mut fresh = built.into_scoping();
    super::observers::settle_eval_scopes(copy, &mut fresh);
    if let Some(stats) = stats.filter(|stats| !same_stats(*stats, fresh_stats)) {
        return Some(format!(
            "stats {stats:?} against a fresh build's {fresh_stats:?}"
        ));
    }
    let held = Side {
        scoping: held,
        cells: cells_of(program),
    };
    let fresh = Side {
        scoping: &fresh,
        cells: cells_of(copy),
    };
    compare(&held, &fresh)
}

fn same_stats(held: Stats, fresh: Stats) -> bool {
    (held.nodes, held.scopes, held.symbols, held.references)
        == (fresh.nodes, fresh.scopes, fresh.symbols, fresh.references)
}

fn compare(held: &Side<'_>, fresh: &Side<'_>) -> Option<String> {
    scopes_differ(held, fresh)
        .or_else(|| symbols_differ(held, fresh))
        .or_else(|| match reference_map(held, fresh) {
            Ok(map) => references_differ(held, fresh).or_else(|| lists_differ(held, fresh, &map)),
            Err(difference) => Some(difference),
        })
}

fn scopes_differ(held: &Side<'_>, fresh: &Side<'_>) -> Option<String> {
    let (h, f) = (held.scoping, fresh.scoping);
    if held.cells.scopes != fresh.cells.scopes || h.scopes_len() != f.scopes_len() {
        return Some(format!(
            "scope cells ({} scopes) against a fresh build's ({} scopes)",
            h.scopes_len(),
            f.scopes_len()
        ));
    }
    f.scope_descendants_from_root()
        .find(|&id| scope_row(h, id) != scope_row(f, id))
        .map(|id| format!("scope {id:?}"))
}

type ScopeRow = (Option<ScopeId>, ScopeFlags, Vec<(String, SymbolId)>);

fn scope_row(scoping: &Scoping, id: ScopeId) -> ScopeRow {
    let mut bindings: Vec<(String, SymbolId)> = scoping
        .get_bindings(id)
        .iter()
        .map(|(name, symbol)| (name.as_str().to_string(), *symbol))
        .collect();
    bindings.sort();
    (
        scoping.scope_parent_id(id),
        scoping.scope_flags(id),
        bindings,
    )
}

fn symbols_differ(held: &Side<'_>, fresh: &Side<'_>) -> Option<String> {
    let (h, f) = (held.scoping, fresh.scoping);
    if held.cells.bindings != fresh.cells.bindings || h.symbols_len() != f.symbols_len() {
        return Some(format!(
            "binding cells ({} symbols) against a fresh build's ({} symbols)",
            h.symbols_len(),
            f.symbols_len()
        ));
    }
    f.symbol_ids()
        .find(|&id| symbol_row(h, id) != symbol_row(f, id))
        .map(|id| format!("symbol {id:?} ({})", f.symbol_name(id)))
}

type SymbolRow = (String, String, ScopeId, (u32, u32), Vec<String>);

fn symbol_row(scoping: &Scoping, id: SymbolId) -> SymbolRow {
    let span = scoping.symbol_span(id);
    let redeclarations = scoping
        .symbol_redeclarations(id)
        .iter()
        .map(|r| format!("{:?}/{:?}", r.span, r.flags))
        .collect();
    (
        scoping.symbol_name(id).to_string(),
        format!("{:?}", scoping.symbol_flags(id)),
        scoping.symbol_scope_id(id),
        (span.start, span.end),
        redeclarations,
    )
}

fn reference_map(held: &Side<'_>, fresh: &Side<'_>) -> Result<Vec<Option<ReferenceId>>, String> {
    let mut map: Vec<Option<ReferenceId>> = vec![None; held.scoping.references_len()];
    if held.cells.references.len() != fresh.cells.references.len() {
        return Err("the reference cells differ in number".to_string());
    }
    for (index, (h, f)) in held
        .cells
        .references
        .iter()
        .zip(&fresh.cells.references)
        .enumerate()
    {
        let (Some(h), Some(f)) = (h, f) else {
            return Err(format!("reference cell {index} is empty"));
        };
        let slot = map
            .get_mut(h.index())
            .ok_or_else(|| format!("reference cell {index} names {h:?}, past the table"))?;
        if slot.replace(*f).is_some() {
            return Err(format!("reference {h:?} sits in two cells"));
        }
    }
    Ok(map)
}

fn references_differ(held: &Side<'_>, fresh: &Side<'_>) -> Option<String> {
    let pairs = held.cells.references.iter().zip(&fresh.cells.references);
    let pairs = pairs.filter_map(|(h, f)| Some(((*h)?, (*f)?)));
    pairs.into_iter().find_map(|(h_id, f_id)| {
        let h = held.scoping.get_reference(h_id);
        let f = fresh.scoping.get_reference(f_id);
        let same = h.symbol_id() == f.symbol_id()
            && h.flags() == f.flags()
            && h.scope_id() == f.scope_id();
        (!same).then(|| format!("reference {h_id:?} ({h:?} against {f:?})"))
    })
}

fn mapped(ids: &[ReferenceId], map: &[Option<ReferenceId>]) -> Option<Vec<ReferenceId>> {
    ids.iter()
        .map(|id| map.get(id.index()).copied().flatten())
        .collect()
}

fn lists_differ(held: &Side<'_>, fresh: &Side<'_>, map: &[Option<ReferenceId>]) -> Option<String> {
    let (h, f) = (held.scoping, fresh.scoping);
    let symbol = f.symbol_ids().find(|&id| {
        mapped(h.get_resolved_reference_ids(id), map).as_deref()
            != Some(f.get_resolved_reference_ids(id))
    });
    if let Some(id) = symbol {
        let held = mapped(h.get_resolved_reference_ids(id), map);
        return Some(format!(
            "the resolved references of symbol {id:?} ({}): {held:?} against {:?}",
            f.symbol_name(id),
            f.get_resolved_reference_ids(id)
        ));
    }
    unresolved_differ(h, f, map)
}

fn unresolved_differ(h: &Scoping, f: &Scoping, map: &[Option<ReferenceId>]) -> Option<String> {
    let held = h.root_unresolved_references();
    let fresh = f.root_unresolved_references();
    if held.len() != fresh.len() {
        return Some(format!(
            "{} unresolved names against a fresh build's {}",
            held.len(),
            fresh.len()
        ));
    }
    fresh.iter().find_map(|(name, ids)| {
        let same = held
            .get(name.as_str())
            .and_then(|held_ids| mapped(held_ids, map))
            .is_some_and(|held_ids| held_ids.as_slice() == ids.as_slice());
        (!same).then(|| format!("the unresolved references to {}", name.as_str()))
    })
}

struct Restore {
    cells: Cells,
    scope: usize,
    binding: usize,
    reference: usize,
}

impl<'a> VisitJs<'a> for Restore {
    fn enter_scope(&mut self, _flags: ScopeFlags, scope_id: &Cell<Option<ScopeId>>) {
        scope_id.set(self.cells.scopes.get(self.scope).copied().flatten());
        self.scope += 1;
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        it.symbol_id
            .set(self.cells.bindings.get(self.binding).copied().flatten());
        self.binding += 1;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        it.reference_id
            .set(self.cells.references.get(self.reference).copied().flatten());
        self.reference += 1;
    }
}

pub fn with_fresh_scoping<'a, R>(
    program: &Program<'a>,
    replay: impl FnOnce(&Program<'a>, &Scoping) -> R,
) -> R {
    let held = cells_of(program);
    let fresh = crate::semantic::build_scoping(program).scoping;
    let replayed = replay(program, &fresh);
    let mut restore = Restore {
        cells: held,
        scope: 0,
        binding: 0,
        reference: 0,
    };
    restore.visit_program(program);
    replayed
}
