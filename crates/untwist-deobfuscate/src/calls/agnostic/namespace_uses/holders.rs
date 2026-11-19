use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::SymbolId;
use std::borrow::Cow;
use untwist_core::semantic::is_shared_global;

use super::keys::KeyProof;
use super::{key_kind, property_name, str_const, Resolve, Walker, GLOBAL, KIND_DEPTH, NAMESPACE};

const BUILTIN_CONSTRUCTORS: [&str; 16] = [
    "Object", "Function", "Array", "String", "Number", "Boolean", "Symbol", "BigInt", "RegExp",
    "Date", "Error", "Map", "Set", "WeakMap", "WeakSet", "Promise",
];

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn computed_kind(
        &self,
        object: &Expression<'a>,
        key: &Expression<'a>,
        base: u8,
    ) -> u8 {
        let text = self.key_text(key, KIND_DEPTH);
        let open = text.is_none();
        if open && self.numeric_key(key, KIND_DEPTH) {
            let global = base & GLOBAL != 0 || self.may_hold_global(object, KIND_DEPTH);
            return if global { GLOBAL } else { 0 };
        }
        if open {
            match self.key_proof(object, key) {
                KeyProof::Open => {}
                KeyProof::Symbol => return 0,
                KeyProof::Enumerated => {
                    let global = base & GLOBAL != 0 || self.may_hold_global(object, KIND_DEPTH);
                    return if global { GLOBAL } else { 0 };
                }
            }
        }
        let kind = key_kind(text.map(Cow::Owned), base);
        if kind == 0 && open && self.may_hold_global(object, KIND_DEPTH) {
            return NAMESPACE;
        }
        kind
    }

    pub(super) fn key_text(&self, expr: &Expression<'a>, depth: u8) -> Option<String> {
        let next = depth.checked_sub(1)?;
        let Expression::Identifier(id) = expr.without_parentheses() else {
            return str_const(expr, self.resolve, depth);
        };
        let symbol = self.symbol(id)?;
        self.fixed_init(symbol)
            .and_then(|init| self.key_text(init, next))
    }

    fn fixed_init(&self, symbol: SymbolId) -> Option<&'a Expression<'a>> {
        let candidate = self.candidates.get(&symbol)?;
        let fixed = !crate::calls::agnostic::rebound(self.scoping, symbol)
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && candidate
                .function
                .is_none_or(|function| !self.arguments_used.contains(&function));
        fixed.then_some(candidate.init)
    }

    pub(super) fn may_hold_global(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return true;
        };
        match expr.without_parentheses() {
            Expression::NullLiteral(_) => false,
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_) => !self.builtins_trusted,
            Expression::ClassExpression(class) => {
                class.heritage.is_some() || !self.builtins_trusted
            }
            Expression::ObjectExpression(object) => {
                !self.builtins_trusted || sets_own_chain(object, self.resolve)
            }
            Expression::Identifier(id) => match self.symbol(id) {
                None => true,
                Some(symbol) => self.symbol_may_hold_global(symbol, next),
            },
            Expression::ThisExpression(_) => true,
            Expression::CallExpression(call) => !self.array_source(call) || !self.builtins_trusted,
            Expression::StaticMemberExpression(member) => {
                !self.builtin_prototype(member) || !self.builtins_trusted
            }
            Expression::ConditionalExpression(c) => {
                self.may_hold_global(&c.consequent, next)
                    || self.may_hold_global(&c.alternate, next)
            }
            Expression::LogicalExpression(l) if l.operator == LogicalOperator::And => {
                self.may_hold_global(&l.right, next)
            }
            Expression::LogicalExpression(l) => {
                self.may_hold_global(&l.left, next) || self.may_hold_global(&l.right, next)
            }
            Expression::SequenceExpression(s) => s
                .expressions
                .last()
                .is_none_or(|last| self.may_hold_global(last, next)),
            _ => true,
        }
    }

    fn symbol_may_hold_global(&self, symbol: SymbolId, depth: u8) -> bool {
        if self.opaque.contains(&symbol) || self.shared_root(symbol) {
            return true;
        }
        let Some(init) = self.fixed_init(symbol) else {
            return true;
        };
        let Some(uses) = self.holder_uses.get(&symbol) else {
            return self.may_hold_global(init, depth);
        };
        if uses.escaped
            || uses
                .protos
                .iter()
                .any(|value| value.is_none_or(|value| self.may_hold_global(value, depth)))
        {
            return true;
        }
        let own_method = uses
            .called
            .iter()
            .any(|key| uses.written.contains(key) || init_defines(init, key, self.resolve));
        own_method || self.may_hold_global(init, depth)
    }

    pub(super) fn shared_root(&self, symbol: SymbolId) -> bool {
        self.script
            && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
            && is_shared_global(self.scoping.symbol_name(symbol))
    }

    pub(super) fn collect_array_source(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::Function(function) if function.is_declaration() => {
                let Some(id) = function.id.as_ref() else {
                    return;
                };
                let shaped = crate::string_array::is_string_array_function(function, &id.name);
                if let (true, Some(symbol)) = (shaped, id.symbol_id.get()) {
                    self.array_functions.insert(symbol, function.span);
                }
            }
            AstKind::AssignmentExpression(assign) => {
                let AssignmentTarget::AssignmentTargetIdentifier(target) = &assign.left else {
                    return;
                };
                if let Some(symbol) = self.symbol(target) {
                    self.assigned_at
                        .entry(symbol)
                        .or_default()
                        .push(assign.span);
                }
            }
            _ => {}
        }
    }

    fn array_source(&self, call: &CallExpression<'a>) -> bool {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return false;
        };
        let Some(symbol) = self.symbol(callee) else {
            return false;
        };
        let Some(span) = self.array_functions.get(&symbol) else {
            return false;
        };
        let writes = self
            .scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .filter(|reference| self.scoping.get_reference(**reference).is_write())
            .count();
        let inside = self.assigned_at.get(&symbol).map_or(&[][..], Vec::as_slice);
        call.arguments.is_empty()
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && inside.len() == writes
            && inside
                .iter()
                .all(|write| span.start <= write.start && write.end <= span.end)
    }

    fn builtin_prototype(&self, member: &StaticMemberExpression<'a>) -> bool {
        let Expression::Identifier(owner) = member.object.without_parentheses() else {
            return false;
        };
        member.property.name == "prototype"
            && self.symbol(owner).is_none()
            && BUILTIN_CONSTRUCTORS.contains(&owner.name.as_str())
            && !self.free_written(owner.name.as_str())
    }
}

fn sets_own_chain(object: &ObjectExpression<'_>, resolve: Resolve<'_>) -> bool {
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::ObjectProperty(p) => {
            p.kind != PropertyKind::Init
                || (!p.computed
                    && !p.shorthand
                    && !p.method
                    && property_name(&p.key, resolve).as_deref() == Some("__proto__"))
        }
        ObjectPropertyKind::SpreadProperty(_) => false,
    })
}

fn init_defines(init: &Expression<'_>, key: &str, resolve: Resolve<'_>) -> bool {
    let object = match init.without_parentheses() {
        Expression::ObjectExpression(object) => object,
        Expression::ArrayExpression(_) => {
            return key == "length" || key.bytes().all(|byte| byte.is_ascii_digit())
        }
        _ => return false,
    };
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::ObjectProperty(p) => {
            property_name(&p.key, resolve).is_none_or(|name| name == key)
        }
        ObjectPropertyKind::SpreadProperty(_) => true,
    })
}
