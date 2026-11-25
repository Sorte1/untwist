use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;

use super::GLOBAL_OBJECTS;

pub(super) type GlobalAliases = FxHashSet<SymbolId>;

pub(super) fn global_aliases(program: &Program<'_>, scoping: &Scoping) -> GlobalAliases {
    let named = scoping
        .root_unresolved_references()
        .keys()
        .any(|name| GLOBAL_OBJECTS.contains(&name.as_str()));
    if !named {
        return GlobalAliases::default();
    }
    let mut found = Aliases {
        scoping,
        aliases: GlobalAliases::default(),
    };
    found.visit_program(program);
    found.aliases
}

pub(super) fn is_global_choice(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::Identifier(id) => {
            GLOBAL_OBJECTS.contains(&id.name.as_str())
                && id
                    .reference_id
                    .get()
                    .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
        }
        Expression::ConditionalExpression(conditional) => {
            is_choice_arm(&conditional.consequent, scoping)
                && is_choice_arm(&conditional.alternate, scoping)
        }
        _ => false,
    }
}

fn is_choice_arm(expression: &Expression<'_>, scoping: &Scoping) -> bool {
    match expression.without_parentheses() {
        Expression::ObjectExpression(object) => object.properties.is_empty(),
        other => is_global_choice(other, scoping),
    }
}

struct Aliases<'s> {
    scoping: &'s Scoping,
    aliases: GlobalAliases,
}

impl<'a> VisitJs<'a> for Aliases<'_> {
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        if let (BindingPattern::BindingIdentifier(id), Some(init)) = (&it.id, &it.init) {
            let symbol = id.symbol_id.get().filter(|symbol| {
                self.scoping
                    .symbol_flags(*symbol)
                    .contains(SymbolFlags::ConstVariable)
            });
            if let Some(symbol) = symbol.filter(|_| is_global_choice(init, self.scoping)) {
                self.aliases.insert(symbol);
            }
        }
        walk_js::walk_variable_declarator(self, it);
    }
}
