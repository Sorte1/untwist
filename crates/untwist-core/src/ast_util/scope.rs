use oxc_ast::ast::*;
use oxc_ast_visit::VisitJs;
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;

struct Finder<'s> {
    scoping: &'s Scoping,
    target: SymbolId,
    found: bool,
}

impl<'a> VisitJs<'a> for Finder<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if self.found {
            return;
        }
        let Some(reference_id) = it.reference_id.get() else {
            return;
        };
        if self.scoping.get_reference(reference_id).symbol_id() == Some(self.target) {
            self.found = true;
        }
    }
}

pub fn references_symbol(expr: &Expression<'_>, scoping: &Scoping, symbol: SymbolId) -> bool {
    let mut finder = Finder {
        scoping,
        target: symbol,
        found: false,
    };
    finder.visit_expression(expr);
    finder.found
}

pub fn binding_symbol(pattern: &BindingPattern<'_>) -> Option<SymbolId> {
    match pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    }
}
