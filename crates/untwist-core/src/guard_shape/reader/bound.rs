use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::controller::handed_out_targets;
use super::Printed;

const READER_PARAMS: usize = 2;

pub(super) fn handed_out_guarded(value: &Expression<'_>, printed: Printed<'_>) -> bool {
    let mut handed = HandedOut {
        printed,
        seen: false,
        broken: false,
    };
    handed.visit_expression(value);
    handed.seen && !handed.broken
}

struct HandedOut<'t> {
    printed: Printed<'t>,
    seen: bool,
    broken: bool,
}

impl<'a> VisitJs<'a> for HandedOut<'_> {
    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        if it.params.items.len() == READER_PARAMS {
            let targets = handed_out_targets(it);
            if !targets.is_empty() {
                self.seen = true;
                for target in targets {
                    self.broken |= !self.printed.on_one_line(target.span);
                }
                return;
            }
        }
        walk_js::walk_function(self, it, flags);
    }
}

#[derive(Default)]
struct Definitions {
    values: u32,
    assigned: usize,
    guarded: bool,
    poisoned: bool,
}

struct Targets<'s, 't> {
    scoping: &'s Scoping,
    printed: Printed<'t>,
    wanted: FxHashMap<SymbolId, Definitions>,
}

fn dynamic_scopes(scoping: &Scoping) -> bool {
    scoping.scope_descendants_from_root().any(|scope| {
        scoping
            .scope_flags(scope)
            .intersects(ScopeFlags::DirectEval | ScopeFlags::With)
    })
}

pub(super) fn settle_bound(
    program: &Program<'_>,
    scoping: &Scoping,
    bound: &[Option<ReferenceId>],
    code: &str,
) -> bool {
    if bound.is_empty() {
        return true;
    }
    if dynamic_scopes(scoping) {
        return false;
    }
    let mut wanted = FxHashMap::default();
    for reference in bound {
        let Some(symbol) = reference.and_then(|id| scoping.get_reference(id).symbol_id()) else {
            return false;
        };
        wanted.insert(symbol, Definitions::default());
    }
    let mut targets = Targets {
        scoping,
        printed: Printed { text: Some(code) },
        wanted,
    };
    targets.visit_program(program);
    targets.wanted.iter().all(|(symbol, found)| {
        let writes = scoping
            .get_resolved_references(*symbol)
            .filter(|reference| reference.is_write())
            .count();
        !found.poisoned && found.values == 1 && found.guarded && writes == found.assigned
    })
}

impl Targets<'_, '_> {
    fn define(&mut self, symbol: Option<SymbolId>, value: &Expression<'_>, assigned: bool) {
        let printed = self.printed;
        let Some(found) = symbol.and_then(|symbol| self.wanted.get_mut(&symbol)) else {
            return;
        };
        found.values += 1;
        found.assigned += usize::from(assigned);
        found.guarded = handed_out_guarded(value, printed);
    }
}

impl<'a> VisitJs<'a> for Targets<'_, '_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        let BindingPattern::BindingIdentifier(id) = &it.id else {
            walk_js::walk_variable_declarator(self, it);
            return;
        };
        if let Some(init) = &it.init {
            self.define(id.symbol_id.get(), init, false);
            self.visit_expression(init);
        }
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if it.operator == AssignmentOperator::Assign {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = &it.left {
                let symbol = id
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
                self.define(symbol, &it.right, true);
            }
        }
        walk_js::walk_assignment_expression(self, it);
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        if let Some(found) = it
            .symbol_id
            .get()
            .and_then(|symbol| self.wanted.get_mut(&symbol))
        {
            found.poisoned = true;
        }
    }
}
