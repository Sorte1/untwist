use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;

use crate::matcher::Factory;

const REQUIRE_PARAM: usize = 0;
const MODULE_PARAM: usize = 1;
const EXPORTS: &str = "exports";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Host {
    Require,
    Module,
}

impl Host {
    fn name(self) -> &'static str {
        match self {
            Self::Require => "require",
            Self::Module => "module",
        }
    }
}

struct Uses<'s> {
    scoping: &'s Scoping,
    require: Option<SymbolId>,
    module: Option<SymbolId>,
    require_rebound: bool,
    found: Option<String>,
}

impl Uses<'_> {
    fn host(&self, expr: &Expression<'_>) -> Option<Host> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        self.host_of(id)
    }

    fn host_of(&self, id: &IdentifierReference<'_>) -> Option<Host> {
        let symbol = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())?;
        if self.module == Some(symbol) {
            return Some(Host::Module);
        }
        (self.require == Some(symbol)).then_some(Host::Require)
    }

    fn found(&mut self, what: String) {
        self.found.get_or_insert(what);
    }
}

impl<'a> VisitJs<'a> for Uses<'_> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        match self.host(&it.object) {
            Some(Host::Module) if it.property.name == EXPORTS && !it.optional => {}
            Some(host) => self.found(format!("reads {}.{}", host.name(), it.property.name)),
            None => walk_js::walk_static_member_expression(self, it),
        }
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        let exports =
            matches!(&it.expression, Expression::StringLiteral(key) if key.value == EXPORTS);
        match self.host(&it.object) {
            Some(Host::Module) if exports && !it.optional => {}
            Some(host) => self.found(format!("reads a computed property of {}", host.name())),
            None => walk_js::walk_computed_member_expression(self, it),
        }
    }

    fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
        let typeof_host = it.operator == UnaryOperator::Typeof && self.host(&it.argument).is_some();
        if !typeof_host {
            walk_js::walk_unary_expression(self, it);
        }
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if self.host(&it.callee) == Some(Host::Require) && !it.optional {
            for argument in &it.arguments {
                self.visit_argument(argument);
            }
            return;
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        match self.host_of(it) {
            Some(Host::Module) => self.found("uses module as a value".to_string()),
            Some(Host::Require) if !self.require_rebound => {
                self.found("uses require other than by calling it".to_string())
            }
            Some(Host::Require) => {}
            None => {}
        }
    }
}

fn param(params: &FormalParameters<'_>, index: usize) -> Option<SymbolId> {
    match &params.items.get(index)?.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}

pub fn host_object_use<'k, 'a: 'k>(
    scoping: &Scoping,
    factories: impl IntoIterator<Item = (String, Factory<'k, 'a>)>,
) -> Option<(String, String)> {
    factories.into_iter().find_map(|(id, factory)| {
        let require = param(factory.params, REQUIRE_PARAM);
        let require_rebound = require.is_some_and(|symbol| {
            scoping.symbol_is_mutated(symbol) || !scoping.symbol_redeclarations(symbol).is_empty()
        });
        let mut uses = Uses {
            scoping,
            require,
            require_rebound,
            module: param(factory.params, MODULE_PARAM),
            found: None,
        };
        uses.visit_function_body(factory.body);
        uses.found.map(|what| (id, what))
    })
}
