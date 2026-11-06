use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::bootstrap::Resolve;
use super::instance::COLLECTIONS;
use super::settle::Written;
use crate::value_flow::{flow_rule_disabled, NATIVE_MAP_PENDING};

const PATCH_KEYS: [&str; 6] = ["Map", "WeakMap", "Set", "WeakSet", "get", "set"];

pub(super) const NATIVE_METHODS: [&str; 2] = ["get", "set"];

const SEALED_MEMBERS: [&str; 7] = ["get", "set", "has", "delete", "clear", "size", "add"];

const RETURNS_RECEIVER: [&str; 2] = ["set", "add"];

pub(super) fn constructor_rebound(scoping: &Scoping) -> bool {
    COLLECTIONS
        .iter()
        .any(|name| bare_global_written(scoping, name))
}

pub(super) fn bare_global_written(scoping: &Scoping, name: &str) -> bool {
    scoping
        .root_unresolved_references()
        .get(name)
        .is_some_and(|references| {
            references
                .iter()
                .any(|reference| scoping.get_reference(*reference).is_write())
        })
}

pub(super) fn may_be_patched(written: &Written<'_>) -> bool {
    if flow_rule_disabled(NATIVE_MAP_PENDING) {
        return PATCH_KEYS.iter().any(|key| written.may_be_spelled(key));
    }
    PATCH_KEYS.iter().any(|key| written.may_be(key))
}

enum MemberKey<'k> {
    Spelled(&'k str),
    Decoded(Span),
    Unknown,
}

#[derive(Clone, Copy)]
enum DecodedUse {
    Member,
    SetCall,
}

#[derive(Default)]
pub(super) struct MapUses {
    discarded: FxHashSet<Span>,
    unsealed: FxHashSet<SymbolId>,
    decoded: Vec<(SymbolId, Span, DecodedUse)>,
}

impl MapUses {
    pub(super) fn discard(&mut self, expr: &Expression<'_>) {
        match expr.without_parentheses() {
            Expression::CallExpression(call) => {
                self.discarded.insert(call.span);
            }
            Expression::SequenceExpression(sequence) => {
                for expr in &sequence.expressions {
                    self.discard(expr);
                }
            }
            Expression::LogicalExpression(logical) => {
                self.discard(&logical.left);
                self.discard(&logical.right);
            }
            Expression::ConditionalExpression(conditional) => {
                self.discard(&conditional.test);
                self.discard(&conditional.consequent);
                self.discard(&conditional.alternate);
            }
            Expression::UnaryExpression(unary)
                if matches!(
                    unary.operator,
                    UnaryOperator::LogicalNot | UnaryOperator::Void
                ) =>
            {
                self.discard(&unary.argument);
            }
            _ => {}
        }
    }

    pub(super) fn call(&mut self, scoping: &Scoping, it: &CallExpression<'_>) {
        let Some(member) = it.callee.without_parentheses().as_member_expression() else {
            return;
        };
        if self.discarded.contains(&it.span) {
            return;
        }
        let Some(symbol) = receiver(scoping, member) else {
            return;
        };
        match key(member) {
            MemberKey::Spelled(name) if RETURNS_RECEIVER.contains(&name) => {
                self.unsealed.insert(symbol);
            }
            MemberKey::Decoded(span) => self.decoded.push((symbol, span, DecodedUse::SetCall)),
            MemberKey::Spelled(_) | MemberKey::Unknown => {}
        }
    }

    pub(super) fn member(&mut self, scoping: &Scoping, it: &MemberExpression<'_>) {
        let Some(symbol) = receiver(scoping, it) else {
            return;
        };
        match key(it) {
            MemberKey::Spelled(name) if SEALED_MEMBERS.contains(&name) => {}
            MemberKey::Decoded(span) => self.decoded.push((symbol, span, DecodedUse::Member)),
            MemberKey::Spelled(_) | MemberKey::Unknown => {
                self.unsealed.insert(symbol);
            }
        }
    }

    pub(super) fn unsealed(self, resolve: Resolve<'_>) -> FxHashSet<SymbolId> {
        let trust_decoded = flow_rule_disabled(NATIVE_MAP_PENDING);
        let mut unsealed = self.unsealed;
        for (symbol, span, used) in self.decoded {
            let name = resolve
                .and_then(|map| map.get(&span))
                .and_then(|text| text.as_string());
            let unseals = match (used, name) {
                (_, None) => matches!(used, DecodedUse::Member),
                (DecodedUse::Member, Some(name)) => {
                    !trust_decoded && !SEALED_MEMBERS.contains(&name.as_str())
                }
                (DecodedUse::SetCall, Some(name)) => {
                    !trust_decoded && RETURNS_RECEIVER.contains(&name.as_str())
                }
            };
            if unseals {
                unsealed.insert(symbol);
            }
        }
        unsealed
    }
}

fn key<'k>(member: &'k MemberExpression<'_>) -> MemberKey<'k> {
    if let Some(name) = member.static_property_name() {
        return MemberKey::Spelled(name);
    }
    let MemberExpression::ComputedMemberExpression(computed) = member else {
        return MemberKey::Unknown;
    };
    match computed.expression.without_parentheses() {
        Expression::CallExpression(call) => MemberKey::Decoded(call.span),
        _ => MemberKey::Unknown,
    }
}

fn receiver(scoping: &Scoping, member: &MemberExpression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(id) = member.object().without_parentheses() else {
        return None;
    };
    let reference = id.reference_id.get()?;
    scoping.get_reference(reference).symbol_id()
}
