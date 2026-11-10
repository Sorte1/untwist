use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;
use untwist_core::pass::Cx;
use untwist_core::semantic::AnnexBWrites;

pub fn resolved_symbol(reference: &IdentifierReference<'_>, scoping: &Scoping) -> Option<SymbolId> {
    scoping
        .get_reference(reference.reference_id.get()?)
        .symbol_id()
}

pub fn is_block_function(scoping: &Scoping, symbol: SymbolId) -> bool {
    let flags = scoping.symbol_flags(symbol);
    if !flags.contains(SymbolFlags::Function) || flags.contains(SymbolFlags::FunctionExpression) {
        return false;
    }
    let scope = scoping.scope_flags(scoping.symbol_scope_id(symbol));
    !(scope.is_top() || scope.is_function())
}

pub fn block_function_names(scoping: &Scoping) -> FxHashSet<&str> {
    scoping
        .symbol_ids()
        .filter(|symbol| is_block_function(scoping, *symbol))
        .map(|symbol| scoping.symbol_name(symbol))
        .collect()
}

pub fn refuse_block_function_names<T, const N: usize>(
    candidates: &mut Vec<T>,
    site: impl Fn(&T) -> (Span, [SymbolId; N]),
    cx: &mut Cx<'_, '_>,
) {
    if candidates.is_empty() {
        return;
    }
    let scoping = cx.scoping();
    let names = block_function_names(scoping);
    if names.is_empty() {
        return;
    }
    let mut refused = Vec::new();
    candidates.retain(|candidate| {
        let (span, symbols) = site(candidate);
        let shared = symbols
            .iter()
            .any(|symbol| names.contains(scoping.symbol_name(*symbol)));
        if shared {
            refused.push(span);
        }
        !shared
    });
    drop(names);
    for span in refused {
        cx.note(crate::notes::BLOCK_FUNCTION_NAME, Some(span));
    }
}

pub fn refuse_annex_b_writes<T>(
    candidates: &mut Vec<T>,
    site: impl Fn(&T) -> (Span, SymbolId),
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
) {
    if candidates.is_empty() || crate::mutants::annex_b_writes_trusted() {
        return;
    }
    let writes = AnnexBWrites::of(program, cx.scoping());
    if writes.is_empty() {
        return;
    }
    let mut refused = Vec::new();
    candidates.retain(|candidate| {
        let (span, symbol) = site(candidate);
        let written = writes.may_write(symbol);
        if written {
            refused.push(span);
        }
        !written
    });
    for span in refused {
        cx.note(crate::notes::BLOCK_FUNCTION_NAME, Some(span));
    }
}

pub fn reference_symbol(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    let Expression::Identifier(reference) = expr else {
        return None;
    };
    resolved_symbol(reference, scoping)
}

struct Stray<'k> {
    tracked: &'k FxHashSet<SymbolId>,
    removed: &'k [Span],
    scoping: &'k Scoping,
    found: bool,
}

impl<'a> VisitJs<'a> for Stray<'_> {
    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.found || self.removed.contains(&it.span()) {
            return;
        }
        walk_js::walk_statement(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.found {
            return;
        }
        if resolved_symbol(it, self.scoping).is_some_and(|symbol| self.tracked.contains(&symbol)) {
            self.found = true;
            return;
        }
        walk_js::walk_identifier_reference(self, it);
    }
}

pub fn has_stray_reference(
    program: &Program<'_>,
    tracked: &FxHashSet<SymbolId>,
    removed: &[Span],
    scoping: &Scoping,
) -> bool {
    let mut stray = Stray {
        tracked,
        removed,
        scoping,
        found: false,
    };
    stray.visit_program(program);
    stray.found
}

pub fn unexposed_globals<T>(
    candidates: Vec<T>,
    symbol_of: impl Fn(&T) -> SymbolId,
    program: &Program<'_>,
    scoping: &Scoping,
) -> Vec<T> {
    let root = scoping.root_scope_id();
    let at_root = |candidate: &T| scoping.symbol_scope_id(symbol_of(candidate)) == root;
    if !candidates.iter().any(at_root) {
        return candidates;
    }
    let exposed = crate::calls::agnostic::exposed_script_globals(program, scoping);
    candidates
        .into_iter()
        .filter(|candidate| {
            !at_root(candidate)
                || exposed
                    .as_ref()
                    .is_some_and(|exposed| !exposed.contains(&symbol_of(candidate)))
        })
        .collect()
}
