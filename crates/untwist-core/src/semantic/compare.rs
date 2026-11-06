use std::cell::Cell;

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{BindingIdentifier, IdentifierReference, Program};
use oxc_ast_visit::VisitJs;
use oxc_semantic::{Scoping, SemanticBuilder, Stats};

use super::Semantics;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;

mod iso;

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

pub(super) struct Side<'s> {
    scoping: &'s Scoping,
    cells: Cells,
}

pub fn first_difference(program: &Program<'_>, held: &Scoping) -> Option<String> {
    differs(program, held, None, true)
}

pub fn patch_difference(program: &Program<'_>, held: &Semantics) -> Option<String> {
    differs(program, &held.scoping, Some(held.stats), held.canonical)
}

fn differs(
    program: &Program<'_>,
    held: &Scoping,
    stats: Option<Stats>,
    canonical: bool,
) -> Option<String> {
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
    iso::compare(&held, &fresh, canonical)
}

fn same_stats(held: Stats, fresh: Stats) -> bool {
    (held.nodes, held.scopes, held.symbols, held.references)
        == (fresh.nodes, fresh.scopes, fresh.symbols, fresh.references)
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
