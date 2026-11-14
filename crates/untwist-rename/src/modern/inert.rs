use oxc_ast::ast::*;
use oxc_ast_visit::{walk, Visit};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use rustc_hash::FxHashSet;

pub struct Prefix<'s> {
    scoping: &'s Scoping,
    body: Span,
    read: FxHashSet<SymbolId>,
    calls_local_function: bool,
    pub budget: usize,
}

impl<'s> Prefix<'s> {
    pub fn new(scoping: &'s Scoping, body: Span, budget: usize) -> Self {
        Prefix {
            scoping,
            body,
            read: FxHashSet::default(),
            calls_local_function: false,
            budget,
        }
    }

    pub fn set_before(&self, symbol: SymbolId) -> bool {
        self.budget > 0 && !self.calls_local_function && !self.read.contains(&symbol)
    }

    pub fn statement(&mut self, it: &Statement<'_>) {
        if !matches!(it, Statement::FunctionDeclaration(_)) {
            self.visit_statement(it);
        }
    }

    pub fn declarator(&mut self, it: &VariableDeclarator<'_>) {
        self.visit_variable_declarator(it);
    }
}

impl<'a> Visit<'a> for Prefix<'_> {
    fn visit_expression(&mut self, it: &Expression<'a>) {
        if self.budget == 0 {
            return;
        }
        self.budget -= 1;
        walk::walk_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        else {
            return;
        };
        self.read.insert(symbol);
        let declared = self.scoping.symbol_span(symbol);
        let local = self.body.start <= declared.start && declared.end <= self.body.end;
        if local
            && self
                .scoping
                .symbol_flags(symbol)
                .intersects(SymbolFlags::Function | SymbolFlags::Class)
        {
            self.calls_local_function = true;
        }
    }
}
