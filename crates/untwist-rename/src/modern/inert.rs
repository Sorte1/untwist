use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolFlags;

struct Inert<'s> {
    scoping: &'s Scoping,
    body: Span,
    ok: bool,
}

impl<'a> Visit<'a> for Inert<'_> {
    fn visit_function(&mut self, _it: &Function<'a>, _flags: ScopeFlags) {
        self.ok = false;
    }

    fn visit_arrow_function_expression(&mut self, _it: &ArrowFunctionExpression<'a>) {
        self.ok = false;
    }

    fn visit_class(&mut self, _it: &Class<'a>) {
        self.ok = false;
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        else {
            return;
        };
        let declared = self.scoping.symbol_span(symbol);
        let local = self.body.start <= declared.start && declared.end <= self.body.end;
        if local
            && self
                .scoping
                .symbol_flags(symbol)
                .intersects(SymbolFlags::Function)
        {
            self.ok = false;
        }
    }

    fn visit_statement(&mut self, it: &Statement<'a>) {
        if self.ok {
            walk::walk_statement(self, it);
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.ok {
            walk::walk_expression(self, it);
        }
    }
}

pub fn statement(it: &Statement<'_>, body: Span, scoping: &Scoping) -> bool {
    if matches!(it, Statement::FunctionDeclaration(_)) {
        return true;
    }
    let mut check = Inert {
        scoping,
        body,
        ok: true,
    };
    check.visit_statement(it);
    check.ok
}

pub fn declarator(it: &VariableDeclarator<'_>, body: Span, scoping: &Scoping) -> bool {
    let mut check = Inert {
        scoping,
        body,
        ok: true,
    };
    check.visit_variable_declarator(it);
    check.ok
}
