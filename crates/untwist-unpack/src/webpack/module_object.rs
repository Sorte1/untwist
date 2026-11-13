use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use crate::matcher::Factory;

const MODULE_PARAM: usize = 0;
const KEPT_MEMBERS: [&str; 2] = ["exports", "loaded"];

struct ModuleUses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    found: Option<String>,
}

impl ModuleUses<'_> {
    fn is_module(&self, expr: &Expression<'_>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        id.reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            == Some(self.symbol)
    }

    fn refuse(&mut self, what: impl Into<String>) {
        self.found.get_or_insert_with(|| what.into());
    }
}

impl<'a> VisitJs<'a> for ModuleUses<'_> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if !self.is_module(&it.object) {
            walk_js::walk_static_member_expression(self, it);
            return;
        }
        let name = it.property.name.as_str();
        if !KEPT_MEMBERS.contains(&name) {
            self.refuse(format!("module.{name}"));
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.is_module(&it.object) {
            self.refuse("a computed member of module");
        }
        walk_js::walk_computed_member_expression(self, it);
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let typeof_module = it.operator == UnaryOperator::Typeof && self.is_module(&it.argument);
        if !typeof_module {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let symbol = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id());
        if symbol == Some(self.symbol) {
            self.refuse("module as a value");
        }
    }
}

fn module_symbol(factory: &Factory<'_, '_>) -> Option<SymbolId> {
    match &factory.params.items.get(MODULE_PARAM)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn module_object_use(scoping: &Scoping, factory: &Factory<'_, '_>) -> Option<String> {
    let symbol = module_symbol(factory)?;
    let mut uses = ModuleUses {
        scoping,
        symbol,
        found: None,
    };
    uses.visit_function_body(factory.body);
    uses.found
}
