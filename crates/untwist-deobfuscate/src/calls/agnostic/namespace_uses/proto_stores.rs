use oxc_ast::ast::*;
use oxc_ast::AstKind;
use untwist_core::semantic::builtins::{scan_with_unknown_keys_plain, Builtins, Reliance};

use super::{member_key, property_name, Walker, KIND_DEPTH};

pub(super) const OBJECT_METHODS: [&str; 4] = [
    "hasOwnProperty",
    "propertyIsEnumerable",
    "isPrototypeOf",
    "toString",
];

pub(super) const ARRAY_METHODS: [&str; 12] = [
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "splice",
    "concat",
    "join",
    "indexOf",
    "lastIndexOf",
    "includes",
    "at",
];

const ARRAY_AND_OBJECT_METHODS: [&str; 16] = [
    "hasOwnProperty",
    "propertyIsEnumerable",
    "isPrototypeOf",
    "toString",
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "splice",
    "concat",
    "join",
    "indexOf",
    "lastIndexOf",
    "includes",
    "at",
];

static RELIED_ON: Reliance = Reliance {
    roots: &[
        "Object", "Function", "Array", "String", "Number", "Boolean", "Symbol", "BigInt", "RegExp",
        "Date", "Error", "Map", "Set", "WeakMap", "WeakSet", "Promise", "Reflect",
    ],
    members: &[
        ("Object", true, &OBJECT_METHODS),
        ("Array", true, &ARRAY_AND_OBJECT_METHODS),
    ],
};

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn builtins_intact(&self) -> bool {
        *self.intact.get_or_init(|| {
            let module = untwist_core::semantic::has_module_syntax(self.program);
            scan_with_unknown_keys_plain(self.program, self.scoping, module, &RELIED_ON)
                == Builtins::Intact
        })
    }

    pub(super) fn collect_proto_store(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::ObjectProperty(property)
                if !property.computed && !property.shorthand && !property.method =>
            {
                if property_name(&property.key, self.resolve).as_deref() == Some("__proto__") {
                    self.proto_stores.push(Some(&property.value));
                }
            }
            AstKind::AssignmentExpression(assign) => {
                let Some(member) = assign.left.as_member_expression() else {
                    return;
                };
                let key = member_key(member, self.resolve);
                if matches!(key.as_deref(), Some("__proto__" | "prototype")) {
                    self.proto_stores.push(Some(&assign.right));
                }
            }
            AstKind::CallExpression(call) => self.collect_proto_call(call),
            _ => {}
        }
    }

    fn collect_proto_call(&mut self, call: &'a CallExpression<'a>) {
        let Some(member) = call.callee.without_parentheses().as_member_expression() else {
            return;
        };
        let at = match member_key(member, self.resolve).as_deref() {
            Some("setPrototypeOf") => 1,
            Some("create") => 0,
            _ => return,
        };
        let spread = call.arguments.iter().take(at + 1).any(Argument::is_spread);
        let value = call.arguments.get(at).and_then(Argument::as_expression);
        match value {
            Some(value) if !spread => self.proto_stores.push(Some(value)),
            None if !spread => {}
            _ => self.proto_stores.push(None),
        }
    }

    pub(super) fn settle_proto_stores(&mut self) {
        let stores = std::mem::take(&mut self.proto_stores);
        self.holders_open = stores
            .iter()
            .any(|value| value.is_none_or(|value| !self.fresh_prototype(value)));
        self.proto_stores = stores;
    }

    fn fresh_prototype(&self, value: &Expression<'a>) -> bool {
        match value.without_parentheses() {
            Expression::NullLiteral(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_) => true,
            Expression::Identifier(id) => match self.symbol(id) {
                None => id.name == "undefined" && !self.free_written("undefined"),
                Some(_) => !self.may_hold_global(value, KIND_DEPTH),
            },
            _ => false,
        }
    }
}
