use oxc_ast::ast::*;
use oxc_ast::AstType;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_syntax::symbol::SymbolId;

use super::{Emitter, Sink};
use crate::library::guard::{assigned_to, called_wrapper, inlined_wrapper, InlinedWrapper};

impl<'n, 'a: 'n> Emitter<'n, 'a> {
    pub(super) fn emit_body(&mut self, statements: &[Statement<'a>]) {
        match inlined_wrapper(statements, self.scoping) {
            Some(wrapper) => self.emit_inlined_wrapper(&wrapper),
            None => self.emit_list(statements),
        }
    }

    fn emit_inlined_wrapper(&mut self, wrapper: &InlinedWrapper<'_, 'a>) {
        self.open(AstType::IfStatement);
        self.visit_expression(&wrapper.guard.test);
        self.emit_statement(&wrapper.guard.consequent);
        self.close();
        let (last, init) = wrapper
            .tail
            .split_last()
            .expect("an inlined wrapper has a tail");
        self.emit_list(init);
        if self.emit_exports_assignment(last, wrapper.exports) {
            return;
        }
        self.emit_statement(last);
        self.open(AstType::ReturnStatement);
        self.open(AstType::IdentifierReference);
        self.push_symbol(wrapper.exports);
        self.close();
        self.close();
    }

    fn emit_exports_assignment(&mut self, last: &Statement<'a>, exports: SymbolId) -> bool {
        let Statement::ExpressionStatement(statement) = last else {
            return false;
        };
        let parts = match statement.expression.without_parentheses() {
            Expression::SequenceExpression(sequence) => sequence.expressions.as_slice(),
            _ => std::slice::from_ref(&statement.expression),
        };
        let (assignment, before) = parts.split_last().expect("a sequence has expressions");
        if assigned_to(assignment, self.scoping) != Some(exports) {
            return false;
        }
        for part in before {
            self.emit_expression_statement(part);
        }
        self.open(AstType::ReturnStatement);
        self.visit_expression(assignment);
        self.close();
        true
    }

    pub(super) fn emit_call(&mut self, call: &CallExpression<'a>) {
        let found = called_wrapper(call, self.scoping).filter(|_| !self.expand);
        let Some((flag, function)) = found else {
            walk_js::walk_call_expression(self, call);
            return;
        };
        let function: &'n Function<'a> = self.alloc(function);
        self.open(AstType::CallExpression);
        self.open(AstType::IdentifierReference);
        match &mut self.sink {
            Sink::Prefix(_) => {
                self.deferred.push(function);
                self.inlined_spans.push(function.span);
            }
            Sink::Exact(sink) => sink.inlined.push((flag, function.span)),
        }
        self.push_symbol(flag);
        self.close();
        self.close();
    }

    pub(super) fn emit_deferred(&mut self) {
        while let Some(function) = self.deferred.pop() {
            self.visit_function(function, oxc_syntax::scope::ScopeFlags::Function);
        }
    }
}
