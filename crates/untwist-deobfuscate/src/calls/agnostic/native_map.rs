use oxc_ast::ast::*;
use oxc_ast_visit::{walk_js, VisitJs};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::operator::UnaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::keys::Keys;
use super::settle::Written;
use super::sites::Sites;
use super::AKey;

const CONSTRUCTORS: [&str; 2] = ["Map", "WeakMap"];

const PATCH_KEYS: [&str; 4] = ["Map", "WeakMap", "get", "set"];

const NATIVE_METHODS: [&str; 2] = ["get", "set"];

const SEALED_MEMBERS: [&str; 6] = ["get", "set", "has", "delete", "clear", "size"];

pub(super) fn constructor_rebound(scoping: &Scoping) -> bool {
    CONSTRUCTORS
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

fn native_method(key: &AKey<'_>) -> bool {
    matches!(key, AKey::Named(name) if NATIVE_METHODS.contains(&name.as_ref()))
}

pub(super) fn may_be_patched(written: &Written<'_>) -> bool {
    PATCH_KEYS.iter().any(|key| written.may_be(key))
}

pub(super) fn unsealed_maps(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<SymbolId> {
    let mut scan = MapUses {
        scoping,
        discarded: FxHashSet::default(),
        unsealed: FxHashSet::default(),
    };
    scan.visit_program(program);
    scan.unsealed
}

struct MapUses<'s> {
    scoping: &'s Scoping,
    discarded: FxHashSet<Span>,
    unsealed: FxHashSet<SymbolId>,
}

impl MapUses<'_> {
    fn discard(&mut self, expr: &Expression<'_>) {
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

    fn receiver(&self, member: &MemberExpression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = member.object().without_parentheses() else {
            return None;
        };
        let reference = id.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }
}

impl<'a> VisitJs<'a> for MapUses<'_> {
    fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
        self.discard(&it.expression);
        walk_js::walk_expression_statement(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.discard(&it.test);
        walk_js::walk_if_statement(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some(member) = it.callee.without_parentheses().as_member_expression() {
            let is_set = member.static_property_name() == Some("set");
            if let Some(symbol) = self.receiver(member).filter(|_| is_set) {
                if !self.discarded.contains(&it.span) {
                    self.unsealed.insert(symbol);
                }
            }
        }
        walk_js::walk_call_expression(self, it);
    }

    fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
        if let Some(symbol) = self.receiver(it) {
            let sealed = it
                .static_property_name()
                .is_some_and(|name| SEALED_MEMBERS.contains(&name));
            if !sealed {
                self.unsealed.insert(symbol);
            }
        }
        walk_js::walk_member_expression(self, it);
    }
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn call_keys(&self, member: &MemberExpression<'a>) -> Keys<'a> {
        let keys = self.member_keys(member);
        if !self.native_map(member.object()) {
            return keys;
        }
        Keys::Many(keys.into_iter().filter(|key| !native_method(key)).collect())
    }

    fn native_map(&self, expr: &Expression<'_>) -> bool {
        if !self.map_safe {
            return false;
        }
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.fresh.symbol(id) else {
            return false;
        };
        let bindings = self.fresh.bindings;
        if bindings.unsealed_maps.contains(&symbol) {
            return false;
        }
        let Some(defs) = bindings.defs.get(symbol) else {
            return false;
        };
        if defs.opaque || defs.mutated || defs.escapes || defs.fields_written || defs.callable {
            return false;
        }
        let [Some(init)] = defs.inits else {
            return false;
        };
        defs.writes.is_empty() && self.constructs_native_map(init)
    }

    fn constructs_native_map(&self, init: &Expression<'_>) -> bool {
        let Expression::NewExpression(new) = init.without_parentheses() else {
            return false;
        };
        let Expression::Identifier(callee) = new.callee.without_parentheses() else {
            return false;
        };
        new.arguments.is_empty()
            && CONSTRUCTORS.contains(&callee.name.as_str())
            && self.fresh.symbol(callee).is_none()
    }
}
