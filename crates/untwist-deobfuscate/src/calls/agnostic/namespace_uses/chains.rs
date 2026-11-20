use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolFlags;
use std::borrow::Cow;
use untwist_core::ast_util::js_text::js_text;
use untwist_core::semantic::{is_foreign_global, observers, Observers};

use super::{member_address, same, str_const, Walker, KIND_DEPTH};

const PROTOTYPE_READERS: [&str; 5] = [
    "__proto__",
    "getPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "__lookupSetter__",
];

const BUILTIN_ROOTS: [&str; 22] = [
    "Object", "Function", "Array", "String", "Number", "Boolean", "Symbol", "BigInt", "RegExp",
    "Date", "Error", "Map", "Set", "WeakMap", "WeakSet", "Promise", "Reflect", "Iterator", "Math",
    "JSON", "Intl", "Atomics",
];

const CONSTRUCTORS: [&str; 18] = [
    "Object", "Function", "Array", "String", "Number", "Boolean", "Symbol", "BigInt", "RegExp",
    "Date", "Error", "Map", "Set", "WeakMap", "WeakSet", "Promise", "Reflect", "Iterator",
];

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_chain(&mut self, kind: AstKind<'a>) {
        if self.chains_open {
            return;
        }
        let obtained = match kind {
            AstKind::StaticMemberExpression(member) => {
                self.member_obtains(member.property.name.as_str(), &member.object)
            }
            AstKind::ComputedMemberExpression(member) => {
                match str_const(&member.expression, self.resolve, KIND_DEPTH) {
                    Some(key) => self.member_obtains(&key, &member.object),
                    None => self.constructor_name(&member.object),
                }
            }
            AstKind::StringLiteral(text) => {
                !self.member_key_position()
                    && !self.decoded_array_text(text.span)
                    && PROTOTYPE_READERS.contains(&js_text(text.value).as_ref())
            }
            AstKind::TemplateElement(element) => element
                .value
                .cooked
                .is_some_and(|cooked| PROTOTYPE_READERS.contains(&js_text(cooked).as_ref())),
            AstKind::IdentifierReference(id) => self.reference_obtains(id),
            AstKind::IdentifierName(name) => PROTOTYPE_READERS[1..].contains(&name.name.as_str()),
            AstKind::WithStatement(_) => true,
            _ => false,
        };
        self.chains_open |= obtained;
    }

    fn member_obtains(&self, key: &str, object: &Expression<'a>) -> bool {
        match key {
            "__proto__" => !self.member_written(),
            "prototype" => !self.user_constructor(object) && self.value_position(),
            _ => false,
        }
    }

    fn decoded_array_text(&self, span: oxc_span::Span) -> bool {
        self.resolve.is_some()
            && self
                .array_functions
                .values()
                .any(|function| function.start <= span.start && span.end <= function.end)
    }

    fn member_key_position(&self) -> bool {
        let at = self.stack.len() - 1;
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        matches!(self.stack[up], AstKind::ComputedMemberExpression(member) if same(&member.expression, child))
    }

    fn member_written(&self) -> bool {
        let at = self.stack.len() - 1;
        let Some((up, child)) = self.parent(at) else {
            return false;
        };
        matches!(self.stack[up], AstKind::AssignmentExpression(assign)
            if assign.left.as_member_expression().is_some_and(|target| member_address(target) == child))
    }

    fn value_position(&self) -> bool {
        let at = self.stack.len() - 1;
        let Some((up, child)) = self.parent(at) else {
            return true;
        };
        let outer_key = match self.stack[up] {
            AstKind::StaticMemberExpression(outer) if same(&outer.object, child) => {
                Some(Cow::Borrowed(outer.property.name.as_str()))
            }
            AstKind::ComputedMemberExpression(outer) if same(&outer.object, child) => {
                str_const(&outer.expression, self.resolve, KIND_DEPTH).map(Cow::Owned)
            }
            AstKind::AssignmentExpression(assign) => {
                return !assign
                    .left
                    .as_member_expression()
                    .is_some_and(|target| member_address(target) == child)
            }
            _ => return true,
        };
        match outer_key {
            Some(key) if key != "__proto__" && key != "constructor" => !self.method_use(up),
            _ => true,
        }
    }

    fn method_use(&self, member_at: usize) -> bool {
        let Some((up, child)) = self.parent(member_at) else {
            return false;
        };
        match self.stack[up] {
            AstKind::AssignmentExpression(assign) => {
                assign
                    .left
                    .as_member_expression()
                    .is_some_and(|target| member_address(target) == child)
                    && plain_method(&assign.right)
            }
            AstKind::StaticMemberExpression(outer)
                if same(&outer.object, child)
                    && matches!(outer.property.name.as_str(), "call" | "apply") =>
            {
                let called = self.parent(up).is_some_and(|(call_at, callee)| {
                    matches!(self.stack[call_at], AstKind::CallExpression(call) if same(&call.callee, callee))
                });
                called && self.builtins_intact()
            }
            _ => false,
        }
    }

    fn user_constructor(&self, object: &Expression<'a>) -> bool {
        match object.without_parentheses() {
            Expression::FunctionExpression(_) | Expression::ClassExpression(_) => true,
            Expression::Identifier(id) => self.symbol(id).is_some_and(|symbol| {
                self.scoping
                    .symbol_flags(symbol)
                    .intersects(SymbolFlags::Function | SymbolFlags::Class)
                    && !crate::calls::agnostic::rebound(self.scoping, symbol)
                    && !self.opaque.contains(&symbol)
            }),
            _ => false,
        }
    }

    fn constructor_name(&self, object: &Expression<'a>) -> bool {
        match object.without_parentheses() {
            Expression::Identifier(id) => {
                self.symbol(id).is_none() && BUILTIN_ROOTS.contains(&id.name.as_str())
            }
            other => other.as_member_expression().is_some_and(|member| {
                matches!(
                    super::member_key(member, self.resolve).as_deref(),
                    Some("prototype" | "__proto__")
                )
            }),
        }
    }

    fn reference_obtains(&self, id: &IdentifierReference<'a>) -> bool {
        if self.symbol(id).is_some() || !CONSTRUCTORS.contains(&id.name.as_str()) {
            return false;
        }
        let at = self.stack.len() - 1;
        let Some((up, child)) = self.parent(at) else {
            return true;
        };
        match self.stack[up] {
            AstKind::StaticMemberExpression(m) => !same(&m.object, child),
            AstKind::ComputedMemberExpression(m) => !same(&m.object, child),
            AstKind::CallExpression(call) => !same(&call.callee, child),
            AstKind::NewExpression(new) => !same(&new.callee, child),
            AstKind::BinaryExpression(binary) => {
                !(binary.operator == BinaryOperator::Instanceof && same(&binary.right, child))
            }
            AstKind::UnaryExpression(_) => false,
            _ => true,
        }
    }

    pub(super) fn settle_chains(&mut self, texts_run: bool) {
        self.chains_open |= CONSTRUCTORS.iter().any(|root| is_foreign_global(root));
        self.builtins_trusted = observers() == Observers::None || !(self.chains_open || texts_run);
        self.settle_proto_stores();
        if self.holders_open {
            self.builtins_trusted = false;
        }
    }

    pub(super) fn opens_chains(&self) -> bool {
        self.chains_open || self.holders_open
    }
}

fn plain_method(value: &Expression<'_>) -> bool {
    matches!(
        value.without_parentheses(),
        Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
    )
}
