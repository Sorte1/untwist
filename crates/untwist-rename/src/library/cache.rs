use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHasher};

use super::shape::{shape_program, Shaped};
use super::table::Table;
use super::LibraryNames;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    tokens: u32,
    shape: u64,
    units: u64,
    bindings: u64,
}

struct Entry {
    table: Weak<Table>,
    key: Key,
    declared: Vec<(SymbolId, Span)>,
    found: Arc<LibraryNames>,
}

static LAST: Mutex<Option<Entry>> = Mutex::new(None);
static COMPUTED: AtomicUsize = AtomicUsize::new(0);

pub fn computed() -> usize {
    COMPUTED.load(Ordering::Relaxed)
}

fn last() -> std::sync::MutexGuard<'static, Option<Entry>> {
    match LAST.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn key_of(shaped: &Shaped<'_, '_>, scoping: &Scoping) -> Key {
    let tokens = shaped.units.last().map_or(0, |program| program.len);
    let mut units = FxHasher::default();
    for unit in &shaped.units {
        (unit.span.start, unit.span.end, unit.start, unit.len).hash(&mut units);
        unit.binding
            .map(|symbol| {
                let span = scoping.symbol_span(symbol);
                (span.start, span.end)
            })
            .hash(&mut units);
    }
    Key {
        tokens,
        shape: shaped.prefix.range(0, tokens),
        units: units.finish(),
        bindings: shaped.bindings,
    }
}

fn symbols_of(found: &LibraryNames) -> Vec<SymbolId> {
    let mut symbols: Vec<SymbolId> = found
        .names
        .keys()
        .copied()
        .chain(found.conflicts.iter().map(|(symbol, _)| *symbol))
        .collect();
    symbols.sort_unstable();
    symbols.dedup();
    symbols
}

fn same_ids(declared: &[(SymbolId, Span)], scoping: &Scoping) -> bool {
    declared.iter().all(|(symbol, span)| {
        symbol.index() < scoping.symbols_len() && scoping.symbol_span(*symbol) == *span
    })
}

fn remapped(entry: &Entry, scoping: &Scoping) -> Option<LibraryNames> {
    let mut by_span: FxHashMap<Span, Option<SymbolId>> = FxHashMap::default();
    for symbol in scoping.symbol_ids() {
        by_span
            .entry(scoping.symbol_span(symbol))
            .and_modify(|known| *known = None)
            .or_insert(Some(symbol));
    }
    let mut moved: FxHashMap<SymbolId, SymbolId> = FxHashMap::default();
    for (old, span) in &entry.declared {
        moved.insert(*old, (*by_span.get(span)?)?);
    }
    let found = &entry.found;
    Some(LibraryNames {
        names: found
            .names
            .iter()
            .map(|(symbol, name)| (moved[symbol], name.clone()))
            .collect(),
        conflicts: found
            .conflicts
            .iter()
            .map(|(symbol, names)| (moved[symbol], names.clone()))
            .collect(),
        hits: found.hits.clone(),
    })
}

fn reuse(entry: &Entry, table: &Arc<Table>, key: Key, scoping: &Scoping) -> Option<Arc<LibraryNames>> {
    let same_table = entry
        .table
        .upgrade()
        .is_some_and(|known| Arc::ptr_eq(&known, table));
    if !same_table || entry.key != key {
        return None;
    }
    if same_ids(&entry.declared, scoping) {
        return Some(Arc::clone(&entry.found));
    }
    remapped(entry, scoping).map(Arc::new)
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
    let cached = last()
        .as_ref()
        .and_then(|entry| reuse(entry, table, key, scoping));
    if let Some(found) = cached {
        return found;
    }
    COMPUTED.fetch_add(1, Ordering::Relaxed);
    let found = Arc::new(compute(table, &shaped, scoping));
    let declared = symbols_of(&found)
        .into_iter()
        .map(|symbol| (symbol, scoping.symbol_span(symbol)))
        .collect();
    *last() = Some(Entry {
        table: Arc::downgrade(table),
        key,
        declared,
        found: Arc::clone(&found),
    });
    found
}
