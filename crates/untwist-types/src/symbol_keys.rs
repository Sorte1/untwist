use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};

const SYMBOL: &str = "Symbol";

#[derive(Default)]
struct SymbolKeys {
    found: bool,
}

fn is_symbol_member(key: &PropertyKey<'_>) -> bool {
    let Some(expression) = key.as_expression() else {
        return false;
    };
    match expression.without_parentheses() {
        Expression::StaticMemberExpression(member) => names_symbol(&member.object),
        Expression::ComputedMemberExpression(member) => names_symbol(&member.object),
        _ => false,
    }
}

fn names_symbol(object: &Expression<'_>) -> bool {
    matches!(object.without_parentheses(), Expression::Identifier(id) if id.name == SYMBOL)
}

impl<'a> Visit<'a> for SymbolKeys {
    fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
        self.found |= it.computed && is_symbol_member(&it.key);
        walk::walk_object_property(self, it);
    }

    fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
        self.found |= it.computed && is_symbol_member(&it.key);
        walk::walk_method_definition(self, it);
    }

    fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
        self.found |= it.computed && is_symbol_member(&it.key);
        walk::walk_property_definition(self, it);
    }

    fn visit_accessor_property(&mut self, it: &AccessorProperty<'a>) {
        self.found |= it.computed && is_symbol_member(&it.key);
        walk::walk_accessor_property(self, it);
    }
}

pub(crate) fn has_well_known_symbol_key(program: &Program<'_>) -> bool {
    let mut keys = SymbolKeys::default();
    keys.visit_program(program);
    keys.found
}
