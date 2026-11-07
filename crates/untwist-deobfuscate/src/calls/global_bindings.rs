use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;

use super::agnostic::{global_reach, ReachWatch};
use crate::mutants::global_binding_writes_trusted;

#[derive(Default)]
pub(super) struct GlobalBindings {
    reached: Option<Span>,
    observed: bool,
    watched: FxHashSet<SymbolId>,
    properties: FxHashSet<SymbolId>,
}

impl GlobalBindings {
    pub(super) fn reached(&self, symbol: SymbolId) -> Option<Span> {
        self.reached.filter(|_| self.watched.contains(&symbol))
    }

    pub(super) fn observed(&self, symbol: SymbolId) -> bool {
        self.observed && self.properties.contains(&symbol)
    }
}

fn is_script_global(program: &Program<'_>, cx: &Cx<'_, '_>) -> bool {
    !cx.opts.nested_root && !untwist_core::semantic::has_module_syntax(program)
}

fn is_property(scoping: &Scoping, symbol: SymbolId) -> bool {
    let property = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
    scoping.symbol_flags(symbol).intersects(property)
}

pub(super) fn global_bindings(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    symbols: &[SymbolId],
) -> GlobalBindings {
    let scoping = cx.scoping();
    let root = scoping.root_scope_id();
    let watched: FxHashSet<SymbolId> = symbols
        .iter()
        .copied()
        .filter(|symbol| scoping.symbol_scope_id(*symbol) == root)
        .collect();
    if watched.is_empty() || global_binding_writes_trusted() || !is_script_global(program, cx) {
        return GlobalBindings::default();
    }
    let properties: FxHashSet<SymbolId> = watched
        .iter()
        .copied()
        .filter(|symbol| is_property(scoping, *symbol))
        .collect();
    let mut names: Vec<&str> = watched.iter().map(|s| scoping.symbol_name(*s)).collect();
    let mut props: Vec<&str> = properties.iter().map(|s| scoping.symbol_name(*s)).collect();
    names.sort_unstable();
    props.sort_unstable();
    let watch = ReachWatch {
        names: &names,
        props: &props,
        fns: &[],
        observe: !props.is_empty(),
    };
    let found = global_reach(program, scoping, (None, &[]), watch);
    GlobalBindings {
        reached: found.reach,
        observed: found.observed.is_some(),
        watched,
        properties,
    }
}
