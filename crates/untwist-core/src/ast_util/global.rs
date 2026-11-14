use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_str::Ident;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::{ScopeFlags, ScopeId};
use oxc_syntax::symbol::SymbolId;

use crate::semantic::is_foreign_global;

fn program_has_direct_eval(scoping: &Scoping) -> bool {
    scoping
        .scope_flags(scoping.root_scope_id())
        .contains_direct_eval()
}

fn inside_with(scoping: &Scoping, scope: ScopeId) -> bool {
    scoping
        .scope_ancestors(scope)
        .any(|ancestor| scoping.scope_flags(ancestor).contains(ScopeFlags::With))
}

fn object_environment_may_bind(scoping: &Scoping, scope: ScopeId) -> bool {
    program_has_direct_eval(scoping) || inside_with(scoping, scope)
}

pub fn is_global_name_at(scoping: &Scoping, scope: ScopeId, name: &str) -> bool {
    scoping.find_binding(scope, Ident::from(name)).is_none()
        && !object_environment_may_bind(scoping, scope)
        && !is_foreign_global(name)
}

pub fn global_is_written(scoping: &Scoping, name: &str) -> bool {
    is_foreign_global(name)
        || scoping
            .root_unresolved_references()
            .iter()
            .filter(|(unresolved, _)| unresolved.as_str() == name)
            .any(|(_, ids)| ids.iter().any(|id| scoping.get_reference(*id).is_write()))
}

pub fn program_rebinds_global(scoping: &Scoping, name: &str) -> bool {
    global_is_written(scoping, name) || scoping.symbol_names().any(|declared| declared == name)
}

pub fn is_global_reference_id(scoping: &Scoping, reference: Option<ReferenceId>) -> bool {
    let Some(reference) = reference else {
        return false;
    };
    let reference = scoping.get_reference(reference);
    reference.symbol_id().is_none() && !object_environment_may_bind(scoping, reference.scope_id())
}

pub fn is_global_reference(scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
    is_global_reference_id(scoping, id.reference_id.get()) && !is_foreign_global(id.name.as_str())
}

pub fn is_declared_reference(scoping: &Scoping, id: &IdentifierReference<'_>) -> bool {
    let Some(reference) = id.reference_id.get() else {
        return false;
    };
    let reference = scoping.get_reference(reference);
    reference.symbol_id().is_some() && !object_environment_may_bind(scoping, reference.scope_id())
}

pub fn is_global_undefined(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    matches!(
        expr,
        Expression::Identifier(id)
            if id.name.as_str() == "undefined" && is_global_reference(scoping, id)
    )
}

pub fn pure_value_reads_globals(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    match expr.without_parentheses() {
        Expression::Identifier(id) => is_global_reference(scoping, id),
        Expression::UnaryExpression(unary) => match &unary.argument {
            Expression::Identifier(id) => is_global_reference(scoping, id),
            _ => true,
        },
        _ => true,
    }
}

struct GlobalCheck<'s, F> {
    scoping: &'s Scoping,
    allowed: F,
    global: bool,
}

impl<'a, F: Fn(SymbolId) -> bool> VisitJs<'a> for GlobalCheck<'_, F> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if !self.global {
            return;
        }
        let resolved = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        self.global = match resolved {
            Some(symbol) => (self.allowed)(symbol),
            None => is_global_reference(self.scoping, it),
        };
    }
}

pub fn references_are_global_or(
    expr: &Expression<'_>,
    scoping: &Scoping,
    allowed: impl Fn(SymbolId) -> bool,
) -> bool {
    let mut check = GlobalCheck {
        scoping,
        allowed,
        global: true,
    };
    check.visit_expression(expr);
    check.global
}

pub fn references_are_global(expr: &Expression<'_>, scoping: &Scoping) -> bool {
    references_are_global_or(expr, scoping, |_| false)
}
