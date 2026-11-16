use oxc_allocator::{Address, GetAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolId;

use super::{
    function_literal, member_key, same, str_const, transparent, unsafe_read, Walker, GLOBAL,
    KIND_DEPTH,
};
use crate::proof::GLOBAL_OBJECTS;
use oxc_syntax::symbol::SymbolFlags;

const FORWARDS: [&str; 2] = ["apply", "call"];

const TIMERS: [&str; 2] = ["setInterval", "setTimeout"];

#[derive(Default)]
pub(super) struct Fallback {
    uses: usize,
    function_write: bool,
    catch_write: bool,
}

enum Use {
    FunctionWrite,
    CatchWrite,
    Read,
    Other,
}

fn function_result(expr: &Expression<'_>) -> bool {
    let Expression::CallExpression(outer) = expr.without_parentheses() else {
        return false;
    };
    let Expression::CallExpression(inner) = outer.callee.without_parentheses() else {
        return false;
    };
    matches!(inner.callee.without_parentheses(), Expression::Identifier(id) if id.name == "Function")
}

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
        let used = self.fallback_use(at);
        if matches!(used, Use::Other) {
            return;
        }
        let fallback = self.fallbacks.entry(symbol).or_default();
        fallback.uses += 1;
        match used {
            Use::FunctionWrite => fallback.function_write = true,
            Use::CatchWrite => fallback.catch_write = true,
            Use::Read | Use::Other => {}
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

    fn fallback_use(&self, at: usize) -> Use {
        let Some((up, child)) = self.parent(at) else {
            return Use::Other;
        };
        match &self.stack[up] {
            AstKind::AssignmentExpression(assign)
                if assign.operator == AssignmentOperator::Assign =>
            {
                if function_result(&assign.right) || self.compiled_result(&assign.right) {
                    return Use::FunctionWrite;
                }
                let caught = self.stack[..up]
                    .iter()
                    .any(|kind| matches!(kind, AstKind::CatchClause(_)));
                if caught && self.global_name(&assign.right) {
                    return Use::CatchWrite;
                }
                Use::Other
            }
            AstKind::ReturnStatement(_) => Use::Read,
            AstKind::StaticMemberExpression(member) if member.object.address() == child => {
                self.safe_member_read(up, &member.property.name)
            }
            AstKind::ComputedMemberExpression(member) if member.object.address() == child => {
                match str_const(&member.expression, self.resolve, KIND_DEPTH) {
                    Some(key) => self.safe_member_read(up, &key),
                    None => Use::Other,
                }
            }
            _ => Use::Other,
        }
    }

    fn compiled_result(&self, expr: &Expression<'a>) -> bool {
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.symbol(callee) else {
            return false;
        };
        !self.scoping.symbol_is_mutated(symbol)
            && self.candidates.get(&symbol).is_some_and(|candidate| {
                matches!(candidate.init.without_parentheses(), Expression::CallExpression(compile)
                    if matches!(compile.callee.without_parentheses(), Expression::Identifier(id)
                        if id.name == "Function" && self.symbol(id).is_none()))
            })
    }

    fn timer_call(&self, at: usize, key: &str) -> bool {
        if !TIMERS.contains(&key) {
            return false;
        }
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        let AstKind::CallExpression(call) = &self.stack[up] else {
            return false;
        };
        same(&call.callee, child)
            && call
                .arguments
                .first()
                .and_then(Argument::as_expression)
                .is_some_and(|first| self.function_value(first))
    }

    fn function_value(&self, expr: &Expression<'a>) -> bool {
        match expr.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => true,
            Expression::Identifier(id) => self.symbol(id).is_some_and(|symbol| {
                !self.scoping.symbol_is_mutated(symbol)
                    && (self
                        .scoping
                        .symbol_flags(symbol)
                        .contains(SymbolFlags::Function)
                        || self
                            .candidates
                            .get(&symbol)
                            .is_some_and(|candidate| function_literal(candidate.init)))
            }),
            _ => false,
        }
    }

    fn safe_member_read(&self, at: usize, key: &str) -> Use {
        if self.timer_call(at, key) {
            return Use::Read;
        }
        if unsafe_read(key) {
            return Use::Other;
        }
        match self.parent(at) {
            Some((up, child)) => match &self.stack[up] {
                AstKind::CallExpression(call) if same(&call.callee, child) => Use::Other,
                AstKind::AssignmentExpression(assign)
                    if assign
                        .left
                        .as_member_expression()
                        .is_some_and(|m| super::member_address(m) == child) =>
                {
                    Use::Other
                }
                AstKind::UpdateExpression(_) => Use::Other,
                _ => Use::Read,
            },
            None => Use::Read,
        }
    }

    fn global_name(&self, expr: &Expression<'a>) -> bool {
        matches!(expr.without_parentheses(), Expression::Identifier(id)
            if self.symbol(id).is_none() && GLOBAL_OBJECTS.contains(&id.name.as_str()))
    }

    pub(super) fn global_fallback(&self, assign: &AssignmentExpression<'a>) -> bool {
        let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
            return false;
        };
        if !self.global_name(&assign.right) {
            return false;
        }
        let Some(symbol) = self.symbol(target) else {
            return false;
        };
        let local = self.scoping.symbol_scope_id(symbol) != self.scoping.root_scope_id();
        let total = self.scoping.get_resolved_reference_ids(symbol).len();
        local
            && self.fallbacks.get(&symbol).is_some_and(|fallback| {
                fallback.function_write && fallback.catch_write && fallback.uses == total
            })
    }

    pub(super) fn wrapper_receiver(&self, call: &CallExpression<'a>) -> Option<usize> {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return None;
        };
        let symbol = self.symbol(callee)?;
        if self.scoping.symbol_is_mutated(symbol) {
            return None;
        }
        let wrapper = returned_function(self.candidates.get(&symbol)?.init)?;
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
