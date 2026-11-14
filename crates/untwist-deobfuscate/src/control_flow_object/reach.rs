use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;
use untwist_core::semantic::{has_module_syntax, observing_open, open_eval_sites, with_observes};

use crate::calls::agnostic::names_reachable_from;
use crate::notes;

use super::declared::DeclaredObject;

struct Reach {
    eval_sites: Vec<ScopeId>,
    with_scopes: Vec<ScopeId>,
    global: bool,
}

fn assumed_eval_sites(program: &Program<'_>, scoping: &Scoping) -> Vec<ScopeId> {
    if with_observes() || !scoping.root_unresolved_references().contains_key("eval") {
        return Vec::new();
    }
    open_eval_sites(program, scoping)
}

fn with_scopes(scoping: &Scoping) -> Vec<ScopeId> {
    scoping
        .scope_descendants_from_root()
        .filter(|scope| scoping.scope_flags(*scope).contains(ScopeFlags::With))
        .collect()
}

fn is_script_global(program: &Program<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
        && !program.source_type.is_module()
        && !has_module_syntax(program)
}

fn globals_reached(
    program: &Program<'_>,
    scoping: &Scoping,
    candidates: &[DeclaredObject],
) -> bool {
    let mut names: Vec<&str> = candidates
        .iter()
        .filter(|candidate| is_script_global(program, scoping, candidate.symbol))
        .map(|candidate| scoping.symbol_name(candidate.symbol))
        .collect();
    if names.is_empty() {
        return false;
    }
    names.sort_unstable();
    names.dedup();
    observing_open(|| names_reachable_from(program, scoping, &names))
}

impl Reach {
    fn refusal(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        symbol: SymbolId,
    ) -> Option<&'static str> {
        let declared = scoping.symbol_scope_id(symbol);
        let under = |sites: &[ScopeId]| {
            sites.iter().any(|site| {
                scoping
                    .scope_ancestors(*site)
                    .any(|scope| scope == declared)
            })
        };
        if scoping.scope_flags(declared).contains_direct_eval() || under(&self.eval_sites) {
            return Some(notes::CONTROL_FLOW_DIRECT_EVAL);
        }
        if under(&self.with_scopes) {
            return Some(notes::CONTROL_FLOW_WITH);
        }
        (self.global && is_script_global(program, scoping, symbol))
            .then_some(notes::CONTROL_FLOW_GLOBAL_REACH)
    }
}

pub(super) fn refuse_dynamic_reach(
    candidates: &mut Vec<DeclaredObject>,
    program: &Program<'_>,
    cx: &mut Cx<'_, '_>,
) {
    if candidates.is_empty() {
        return;
    }
    let scoping = cx.scoping();
    let reach = Reach {
        eval_sites: assumed_eval_sites(program, scoping),
        with_scopes: with_scopes(scoping),
        global: globals_reached(program, scoping, candidates),
    };
    let mut refused: Vec<(Span, &'static str)> = Vec::new();
    candidates.retain(
        |candidate| match reach.refusal(program, scoping, candidate.symbol) {
            Some(note) => {
                refused.push((candidate.statement, note));
                false
            }
            None => true,
        },
    );
    for (span, note) in refused {
        cx.note(note, Some(span));
    }
}
