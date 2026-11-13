use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::collect::Facts;
use super::plan::{Source, Unit};

const PATH_READERS: [&str; 10] = [
    "internals/path",
    "internals/get-built-in",
    "internals/well-known-symbol-define",
    "internals/define-well-known-symbol",
    "internals/entry-virtual",
    "internals/async-from-sync-iterator",
    "internals/async-iterator-create-proxy",
    "internals/create-async-iterator-proxy",
    "internals/create-iterator-proxy",
    "internals/iterator-create-proxy",
];

fn names(unit: &Unit, list: &[&str]) -> bool {
    unit.modules
        .iter()
        .any(|module| list.contains(&module.as_str()))
}

pub fn looks_pure(units: &[Unit], facts: &Facts) -> bool {
    let by_binding: FxHashMap<SymbolId, &Unit> = units
        .iter()
        .filter_map(|unit| unit.binding.map(|binding| (binding, unit)))
        .collect();
    units
        .iter()
        .filter(|unit| unit.source != Source::Trivial)
        .any(|unit| {
            let reads_path = names(unit, &PATH_READERS);
            unit.outer
                .iter()
                .filter_map(|symbol| by_binding.get(symbol))
                .filter(|dep| dep.source == Source::Trivial)
                .any(|dep| reads_path || facts.trivial_true.contains(&dep.span))
        })
}
