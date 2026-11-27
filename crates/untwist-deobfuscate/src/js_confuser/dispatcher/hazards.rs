use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::scope::{ScopeFlags, ScopeId};

struct Hazards<'s> {
    scoping: &'s Scoping,
    scope: ScopeId,
    depth: u32,
    found: bool,
}

impl<'a> VisitJs<'a> for Hazards<'_> {
    fn visit_this_expression(&mut self, _it: &ThisExpression) {
        self.found |= self.depth == 0;
    }

    fn visit_super(&mut self, _it: &Super) {
        self.found = true;
    }

    fn visit_new_target(&mut self, _it: &NewTarget) {
        self.found |= self.depth == 0;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == "arguments" && self.depth == 0 {
            self.found = true;
        }
        let symbol = it
            .reference_id
            .get()
            .and_then(|id| self.scoping.get_reference(id).symbol_id());
        match symbol {
            Some(symbol) => self.found |= self.scoping.symbol_scope_id(symbol) == self.scope,
            None => self.found |= it.reference_id.get().is_none(),
        }
    }

    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        let Some(symbol) = it.symbol_id.get() else {
            self.found = true;
            return;
        };
        self.found |= self.depth == 0 && !self.scoping.symbol_redeclarations(symbol).is_empty();
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        self.depth += 1;
        walk_js::walk_function(self, it, flags);
        self.depth -= 1;
    }
}

pub(super) fn hazardous(
    function: &Function<'_>,
    scoping: &Scoping,
    dispatcher_scope: ScopeId,
) -> bool {
    let Some(body) = function.body.as_ref() else {
        return true;
    };
    let mut hazards = Hazards {
        scoping,
        scope: dispatcher_scope,
        depth: 0,
        found: false,
    };
    hazards.visit_function_body(body);
    hazards.found
}
