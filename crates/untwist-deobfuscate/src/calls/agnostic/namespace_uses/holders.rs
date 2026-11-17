use oxc_ast::ast::*;
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use std::borrow::Cow;

use super::{key_kind, str_const, Walker, KIND_DEPTH, NAMESPACE};
use crate::proof::GLOBAL_OBJECTS;

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
}
