use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

use crate::matcher::Factory;

const MODULE_PARAM: usize = 0;
const REQUIRE_PARAM: usize = 2;
const KEPT_MEMBERS: [&str; 2] = ["exports", "loaded"];
const ID_MEMBERS: [&str; 2] = ["id", "i"];
const ASYNC_MODULE: &str = "a";
const WASM_MODULE: &str = "v";
const WASM_EXPORTS: &str = "w";

struct ModuleUses<'s> {
    scoping: &'s Scoping,
    symbol: SymbolId,
    require: Option<SymbolId>,
    found: Option<String>,
}

impl ModuleUses<'_> {
    fn resolves_to(&self, expr: &Expression<'_>, symbol: Option<SymbolId>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        symbol.is_some()
            && id
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                == symbol
    }

    fn is_module(&self, expr: &Expression<'_>) -> bool {
        self.resolves_to(expr, Some(self.symbol))
    }

    fn helper(&self, expr: &Expression<'_>, name: &str) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if member.property.name == name && self.resolves_to(&member.object, self.require))
    }

    fn module_id(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::StaticMemberExpression(member)
            if ID_MEMBERS.contains(&member.property.name.as_str()) && self.is_module(&member.object))
    }

    fn handed_to_helper(&self, call: &CallExpression<'_>, at: usize) -> bool {
        let Some(argument) = call.arguments.get(at).and_then(Argument::as_expression) else {
            return false;
        };
        match at {
            0 => self.helper(&call.callee, ASYNC_MODULE) && self.is_module(argument),
            1 => self.helper(&call.callee, WASM_MODULE) && self.module_id(argument),
            _ => false,
        }
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

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.visit_expression(&it.callee);
        for (at, argument) in it.arguments.iter().enumerate() {
            if !self.handed_to_helper(it, at) {
                self.visit_argument(argument);
            }
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        if self.helper(&it.object, WASM_EXPORTS) && self.module_id(&it.expression) {
            return;
        }
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

fn param_symbol(factory: &Factory<'_, '_>, at: usize) -> Option<SymbolId> {
    match &factory.params.items.get(at)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn module_object_use(scoping: &Scoping, factory: &Factory<'_, '_>) -> Option<String> {
    let symbol = param_symbol(factory, MODULE_PARAM)?;
    let mut uses = ModuleUses {
        scoping,
        symbol,
        require: param_symbol(factory, REQUIRE_PARAM),
        found: None,
    };
    uses.visit_function_body(factory.body);
    uses.found
}
