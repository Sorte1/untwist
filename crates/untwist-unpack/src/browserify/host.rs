use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;

use crate::matcher::Factory;
use crate::webpack::module_object::module_param_use;

const REQUIRE_PARAM: usize = 0;
const MODULE_PARAM: usize = 1;
const KEPT_MEMBERS: [&str; 1] = ["exports"];

struct Uses<'s> {
    scoping: &'s Scoping,
    require: SymbolId,
    require_rebound: bool,
    found: Option<String>,
}

impl Uses<'_> {
    fn is_require(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        self.resolves(id)
    }

    fn resolves(&self, id: &IdentifierReference<'_>) -> bool {
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            == Some(self.require)
    }

    fn found(&mut self, what: String) {
        self.found.get_or_insert(what);
    }
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if self.is_require(&it.object) {
            self.found(format!("reads require.{}", it.property.name));
            return;
        }
        walk_js::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_require(&it.object) {
            self.found("reads a computed property of require".to_string());
            return;
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let typeof_require = it.operator == UnaryOperator::Typeof && self.is_require(&it.argument);
        if !typeof_require {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.is_require(&it.callee) && !it.optional {
            for argument in &it.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.resolves(it) && !self.require_rebound {
            self.found("uses require other than by calling it".to_string());
        }
    }
}

fn param(params: &FormalParameters<'_>, index: usize) -> Option<SymbolId> {
    match &params.items.get(index)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

fn require_use(scoping: &Scoping, factory: &Factory<'_, '_>) -> Option<String> {
    let require = param(factory.params, REQUIRE_PARAM)?;
    let require_rebound =
        scoping.symbol_is_mutated(require) || !scoping.symbol_redeclarations(require).is_empty();
    let mut uses = Uses {
        scoping,
        require,
        require_rebound,
        found: None,
    };
    uses.visit_function_body(factory.body);
    uses.found
}

fn module_use(scoping: &Scoping, factory: &Factory<'_, '_>) -> Option<String> {
    let module = param(factory.params, MODULE_PARAM)?;
    module_param_use(scoping, factory.body, module, &KEPT_MEMBERS)
        .map(|what| format!("uses {what}"))
}

pub fn host_object_use<'k, 'a: 'k>(
    scoping: &Scoping,
    factories: impl IntoIterator<Item = (String, Factory<'k, 'a>)>,
) -> Option<(String, String)> {
    factories.into_iter().find_map(|(id, factory)| {
        module_use(scoping, &factory)
            .or_else(|| require_use(scoping, &factory))
            .map(|what| (id, what))
    })
}
