use oxc_ast::ast::Program;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;
use untwist_core::pass::Cx;
use untwist_core::semantic::{has_module_syntax, with_observes};

use crate::calls::agnostic::names_reachable_from;
use crate::notes;

use super::declared::DeclaredObject;

struct Reach {
    with_scopes: Vec<ScopeId>,
    global: bool,
}

fn with_scopes(scoping: &Scoping) -> Vec<ScopeId> {
    if !with_observes() {
        return Vec::new();
    }
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
    names_reachable_from(program, scoping, &names)
}

impl Reach {
    fn refusal(
        &self,
        program: &Program<'_>,
        scoping: &Scoping,
        symbol: SymbolId,
    ) -> Option<&'static str> {
        let declared = scoping.symbol_scope_id(symbol);
        if scoping.scope_flags(declared).contains_direct_eval() {
            return Some(notes::CONTROL_FLOW_DIRECT_EVAL);
        }
        let under_with = self.with_scopes.iter().any(|with| {
            scoping
                .scope_ancestors(*with)
                .any(|scope| scope == declared)
        });
        if under_with {
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
