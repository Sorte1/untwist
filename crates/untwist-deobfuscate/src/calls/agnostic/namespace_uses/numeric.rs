use oxc_ast::ast::*;
use oxc_ast::AstKind;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::Walker;

#[derive(Clone, Copy)]
pub(super) enum NumericWrite<'a> {
    Numeric,
    Value(&'a Expression<'a>),
    Unknown,
}

impl<'s, 'a> Walker<'s, 'a> {
    pub(super) fn collect_numeric(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::WithStatement(_) => self.dynamic_scope = true,
            AstKind::CallExpression(call) if call.callee.is_specific_id("eval") => {
                self.dynamic_scope = true;
            }
            AstKind::IdentifierReference(id) => self.collect_numeric_write(id),
            _ => {}
        }
    }

    fn collect_numeric_write(&mut self, id: &IdentifierReference<'a>) {
        let Some(symbol) = self.symbol(id) else {
            return;
        };
        let written = id
            .reference_id
            .get()
            .is_some_and(|reference| self.scoping.get_reference(reference).is_write());
        if !written {
            return;
        }
        let write = self.numeric_write(self.stack.len() - 1);
        self.numeric_writes.entry(symbol).or_default().push(write);
    }

    fn numeric_write(&self, at: usize) -> NumericWrite<'a> {
        let Some((up, _)) = self.parent(at) else {
            return NumericWrite::Unknown;
        };
        match self.stack[up] {
            AstKind::UpdateExpression(_) => NumericWrite::Numeric,
            AstKind::AssignmentExpression(assign) => match assign.operator {
                AssignmentOperator::Assign
                | AssignmentOperator::Addition
                | AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalOr
                | AssignmentOperator::LogicalNullish => NumericWrite::Value(&assign.right),
                _ => NumericWrite::Numeric,
            },
            _ => NumericWrite::Unknown,
        }
    }

    pub(super) fn numeric_key(&self, expr: &Expression<'a>, depth: u8) -> bool {
        let Some(next) = depth.checked_sub(1) else {
            return false;
        };
        match expr.without_parentheses() {
            Expression::NumericLiteral(_) | Expression::UpdateExpression(_) => true,
            Expression::UnaryExpression(unary) => matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus | UnaryOperator::BitwiseNot
            ),
            Expression::BinaryExpression(binary) => match binary.operator {
                BinaryOperator::Addition => {
                    self.numeric_key(&binary.left, next) && self.numeric_key(&binary.right, next)
                }
                operator => operator.is_arithmetic() || operator.is_bitwise(),
            },
            Expression::ConditionalExpression(c) => {
                self.numeric_key(&c.consequent, next) && self.numeric_key(&c.alternate, next)
            }
            Expression::SequenceExpression(s) => s
                .expressions
                .last()
                .is_some_and(|last| self.numeric_key(last, next)),
            Expression::Identifier(id) => self
                .symbol(id)
                .is_some_and(|symbol| self.numeric_symbol(symbol, next)),
            Expression::CallExpression(call) => self.fresh_symbol(call),
            _ => false,
        }
    }

    fn fresh_symbol(&self, call: &CallExpression<'a>) -> bool {
        let Expression::Identifier(callee) = call.callee.without_parentheses() else {
            return false;
        };
        let fresh = self.trust_symbol
            && callee.name == "Symbol"
            && self.symbol(callee).is_none()
            && !self.free_written("Symbol");
        if fresh {
            self.symbol_proof_used.set(true);
        }
        fresh
    }

    fn numeric_symbol(&self, symbol: SymbolId, depth: u8) -> bool {
        if self.dynamic_scope
            || self.opaque.contains(&symbol)
            || self.shared_root(symbol)
            || !self.scoping.symbol_redeclarations(symbol).is_empty()
        {
            return false;
        }
        let Some(candidate) = self.candidates.get(&symbol) else {
            return false;
        };
        let arguments = candidate
            .function
            .is_some_and(|function| self.arguments_used.contains(&function));
        if arguments || !self.numeric_key(candidate.init, depth) {
            return false;
        }
        self.numeric_writes.get(&symbol).is_none_or(|writes| {
            writes.iter().all(|write| match write {
                NumericWrite::Numeric => true,
                NumericWrite::Value(value) => self.numeric_key(value, depth),
                NumericWrite::Unknown => false,
            })
        })
    }
}
