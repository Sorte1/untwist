use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;
use untwist_core::semantic::program_is_strict;

use crate::bindings::{block_function_names, is_block_function};

#[derive(Clone, Copy)]
struct Hit {
    candidate: usize,
    var: ScopeId,
    sloppy: bool,
}

struct Strictness<'h> {
    strict: bool,
    hits: &'h mut [Hit],
}

impl Strictness<'_> {
    fn mark(&mut self, scope: Option<ScopeId>, strict: bool) {
        let Some(scope) = scope else {
            return;
        };
        for hit in self.hits.iter_mut().filter(|hit| hit.var == scope) {
            hit.sloppy = !strict;
        }
    }
}

impl<'a> VisitJs<'a> for Strictness<'_> {
    fn visit_program(&mut self, it: &Program<'a>) {
        self.strict = program_is_strict(it);
        self.mark(it.scope_id.get(), self.strict);
        walk_js::walk_program(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let outer = self.strict;
        self.strict = outer
            || it
                .body
                .as_ref()
                .is_some_and(|body| body.has_use_strict_directive());
        self.mark(it.scope_id.get(), self.strict);
        walk_js::walk_function(self, it, flags);
        self.strict = outer;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let outer = self.strict;
        self.strict = outer || it.body.has_use_strict_directive();
        self.mark(it.scope_id.get(), self.strict);
        walk_js::walk_arrow_function_expression(self, it);
        self.strict = outer;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let outer = self.strict;
        self.strict = true;
        walk_js::walk_class(self, it);
        self.strict = outer;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        self.mark(it.scope_id.get(), true);
        walk_js::walk_static_block(self, it);
    }
}

fn var_scope(scoping: &Scoping, scope: ScopeId) -> Option<ScopeId> {
    scoping
        .scope_ancestors(scope)
        .find(|ancestor| scoping.scope_flags(*ancestor).is_var())
}

fn within(scoping: &Scoping, scope: ScopeId, outer: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| ancestor == outer)
}

fn read_within(scoping: &Scoping, symbol: SymbolId, var: ScopeId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .any(|reference| within(scoping, scoping.get_reference(*reference).scope_id(), var))
}

fn intercepted(scoping: &Scoping, symbol: SymbolId, var: ScopeId) -> bool {
    let declared = scoping.symbol_scope_id(symbol);
    var_scope(scoping, declared) == Some(var)
        || (var != declared && within(scoping, var, declared) && read_within(scoping, symbol, var))
}

type AnySymbol<'f> = &'f dyn Fn(usize, &mut dyn FnMut(SymbolId) -> bool) -> bool;

fn sharing_candidates(scoping: &Scoping, count: usize, any_symbol: AnySymbol<'_>) -> Vec<usize> {
    let names = block_function_names(scoping);
    if names.is_empty() {
        return Vec::new();
    }
    (0..count)
        .filter(|candidate| {
            any_symbol(*candidate, &mut |symbol| {
                names.contains(scoping.symbol_name(symbol))
            })
        })
        .collect()
}

fn scope_hits(scoping: &Scoping, sharing: &[usize], any_symbol: AnySymbol<'_>) -> Vec<Hit> {
    let mut hits = Vec::new();
    for function in scoping.symbol_ids() {
        if !is_block_function(scoping, function) {
            continue;
        }
        let name = scoping.symbol_name(function);
        let Some(var) = var_scope(scoping, scoping.symbol_scope_id(function)) else {
            continue;
        };
        for &candidate in sharing {
            let reached = any_symbol(candidate, &mut |symbol| {
                scoping.symbol_name(symbol) == name && intercepted(scoping, symbol, var)
            });
            if reached {
                hits.push(Hit {
                    candidate,
                    var,
                    sloppy: true,
                });
            }
        }
    }
    hits
}

fn sloppy_hits(
    scoping: &Scoping,
    program: &Program<'_>,
    count: usize,
    any_symbol: AnySymbol<'_>,
) -> Vec<Hit> {
    let sharing = sharing_candidates(scoping, count, any_symbol);
    if sharing.is_empty() {
        return Vec::new();
    }
    let mut hits = scope_hits(scoping, &sharing, any_symbol);
    if hits.is_empty() {
        return hits;
    }
    let mut walk = Strictness {
        strict: false,
        hits: &mut hits,
    };
    walk.visit_program(program);
    hits.retain(|hit| hit.sloppy);
    hits
}

pub fn refuse_annex_b_bindings<T, const N: usize>(
    candidates: &mut Vec<T>,
    site: impl Fn(&T) -> (Span, [SymbolId; N]),
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
) {
    if candidates.is_empty() {
        return;
    }
    let any_symbol = |candidate: usize, test: &mut dyn FnMut(SymbolId) -> bool| {
        candidates
            .get(candidate)
            .is_some_and(|entry| site(entry).1.into_iter().any(&mut *test))
    };
    let hits = sloppy_hits(cx.scoping(), program, candidates.len(), &any_symbol);
    if hits.is_empty() {
        return;
    }
    let mut refused = Vec::new();
    let mut index = 0;
    candidates.retain(|candidate| {
        let hit = hits.iter().any(|hit| hit.candidate == index);
        index += 1;
        if hit {
            refused.push(site(candidate).0);
        }
        !hit
    });
    for span in refused {
        cx.note(crate::notes::BLOCK_FUNCTION_NAME, Some(span));
    }
}
