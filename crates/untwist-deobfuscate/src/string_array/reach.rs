use oxc_syntax::symbol::SymbolFlags;

use super::*;
use crate::calls::agnostic::{global_reach, ReachWatch};

pub(super) struct Kept {
    pub(super) symbols: FxHashSet<SymbolId>,
    pub(super) declared: FxHashSet<SymbolId>,
    pub(super) notes: Vec<(&'static str, Span)>,
    pub(super) observed: Option<Span>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visible {
    Nested,
    Lexical,
    Property,
}

struct InputScope {
    globals: bool,
    dynamic: DynamicScopes,
}

impl InputScope {
    fn of(program: &Program<'_>, cx: &Cx<'_, '_>) -> Self {
        Self {
            globals: !cx.opts.nested_root && !untwist_core::semantic::has_module_syntax(program),
            dynamic: DynamicScopes::of(cx.scoping()),
        }
    }

    fn visible(&self, scoping: &Scoping, symbol: SymbolId) -> Visible {
        if !self.globals || scoping.symbol_scope_id(symbol) != scoping.root_scope_id() {
            return Visible::Nested;
        }
        let property = SymbolFlags::FunctionScopedVariable.union(SymbolFlags::Function);
        if scoping.symbol_flags(symbol).intersects(property) {
            Visible::Property
        } else {
            Visible::Lexical
        }
    }

    fn local_refusal(&self, scoping: &Scoping, symbol: SymbolId) -> Option<&'static str> {
        let scope = scoping.symbol_scope_id(symbol);
        let nested = self.visible(scoping, symbol) == Visible::Nested;
        if nested && scoping.scope_flags(scope).contains_direct_eval() {
            return Some(notes::SIMPLE_ARRAY_DIRECT_EVAL);
        }
        self.dynamic
            .sees_with(scoping, symbol)
            .then_some(notes::SIMPLE_ARRAY_WITH)
    }
}

pub(super) fn reached(
    program: &Program<'_>,
    cx: &Cx<'_, '_>,
    candidates: &[(SymbolId, Span)],
) -> Kept {
    let scope = InputScope::of(program, cx);
    let scoping = cx.scoping();
    let mut kept = Kept {
        symbols: FxHashSet::default(),
        declared: FxHashSet::default(),
        notes: Vec::new(),
        observed: None,
    };
    for (symbol, statement) in candidates {
        if let Some(note) = scope.local_refusal(scoping, *symbol) {
            kept.symbols.insert(*symbol);
            kept.notes.push((note, *statement));
        }
    }
    let globals: Vec<(SymbolId, Visible)> = candidates
        .iter()
        .filter(|(symbol, _)| !kept.symbols.contains(symbol))
        .map(|(symbol, _)| (*symbol, scope.visible(scoping, *symbol)))
        .filter(|(_, visible)| *visible != Visible::Nested)
        .collect();
    judge_globals(program, scoping, &globals, &mut kept);
    kept
}

fn judge_globals(
    program: &Program<'_>,
    scoping: &Scoping,
    globals: &[(SymbolId, Visible)],
    kept: &mut Kept,
) {
    if globals.is_empty() {
        return;
    }
    let name = |(symbol, _): &(SymbolId, Visible)| scoping.symbol_name(*symbol);
    let names: Vec<&str> = globals.iter().map(name).collect();
    let props: Vec<&str> = globals
        .iter()
        .filter(|(_, visible)| *visible == Visible::Property)
        .map(name)
        .collect();
    let watch = ReachWatch {
        names: &names,
        props: &props,
        fns: &[],
        observe: true,
    };
    let found = global_reach(program, scoping, (None, &[]), watch);
    if let Some(span) = found.reach {
        kept.symbols
            .extend(globals.iter().map(|(symbol, _)| *symbol));
        kept.notes.push((notes::SIMPLE_ARRAY_GLOBAL_REACH, span));
    } else if let Some(span) = found.observed.filter(|_| !props.is_empty()) {
        kept.declared.extend(
            globals
                .iter()
                .filter(|(_, visible)| *visible == Visible::Property)
                .map(|(symbol, _)| *symbol),
        );
        kept.observed = Some(span);
    }
}
