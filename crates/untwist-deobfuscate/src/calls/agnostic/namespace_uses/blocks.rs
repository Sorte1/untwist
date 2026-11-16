use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_span::{GetSpan, Span};

use super::{Loose, Walker};
use crate::protection_shape::{
    is_decoded_interval_function, is_decoded_protection_callback, is_decoded_protection_function,
    Roles,
};

const CONSOLE_EFFECTS: [&str; 10] = [
    "console",
    "log",
    "warn",
    "info",
    "error",
    "exception",
    "table",
    "trace",
    "Function",
    "window",
];

const INTERVAL_EFFECTS: [&str; 3] = ["setInterval", "Function", "window"];

fn protection_outline(function: &Function<'_>) -> bool {
    function.params.items.len() == 1
        && function.body.as_ref().is_some_and(|body| {
            body.statements
                .iter()
                .any(|statement| matches!(statement, Statement::TryStatement(_)))
        })
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_protection(&mut self, kind: AstKind<'a>) {
        let (function, symbol) = match kind {
            AstKind::Function(function) => {
                let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) else {
                    return;
                };
                (function, symbol)
            }
            AstKind::VariableDeclarator(declarator) => {
                let (
                    BindingPattern::BindingIdentifier(id),
                    Some(Expression::FunctionExpression(function)),
                ) = (
                    &declarator.id,
                    declarator
                        .init
                        .as_ref()
                        .map(Expression::without_parentheses),
                )
                else {
                    return;
                };
                let Some(symbol) = id.symbol_id.get() else {
                    return;
                };
                (&**function, symbol)
            }
            _ => return,
        };
        if protection_outline(function)
            && is_decoded_protection_function(function, self.scoping, self.resolve)
        {
            self.protections.insert(symbol);
        }
    }

    pub(super) fn check_blocks(&mut self, call: &CallExpression<'a>) {
        if let Some(index) = self.wrapper_receiver(call) {
            let controller = match call.callee.without_parentheses() {
                Expression::Identifier(id) => self.symbol(id),
                _ => None,
            };
            let own = self.declared_by_parent();
            for (at, argument) in call.arguments.iter().enumerate() {
                let Some(callback) = argument.as_expression().filter(|_| at != index) else {
                    continue;
                };
                let roles = Roles {
                    own,
                    controller,
                    protections: Some(&self.protections),
                };
                if is_decoded_protection_callback(callback, roles, self.scoping, self.resolve) {
                    self.blocks.push((callback.span(), &CONSOLE_EFFECTS));
                }
            }
            return;
        }
        let Expression::FunctionExpression(function) = call.callee.without_parentheses() else {
            return;
        };
        let shell = call.arguments.is_empty()
            && function.params.items.is_empty()
            && is_decoded_interval_function(
                function,
                &self.protections,
                self.scoping,
                self.resolve,
            );
        if shell {
            self.blocks.push((call.span, &INTERVAL_EFFECTS));
        }
    }

    fn declared_by_parent(&self) -> Option<oxc_syntax::symbol::SymbolId> {
        let at = self.stack.len().checked_sub(2)?;
        match &self.stack[at] {
            AstKind::VariableDeclarator(declarator) => match &declarator.id {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            },
            _ => None,
        }
    }
}

impl Loose {
    pub(in crate::calls::agnostic) fn exempt(&self, span: Span) -> bool {
        self.blocks
            .iter()
            .any(|(block, _)| block.start <= span.start && span.end <= block.end)
    }
}
