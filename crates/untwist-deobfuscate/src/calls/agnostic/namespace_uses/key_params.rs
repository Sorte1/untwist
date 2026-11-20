use oxc_allocator::{Address, UnstableAddress};
use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::symbol::SymbolId;

use super::{same, Walker, KIND_DEPTH};

#[derive(Clone, Copy)]
pub(super) struct ParamSlot {
    binding: SymbolId,
    index: usize,
    function: Option<Address>,
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_param_slots(&mut self, kind: AstKind<'a>) {
        let (params, function) = match kind {
            AstKind::Function(function) => (&*function.params, Some(function.unstable_address())),
            AstKind::ArrowFunctionExpression(arrow) => (&*arrow.params, None),
            _ => return,
        };
        let Some(binding) = self.function_binding(kind) else {
            return;
        };
        if params.rest.is_some() {
            return;
        }
        for (index, param) in params.items.iter().enumerate() {
            let (BindingPattern::BindingIdentifier(id), None) =
                (&param.pattern, &param.initializer)
            else {
                continue;
            };
            if let Some(symbol) = id.symbol_id.get() {
                let slot = ParamSlot {
                    binding,
                    index,
                    function,
                };
                self.param_slots.insert(symbol, slot);
            }
        }
    }

    fn function_binding(&self, kind: AstKind<'a>) -> Option<SymbolId> {
        let at = self.stack.len() - 1;
        if let AstKind::Function(function) = kind {
            if function.is_declaration() {
                let exported = self.parent(at).is_some_and(|(up, _)| {
                    matches!(
                        self.stack[up],
                        AstKind::ExportNamedDeclaration(_) | AstKind::ExportDefaultDeclaration(_)
                    )
                });
                return function
                    .id
                    .as_ref()
                    .and_then(|id| id.symbol_id.get())
                    .filter(|_| !exported);
            }
            if super::self_named(function, self.scoping) {
                return None;
            }
        }
        let (up, child) = self.parent(at)?;
        let AstKind::VariableDeclarator(declarator) = self.stack[up] else {
            return None;
        };
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            return None;
        };
        declarator
            .init
            .as_ref()
            .filter(|init| same(init, child))
            .and_then(|_| id.symbol_id.get())
    }

    pub(super) fn collect_callee(&mut self, id: &IdentifierReference<'a>) {
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        let at = self.stack.len() - 1;
        let Some((up, child)) = self.parent(at) else {
            return;
        };
        if let AstKind::CallExpression(call) = self.stack[up] {
            if same(&call.callee, child) {
                self.callee_calls.entry(symbol).or_default().push(call);
            }
        }
    }

    pub(super) fn numeric_param(&self, key: &Expression<'a>) -> bool {
        let Expression::Identifier(id) = key.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.symbol(id) else {
            return false;
        };
        let Some(slot) = self.param_slots.get(&symbol).copied() else {
            return false;
        };
        let settled = !self.dynamic_scope
            && !self.opaque.contains(&symbol)
            && !self.opaque.contains(&slot.binding)
            && !self.shared_root(slot.binding)
            && !self.annex.may_write(slot.binding)
            && !crate::calls::agnostic::rebound(self.scoping, slot.binding)
            && self.scoping.symbol_redeclarations(slot.binding).is_empty()
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && slot
                .function
                .is_none_or(|function| !self.arguments_used.contains(&function));
        settled && self.writes_numeric(symbol) && self.called_with_numbers(slot)
    }

    fn called_with_numbers(&self, slot: ParamSlot) -> bool {
        let references = self.scoping.get_resolved_reference_ids(slot.binding).len();
        let Some(calls) = self.callee_calls.get(&slot.binding) else {
            return references == 0;
        };
        calls.len() == references
            && calls.iter().all(|call| {
                let spread = call
                    .arguments
                    .iter()
                    .take(slot.index + 1)
                    .any(Argument::is_spread);
                let argument = call
                    .arguments
                    .get(slot.index)
                    .and_then(Argument::as_expression);
                !spread && argument.is_some_and(|argument| self.numeric_key(argument, KIND_DEPTH))
            })
    }
}
