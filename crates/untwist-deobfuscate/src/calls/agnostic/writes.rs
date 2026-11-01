use oxc_ast::ast::*;
use oxc_span::GetSpan;

use super::roles::*;
use super::sites::*;
use super::AKey;
use crate::proof::prototype_root_ref;

impl<'s, 'a> Sites<'s, 'a> {
    pub(super) fn target_fresh(&self, expr: &Expression<'a>, depth: u8) -> bool {
        if self.fresh.is_fresh(expr) || self.native_chain(expr) || self.native_fresh_call(expr) {
            return true;
        }
        match expr.without_parentheses() {
            Expression::Identifier(id) if depth > 0 => self
                .fresh
                .literal_init(id)
                .is_some_and(|init| self.target_fresh(init, depth - 1)),
            _ => false,
        }
    }

    pub(super) fn native_fresh_call(&self, expr: &Expression<'a>) -> bool {
        if !self.fresh_calls {
            return false;
        }
        let Expression::CallExpression(call) = expr.without_parentheses() else {
            return false;
        };
        let Some(member) = expr_member(&call.callee) else {
            return false;
        };
        let keys = self.member_keys(member);
        let fresh = !keys.is_empty()
            && keys.iter().all(
                |key| matches!(key, AKey::Named(name) if FRESH_NATIVES.contains(&name.as_ref())),
            );
        if fresh {
            self.fresh_used.set(true);
        }
        fresh
    }

    pub(super) fn native_chain(&self, expr: &Expression<'a>) -> bool {
        let mut methods = Vec::new();
        let mut current = expr.without_parentheses();
        while let Expression::CallExpression(call) = current {
            let Some(member) = expr_member(&call.callee) else {
                return false;
            };
            let keys = self.member_keys(member);
            let [AKey::Named(key)] = keys.as_slice() else {
                return false;
            };
            if !call.arguments.iter().all(plain_argument) {
                return false;
            }
            methods.push(key.clone());
            current = member.object().without_parentheses();
        }
        if methods.is_empty() || !literal_root(current) {
            return false;
        }
        self.assumed.borrow_mut().extend(methods);
        true
    }

    pub(super) fn is_target(&self, elem: Elem<'_, 'a>) -> bool {
        match elem {
            Elem::Hidden => true,
            Elem::Value(expr) => !self.target_fresh(expr, 4),
        }
    }

    pub(super) fn stable_root(&self, expr: &'a Expression<'a>) -> Option<&'a str> {
        match prototype_root_ref(self.unsubstituted(expr).without_parentheses()) {
            Some((id, _)) if self.fresh.symbol(id).is_none() => Some(id.name.as_str()),
            _ => None,
        }
    }

    pub(super) fn identity_write(
        &self,
        member: &MemberExpression<'a>,
        value: &'a Expression<'a>,
    ) -> bool {
        let Some(source) = self
            .unsubstituted(value)
            .without_parentheses()
            .as_member_expression()
        else {
            return false;
        };
        let (Some(target), Some(read)) = (
            self.stable_root(self.alloc_expr(member.object())),
            self.stable_root(source.object()),
        ) else {
            return false;
        };
        if target != read {
            return false;
        }
        let keys = self.member_keys(member);
        let [AKey::Named(key)] = keys.as_slice() else {
            return false;
        };
        key != "__proto__"
            && matches!(self.member_keys(source).as_slice(), [AKey::Named(other)] if other == key)
    }

    pub(super) fn write_site(&mut self, member: &MemberExpression<'a>, kind: &'static str) {
        self.note_rebind(member.object());
        let keys = self.member_keys(member);
        if self.target_fresh(member.object(), 4) {
            if !self.in_machinery(member.span()) {
                self.defined.extend(keys);
            }
            return;
        }
        let root = self.target_root(member.object());
        for key in keys {
            self.push_rooted(member.span(), key, kind, root);
        }
    }
}
