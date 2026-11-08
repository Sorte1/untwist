use oxc_ast::ast::*;

use super::sites::{expr_member, Sites};
use super::AKey;
use crate::value_flow::{flow_rule_disabled, AWAITED_ARRAYS};

const COMBINATORS: [&str; 2] = ["all", "allSettled"];

const STRING_METHODS: [&str; 1] = ["join"];

impl<'a> Sites<'_, 'a> {
    pub(super) fn awaited_array_primitive(&self, expr: &Expression<'a>) -> bool {
        let Expression::CallExpression(call) = expr else {
            return false;
        };
        let Some(member) = expr_member(&call.callee) else {
            return false;
        };
        if flow_rule_disabled(AWAITED_ARRAYS) {
            return true;
        }
        !call.optional
            && !member.optional()
            && self.named_one_of(member, &STRING_METHODS)
            && self.awaited_array(member.object())
    }

    fn named_one_of(&self, member: &MemberExpression<'a>, names: &[&str]) -> bool {
        matches!(self.member_keys(member).as_slice(), [AKey::Named(name)] if names.contains(&name.as_ref()))
    }

    fn awaited_array(&self, expr: &Expression<'a>) -> bool {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        let Some(init) = self
            .fresh
            .symbol(id)
            .and_then(|symbol| self.sole_init(symbol))
        else {
            return false;
        };
        let Expression::AwaitExpression(awaited) = init.without_parentheses() else {
            return false;
        };
        let Expression::CallExpression(call) = awaited.argument.without_parentheses() else {
            return false;
        };
        let Some(member) = expr_member(&call.callee) else {
            return false;
        };
        let Expression::Identifier(root) = member.object().without_parentheses() else {
            return false;
        };
        !call.optional
            && !member.optional()
            && root.name == "Promise"
            && self.fresh.symbol(root).is_none()
            && self.named_one_of(member, &COMBINATORS)
            && self.unwritten.awaited()
            && !self.fresh.bindings.scoped_lookup
            && !self.promise_assigned()
    }

    fn promise_assigned(&self) -> bool {
        let scoping = self.fresh.scoping;
        scoping
            .root_unresolved_references()
            .get("Promise")
            .is_some_and(|references| {
                references
                    .iter()
                    .any(|reference| scoping.get_reference(*reference).is_write())
            })
    }
}
