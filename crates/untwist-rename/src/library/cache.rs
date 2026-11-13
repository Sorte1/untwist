use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeId;
use rustc_hash::FxHasher;

use super::shape::{shape_program, Shaped};
use super::table::Table;
use super::LibraryNames;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    tokens: u32,
    shape: u64,
    units: u64,
    scoping: u64,
}

struct Entry {
    table: Weak<Table>,
    key: Key,
    found: Arc<LibraryNames>,
}

thread_local! {
    static LAST: RefCell<Option<Entry>> = const { RefCell::new(None) };
}

fn scoping_fingerprint(scoping: &Scoping) -> u64 {
    let mut hasher = FxHasher::default();
    scoping.symbols_len().hash(&mut hasher);
    for symbol in scoping.symbol_ids() {
        let span = scoping.symbol_span(symbol);
        (span.start, span.end).hash(&mut hasher);
        scoping.symbol_flags(symbol).bits().hash(&mut hasher);
        scoping.symbol_scope_id(symbol).index().hash(&mut hasher);
    }
    scoping.scopes_len().hash(&mut hasher);
    for index in 0..scoping.scopes_len() {
        let parent = scoping.scope_parent_id(ScopeId::from_usize(index));
        parent.map(ScopeId::index).hash(&mut hasher);
    }
    scoping.references_len().hash(&mut hasher);
    for index in 0..scoping.references_len() {
        let reference = scoping.get_reference(ReferenceId::from_usize(index));
        reference.symbol_id().map(|symbol| symbol.index()).hash(&mut hasher);
        reference.flags().bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn key_of(shaped: &Shaped<'_, '_>, scoping: &Scoping) -> Key {
    let tokens = shaped.units.last().map_or(0, |program| program.len);
    let mut units = FxHasher::default();
    for unit in &shaped.units {
        (unit.span.start, unit.span.end, unit.start, unit.len).hash(&mut units);
        unit.binding.map(|symbol| symbol.index()).hash(&mut units);
    }
    Key {
        tokens,
        shape: shaped.prefix.range(0, tokens),
        units: units.finish(),
        scoping: scoping_fingerprint(scoping),
    }
}

pub(super) fn matched(
    table: &Arc<Table>,
    program: &Program<'_>,
    scoping: &Scoping,
    compute: impl FnOnce(&Table, &Shaped<'_, '_>, &Scoping) -> LibraryNames,
) -> Arc<LibraryNames> {
    if table.is_empty() {
        return Arc::new(LibraryNames::default());
    }
    let shaped = shape_program(program, scoping);
    let key = key_of(&shaped, scoping);
    let cached = LAST.with(|last| {
        last.borrow().as_ref().and_then(|entry| {
            let same_table = entry
                .table
                .upgrade()
                .is_some_and(|known| Arc::ptr_eq(&known, table));
            (same_table && entry.key == key).then(|| Arc::clone(&entry.found))
        })
    });
    if let Some(found) = cached {
        return found;
    }
    let found = Arc::new(compute(table, &shaped, scoping));
    LAST.with(|last| {
        *last.borrow_mut() = Some(Entry {
            table: Arc::downgrade(table),
            key,
            found: Arc::clone(&found),
        });
    });
    found
}
