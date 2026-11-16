use oxc_allocator::{Address, GetAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolId;

use super::{function_literal, member_key, same, transparent, Walker, GLOBAL};

const FORWARDS: [&str; 2] = ["apply", "call"];

fn returned_function<'k, 'a>(init: &'k Expression<'a>) -> Option<&'k Function<'a>> {
    match init.without_parentheses() {
        Expression::FunctionExpression(function) => Some(function),
        Expression::CallExpression(call) if call.arguments.is_empty() => {
            let body = match call.callee.without_parentheses() {
                Expression::FunctionExpression(outer) => outer.body.as_ref()?,
                Expression::ArrowFunctionExpression(outer) => match &outer.body {
                    ArrowFunctionBody::FunctionBody(body) => body,
                    _ => return None,
                },
                _ => return None,
            };
            body.statements.iter().find_map(|statement| {
                let Statement::ReturnStatement(ret) = statement else {
                    return None;
                };
                match ret.argument.as_ref()?.without_parentheses() {
                    Expression::FunctionExpression(function) => Some(&**function),
                    _ => None,
                }
            })
        }
        _ => None,
    }
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_reference(&mut self, id: &IdentifierReference<'a>) {
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        let at = self.stack.len() - 1;
        if self.forwarded_receiver(at) {
            *self.receivers.entry(symbol).or_default() += 1;
        }
    }

    fn parent(&self, at: usize) -> Option<(usize, Address)> {
        let mut child = self.stack[at].address();
        let mut up = at;
        while up > 0 {
            up -= 1;
            if !transparent(&self.stack[up]) {
                return Some((up, child));
            }
            child = self.stack[up].address();
        }
        None
    }

    fn forwarded_receiver(&self, at: usize) -> bool {
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        let AstKind::CallExpression(call) = &self.stack[up] else {
            return false;
        };
        let first = call.arguments.first().and_then(Argument::as_expression);
        first.is_some_and(|first| same(first, child))
            && call
                .callee
                .without_parentheses()
                .as_member_expression()
                .and_then(|member| member_key(member, self.resolve))
                .is_some_and(|key| FORWARDS.contains(&key.as_ref()))
    }

    pub(super) fn wrapper_receiver(&self, call: &CallExpression<'a>) -> Option<usize> {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return None;
        };
        let symbol = self.symbol(callee)?;
        let wrapper = returned_function(self.candidates.get(&symbol)?.init)?;
        if self.scoping.symbol_is_mutated(symbol) {
            return None;
        }
        let index = wrapper.params.items.iter().position(|param| {
            matches!(&param.pattern, BindingPattern::BindingIdentifier(id)
                if param.initializer.is_none() && id.symbol_id.get().is_some_and(|param| self.receiver_only(param)))
        })?;
        let others_literal = call
            .arguments
            .iter()
            .enumerate()
            .filter(|(at, _)| *at != index)
            .all(|(_, argument)| argument.as_expression().is_some_and(function_literal));
        (wrapper.params.rest.is_none() && others_literal).then_some(index)
    }

    fn receiver_only(&self, param: SymbolId) -> bool {
        let total = self.scoping.get_resolved_reference_ids(param).len();
        total > 0 && self.receivers.get(&param) == Some(&total)
    }

    pub(super) fn wrapped_by_global(&self, call: &CallExpression<'a>, function: Address) -> bool {
        let is_argument = call.arguments.iter().any(|argument| {
            argument
                .as_expression()
                .is_some_and(|expr| same(expr, function))
        });
        if !is_argument {
            return false;
        }
        self.wrapper_receiver(call)
            .and_then(|index| call.arguments.get(index))
            .and_then(Argument::as_expression)
            .is_some_and(|receiver| self.kind_here(receiver) & GLOBAL != 0)
    }
}
