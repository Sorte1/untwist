use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use std::borrow::Cow;

use super::{key_kind, str_const, Walker, KIND_DEPTH, NAMESPACE};
use crate::proof::GLOBAL_OBJECTS;

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
        let key = self.key_text(key, KIND_DEPTH);
        let open = key.is_none();
        let kind = key_kind(key.map(Cow::Owned), base);
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
        let fixed = !self.scoping.symbol_is_mutated(symbol)
            && self.scoping.symbol_redeclarations(symbol).is_empty()
            && candidate
                .function
                .is_none_or(|function| !self.arguments_used.contains(&function));
        fixed.then_some(candidate.init)
    }

    fn may_hold_global(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return true;
        };
        match expr.without_parentheses() {
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::UpdateExpression(_) => false,
            Expression::Identifier(id) => match self.symbol(id) {
                None => GLOBAL_OBJECTS.contains(&id.name.as_str()),
                Some(symbol) => self.symbol_may_hold_global(symbol, next),
            },
            Expression::ThisExpression(_) => self.this_open(),
            Expression::CallExpression(call) => !self.array_source(call),
            Expression::StaticMemberExpression(member) => !self.builtin_prototype(member),
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
        let fixed_callable = self
            .scoping
            .symbol_flags(symbol)
            .intersects(SymbolFlags::Function | SymbolFlags::Class)
            && !self.scoping.symbol_is_mutated(symbol)
            && self.scoping.symbol_redeclarations(symbol).is_empty();
        if fixed_callable {
            return false;
        }
        self.fixed_init(symbol)
            .is_none_or(|init| self.may_hold_global(init, depth))
    }

    fn this_open(&self) -> bool {
        self.this_open.last().copied().unwrap_or(true)
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
                if let Some(symbol) = self.symbol(&**target) {
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
    }
}
