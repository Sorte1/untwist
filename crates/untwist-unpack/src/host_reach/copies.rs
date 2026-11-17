use oxc_allocator::{Address, GetAddress};
use oxc_ast::AstKind;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::*;

impl<'a> Reach<'_, 'a> {
    pub(super) fn getter(&mut self, at: usize, held: Held<'a>) -> bool {
        let (Held::Read(Some(key)), AstKind::ArrowFunctionExpression(arrow)) =
            (held, self.stack[at])
        else {
            return false;
        };
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        match self.stack[up] {
            AstKind::ObjectProperty(_) => {
                let source = self.read_source(arrow);
                self.getter_target(up, child, key, source) || self.proxy_trap(up, arrow, key)
            }
            AstKind::StaticMemberExpression(member) if member.property.name == "bind" => {
                self.bound_getter(up, arrow, key)
            }
            _ => false,
        }
    }

    pub(super) fn bound_getter(
        &mut self,
        at: usize,
        arrow: &ArrowFunctionExpression<'a>,
        key: &Expression<'a>,
    ) -> bool {
        let first = arrow
            .params
            .items
            .first()
            .and_then(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            });
        let Expression::Identifier(read) = key.without_parentheses() else {
            return false;
        };
        if first.is_none() || self.symbol(read) != first || arrow.params.items.len() != 1 {
            return false;
        }
        let Some((up, member)) = self.parent(at) else {
            return false;
        };
        let AstKind::CallExpression(call) = self.stack[up] else {
            return false;
        };
        let bound = match call.arguments.as_slice() {
            [_, second] => second.as_expression(),
            _ => None,
        };
        let Some(bound) = bound.filter(|bound| matches!(bound, Expression::Identifier(_))) else {
            return false;
        };
        if !same(&call.callee, member) {
            return false;
        }
        let Some((property, child)) = self.parent(up) else {
            return false;
        };
        let source = self.read_source(arrow);
        matches!(self.stack[property], AstKind::ObjectProperty(_))
            && self.getter_target(property, child, bound, source)
    }

    pub(super) fn proxy_trap(
        &mut self,
        at: usize,
        arrow: &ArrowFunctionExpression<'a>,
        key: &Expression<'a>,
    ) -> bool {
        let second = arrow
            .params
            .items
            .get(1)
            .and_then(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            });
        let keyed = matches!(key.without_parentheses(), Expression::Identifier(read)
            if second.is_some() && self.symbol(read) == second);
        let AstKind::ObjectProperty(property) = self.stack[at] else {
            return false;
        };
        let named_get = !property.computed
            && property
                .key
                .static_name()
                .is_some_and(|name| name.as_str() == Some(GET));
        let Some((object_at, _)) = self.parent(at) else {
            return false;
        };
        let object = self.stack[object_at].address();
        let Some((up, _)) = self.parent(object_at) else {
            return false;
        };
        let AstKind::NewExpression(call) = self.stack[up] else {
            return false;
        };
        let proxy = matches!(call.callee.without_parentheses(), Expression::Identifier(id)
            if id.name == PROXY && self.symbol(id).is_none())
            && call
                .arguments
                .get(1)
                .and_then(Argument::as_expression)
                .is_some_and(|handler| same(handler, object));
        let first = arrow
            .params
            .items
            .first()
            .and_then(|param| match &param.pattern {
                BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
                _ => None,
            });
        let reads_target = first.is_some() && self.read_source(arrow) == first;
        let target = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .and_then(|target| self.identifier_symbol(target));
        keyed && named_get && proxy && reads_target && self.source_carries(target).is_none()
    }

    pub(super) fn getter_target(
        &mut self,
        at: usize,
        child: Address,
        key: &'a Expression<'a>,
        source: Option<SymbolId>,
    ) -> bool {
        let AstKind::ObjectProperty(property) = self.stack[at] else {
            return false;
        };
        let named_get = !property.computed
            && !property.method
            && property
                .key
                .static_name()
                .is_some_and(|name| name.as_str() == Some(GET));
        if !named_get || !same(&property.value, child) {
            return false;
        }
        let Some((object_at, _)) = self.parent(at) else {
            return false;
        };
        let object = self.stack[object_at].address();
        let Some((up, _)) = self.parent(object_at) else {
            return false;
        };
        let AstKind::CallExpression(call) = self.stack[up] else {
            return false;
        };
        let descriptor = call
            .arguments
            .get(2)
            .and_then(Argument::as_expression)
            .is_some_and(|argument| same(argument, object));
        let same_key = call
            .arguments
            .get(1)
            .and_then(Argument::as_expression)
            .is_some_and(|named| self.keys().same_key(key, named));
        let AstKind::ObjectExpression(object) = self.stack[object_at] else {
            return false;
        };
        descriptor
            && same_key
            && self.defines(call)
            && self.getter_carries(call, object, source, key)
    }

    pub(super) fn passed_on(
        &mut self,
        call: &CallExpression<'a>,
        child: Address,
        held: Held<'a>,
    ) -> bool {
        let Expression::Identifier(id) = call.callee.without_parentheses() else {
            return false;
        };
        let Some(function) = self.symbol(id) else {
            return false;
        };
        if !self.writes.stable(self.scoping, function) {
            return false;
        }
        let Some(index) = call.arguments.iter().position(|argument| {
            argument
                .as_expression()
                .is_some_and(|expr| same(expr, child))
        }) else {
            return false;
        };
        let param = self
            .writes
            .fn_params
            .get(&function)
            .and_then(|params| params.get(index).copied().flatten());
        match param {
            Some(param) => {
                self.taint(param, held);
                true
            }
            None => false,
        }
    }

    pub(super) fn member_use(
        &mut self,
        up: usize,
        held: Held<'a>,
        key: &str,
        can_hold: bool,
    ) -> bool {
        let field = held.descriptor() && DESCRIPTOR_FIELDS.contains(&key);
        match field {
            true => !self.written(up) && self.harmless(up, Held::Read(held.key()), can_hold),
            false => member_ok(held, key),
        }
    }

    pub(super) fn sink(
        &mut self,
        call: &'a CallExpression<'a>,
        child: Address,
        held: Held<'a>,
    ) -> bool {
        let free_callee = |names: &[&str], expr: &Expression<'_>| {
            matches!(expr.without_parentheses(), Expression::Identifier(id)
                if names.contains(&id.name.as_str()) && self.symbol(id).is_none())
        };
        if free_callee(&VALUE_SINKS, &call.callee) {
            return true;
        }
        let console = call
            .callee
            .without_parentheses()
            .as_member_expression()
            .is_some_and(|member| free_callee(&[CONSOLE], member.object()));
        if console {
            return true;
        }
        let defines = self.defines(call);
        let at_value = call
            .arguments
            .get(2)
            .and_then(Argument::as_expression)
            .is_some_and(|value| same(value, child));
        let same_key = match (
            held,
            call.arguments.get(1).and_then(Argument::as_expression),
        ) {
            (Held::Descriptor(Some(key)), Some(named)) => self.keys().same_key(key, named),
            _ => false,
        };
        defines && at_value && same_key && self.sink_carries(call, held)
    }

    pub(super) fn assigned(
        &mut self,
        assign: &AssignmentExpression<'a>,
        held: Held<'a>,
        can_hold: bool,
    ) -> bool {
        let replaces =
            assign.operator == AssignmentOperator::Assign || assign.operator.is_logical();
        if !replaces {
            return true;
        }
        match &assign.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => match self.symbol(id) {
                Some(symbol) if can_hold => {
                    self.taint(symbol, held);
                    true
                }
                _ => false,
            },
            AssignmentTarget::ComputedMemberExpression(member) => {
                let copy = can_hold
                    && matches!(held, Held::Read(_))
                    && held
                        .key()
                        .is_some_and(|key| self.keys().same_key(key, &member.expression));
                copy && self.carries(&member.object, held)
            }
            _ => false,
        }
    }

    pub(super) fn hold(&mut self, pattern: &BindingPattern<'a>, held: Held<'a>) -> bool {
        let BindingPattern::BindingIdentifier(id) = pattern else {
            return false;
        };
        let Some(symbol) = id.symbol_id.get() else {
            return false;
        };
        self.taint(symbol, held);
        true
    }

    pub(super) fn taint(&mut self, symbol: SymbolId, held: Held<'a>) {
        let merged = match self.tainted.get(&symbol) {
            None => held,
            Some(known) if known.same(held) => return,
            Some(known) => known.joined(held),
        };
        self.tainted.insert(symbol, merged);
        self.grew = true;
    }
}
