use oxc_ast::ast::*;
use oxc_syntax::symbol::SymbolId;

use super::keys::Keys;
use super::native_map::NATIVE_METHODS;
use super::settle::Written;
use super::sites::{expr_member, Sites};
use super::AKey;
use crate::value_flow::{flow_rule_disabled, INSTANCE_RECEIVERS};

pub(super) const COLLECTIONS: [&str; 4] = ["Map", "WeakMap", "Set", "WeakSet"];

const NAMESPACE_NAMED: [&str; 8] = [
    "set",
    "get",
    "defineProperty",
    "deleteProperty",
    "getOwnPropertyDescriptor",
    "setPrototypeOf",
    "defineProperties",
    "assign",
];

const QUIET_METHODS: [&str; 5] = ["set", "get", "has", "delete", "add"];

const PRIMITIVE_METHODS: [&str; 2] = ["has", "delete"];

const SIZE: &str = "size";

const NAMES: [&str; 12] = [
    "set",
    "get",
    "defineProperty",
    "deleteProperty",
    "getOwnPropertyDescriptor",
    "setPrototypeOf",
    "defineProperties",
    "assign",
    "has",
    "delete",
    "add",
    "size",
];

const PROTO_BIT: u32 = 1 << NAMES.len();

const AWAITED_NAMES: [&str; 6] = [
    "Promise",
    "all",
    "allSettled",
    "then",
    "constructor",
    "join",
];

const AWAITED_BITS: u32 = ((1 << AWAITED_NAMES.len()) - 1) << (NAMES.len() + 1);

const ALL_BITS: u32 = ((PROTO_BIT << 1) - 1) | AWAITED_BITS;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Unwritten(u32);

impl Unwritten {
    pub(super) fn all() -> Self {
        Self(ALL_BITS)
    }

    pub(super) fn after(self, written: &Written<'_>) -> Self {
        let mut bits = self.0;
        for (index, name) in NAMES.iter().enumerate() {
            if written.may_be(name) {
                bits &= !(1 << index);
            }
        }
        if written.may_be("__proto__") {
            bits &= !PROTO_BIT;
        }
        if AWAITED_NAMES.iter().any(|name| written.may_be(name)) {
            bits &= !AWAITED_BITS;
        }
        Self(bits)
    }

    fn holds(self, name: &str) -> bool {
        NAMES
            .iter()
            .position(|known| *known == name)
            .is_some_and(|index| self.0 & (1 << index) != 0)
    }

    fn proto(self) -> bool {
        self.0 & PROTO_BIT != 0
    }

    pub(super) fn awaited(self) -> bool {
        self.0 & AWAITED_BITS == AWAITED_BITS
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Instance {
    Native,
    User,
}

pub(super) fn class_sealed(class: &Class<'_>) -> bool {
    class.heritage.is_none()
        && class.decorators.is_empty()
        && class.body.body.iter().all(element_sealed)
}

fn element_sealed(element: &ClassElement<'_>) -> bool {
    match element {
        ClassElement::MethodDefinition(method) => {
            method.decorators.is_empty()
                && (method.r#static
                    || match method.kind {
                        MethodDefinitionKind::Constructor => true,
                        MethodDefinitionKind::Method => !method.computed,
                        MethodDefinitionKind::Get | MethodDefinitionKind::Set => {
                            plain_key(&method.key, method.computed)
                        }
                    })
        }
        ClassElement::PropertyDefinition(property) => {
            property.decorators.is_empty()
                && (property.r#static || plain_key(&property.key, property.computed))
        }
        ClassElement::AccessorProperty(property) => {
            property.decorators.is_empty()
                && (property.r#static || plain_key(&property.key, property.computed))
        }
        ClassElement::StaticBlock(_) => true,
        ClassElement::TSIndexSignature(_) => false,
    }
}

fn plain_key(key: &PropertyKey<'_>, computed: bool) -> bool {
    !computed
        && key
            .static_name()
            .and_then(|name| name.into_cow_str())
            .is_some_and(|name| !NAMES.contains(&name.as_ref()))
}

impl<'a> Sites<'_, 'a> {
    pub(super) fn call_keys(&self, member: &MemberExpression<'a>) -> Keys<'a> {
        let keys = self.member_keys(member);
        let Some(kind) = self.instance(member.object()) else {
            return keys;
        };
        if !keys.iter().any(|key| self.instance_method(kind, key)) {
            return keys;
        }
        Keys::Many(
            keys.into_iter()
                .filter(|key| !self.instance_method(kind, key))
                .collect(),
        )
    }

    pub(super) fn quiet_collection_call(&self, callee: &Expression<'a>) -> bool {
        let Some(member) = expr_member(callee) else {
            return false;
        };
        if !self.trust_receivers() && self.instance(member.object()) != Some(Instance::Native) {
            return false;
        }
        let keys = self.member_keys(member);
        !keys.is_empty()
            && keys.iter().all(|key| {
                matches!(key, AKey::Named(name)
                    if QUIET_METHODS.contains(&name.as_ref()) && self.method_unwritten(name))
            })
    }

    pub(super) fn collection_primitive(&self, expr: &Expression<'a>) -> bool {
        match expr {
            Expression::StaticMemberExpression(member) => {
                member.property.name == SIZE
                    && self.method_unwritten(SIZE)
                    && self.instance(&member.object) == Some(Instance::Native)
            }
            Expression::CallExpression(call) => {
                let Some(member) = expr_member(&call.callee) else {
                    return false;
                };
                let keys = self.member_keys(member);
                !keys.is_empty()
                    && keys.iter().all(|key| {
                        matches!(key, AKey::Named(name)
                            if PRIMITIVE_METHODS.contains(&name.as_ref())
                                && self.method_unwritten(name))
                    })
                    && self.instance(member.object()) == Some(Instance::Native)
            }
            _ => false,
        }
    }

    fn instance_method(&self, kind: Instance, key: &AKey<'_>) -> bool {
        let AKey::Named(name) = key else {
            return false;
        };
        let name = name.as_ref();
        if kind == Instance::Native && NATIVE_METHODS.contains(&name) {
            return true;
        }
        NAMESPACE_NAMED.contains(&name)
            && self.method_unwritten(name)
            && (kind == Instance::Native || self.unwritten.proto() || self.trust_receivers())
    }

    fn method_unwritten(&self, name: &str) -> bool {
        self.unwritten.holds(name) || self.trust_receivers()
    }

    fn trust_receivers(&self) -> bool {
        flow_rule_disabled(INSTANCE_RECEIVERS)
    }

    fn instance(&self, expr: &Expression<'a>) -> Option<Instance> {
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return None;
        };
        let symbol = self.fresh.symbol(id)?;
        if self.trust_receivers() {
            return Some(Instance::User);
        }
        let init = self.sole_init(symbol)?;
        let Expression::NewExpression(new) = init.without_parentheses() else {
            return None;
        };
        let Expression::Identifier(callee) = new.callee.without_parentheses() else {
            return None;
        };
        match self.fresh.symbol(callee) {
            None => (self.map_safe
                && new.arguments.is_empty()
                && COLLECTIONS.contains(&callee.name.as_str())
                && !self.fresh.bindings.unsealed_maps.contains(&symbol))
            .then_some(Instance::Native),
            Some(class) => self.sealed_class(class).then_some(Instance::User),
        }
    }

    pub(super) fn sole_init(&self, symbol: SymbolId) -> Option<&'a Expression<'a>> {
        let defs = self.fresh.bindings.defs.get(symbol)?;
        if defs.opaque || defs.mutated || defs.escapes || defs.fields_written || defs.callable {
            return None;
        }
        let [Some(init)] = defs.inits else {
            return None;
        };
        defs.writes.is_empty().then_some(*init)
    }

    fn sealed_class(&self, symbol: SymbolId) -> bool {
        if self.fresh.scoping.symbol_is_mutated(symbol) {
            return false;
        }
        let Some(defs) = self.fresh.bindings.defs.get(symbol) else {
            return false;
        };
        if defs.opaque || defs.mutated || !defs.writes.is_empty() {
            return false;
        }
        if defs.callable {
            return defs.constructs_fresh && defs.sealed_class && defs.inits.is_empty();
        }
        match defs.inits {
            [Some(Expression::ClassExpression(class))] => {
                super::bind::class_constructs_fresh(class) && class_sealed(class)
            }
            _ => false,
        }
    }
}
