pub mod notes;

use oxc_ast::ast::*;
use oxc_syntax::symbol::SymbolId;

use crate::matcher::Factory;

const REQUIRE_PARAM: usize = 1;

pub fn require_param(wrapper: Factory<'_, '_>) -> Option<SymbolId> {
    match &wrapper.params.items.get(REQUIRE_PARAM)?.pattern {
        BindingPattern::BindingIdentifier(id) if id.name == "require" => id.symbol_id.get(),
        _ => None,
    }
}
