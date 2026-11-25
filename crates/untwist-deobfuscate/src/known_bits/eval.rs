use oxc_ast::ast::{
    Argument, BinaryExpression, CallExpression, Expression, IdentifierReference, UnaryExpression,
};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::bits::Bits;
use super::value::Val;

const MAX_DEPTH: u32 = 64;

pub trait Leaves {
    fn leaf(&self, id: &IdentifierReference<'_>) -> Option<(SymbolId, Val)>;
    fn is_imul(&self, callee: &Expression<'_>) -> bool;
    fn lax(&self) -> bool;
}

pub fn abstract_value(expr: &Expression<'_>, leaves: &dyn Leaves) -> Option<Val> {
    Walker { leaves }.value(expr, 0)
}

struct Walker<'l> {
    leaves: &'l dyn Leaves,
}

impl Walker<'_> {
    fn value(&self, expr: &Expression<'_>, depth: u32) -> Option<Val> {
        if depth > MAX_DEPTH {
            return None;
        }
        let depth = depth + 1;
        match expr.without_parentheses() {
            Expression::NumericLiteral(literal) => Val::from_number(literal.value),
            Expression::Identifier(id) => self.leaves.leaf(id).map(|(_, value)| value),
            Expression::BinaryExpression(binary) => self.binary(binary, depth),
            Expression::UnaryExpression(unary) => self.unary(unary, depth),
            Expression::CallExpression(call) => self.call(call, depth),
            Expression::ConditionalExpression(conditional) if self.leaves.lax() => {
                let consequent = self.value(&conditional.consequent, depth)?;
                let alternate = self.value(&conditional.alternate, depth)?;
                (consequent.kind == alternate.kind).then(|| Val {
                    bits: consequent.bits.join(alternate.bits),
                    kind: consequent.kind,
                })
            }
            _ => None,
        }
    }

    fn coerced(&self, expr: &Expression<'_>, depth: u32) -> Option<Bits> {
        self.coerced_value(self.value(expr, depth))
    }

    fn coerced_value(&self, value: Option<Val>) -> Option<Bits> {
        match value {
            Some(value) => Some(value.bits),
            None => self.leaves.lax().then_some(Bits::TOP),
        }
    }

    fn binary(&self, binary: &BinaryExpression<'_>, depth: u32) -> Option<Val> {
        let operator = binary.operator;
        if matches!(
            operator,
            BinaryOperator::Addition | BinaryOperator::Subtraction
        ) {
            let left = self.value(&binary.left, depth)?;
            let right = self.value(&binary.right, depth)?;
            let bits = if operator == BinaryOperator::Addition {
                left.bits.add(right.bits)
            } else {
                left.bits.sub(right.bits)
            };
            return left.arithmetic(right, bits);
        }
        if !is_bitwise(operator) {
            return None;
        }
        let left_value = self.value(&binary.left, depth);
        let right_value = self.value(&binary.right, depth);
        let always_number = operator == BinaryOperator::ShiftRightZeroFill;
        if left_value.is_none() && right_value.is_none() && !always_number {
            return None;
        }
        let left = self.coerced_value(left_value)?;
        let right = self.coerced_value(right_value)?;
        let count = right.value();
        let bits = match (operator, count) {
            (BinaryOperator::BitwiseAnd, _) => left.and(right),
            (BinaryOperator::BitwiseOR, _) => left.or(right),
            (BinaryOperator::BitwiseXOR, _) => left.xor(right),
            (BinaryOperator::ShiftLeft, Some(count)) => left.shl(count),
            (BinaryOperator::ShiftRight, Some(count)) => left.sar(count),
            (BinaryOperator::ShiftRightZeroFill, Some(count)) => left.shr(count),
            _ => Bits::TOP,
        };
        Some(if operator == BinaryOperator::ShiftRightZeroFill {
            Val::uint32(bits)
        } else {
            Val::int32(bits)
        })
    }

    fn unary(&self, unary: &UnaryExpression<'_>, depth: u32) -> Option<Val> {
        match unary.operator {
            UnaryOperator::BitwiseNot => {
                Some(Val::int32(self.value(&unary.argument, depth)?.bits.not()))
            }
            UnaryOperator::UnaryNegation => self.value(&unary.argument, depth)?.negated(),
            UnaryOperator::UnaryPlus => self.value(&unary.argument, depth),
            _ => None,
        }
    }

    fn call(&self, call: &CallExpression<'_>, depth: u32) -> Option<Val> {
        let [left, right] = imul_arguments(call, self.leaves)?;
        let left = self.coerced(left, depth)?;
        let right = self.coerced(right, depth)?;
        Some(Val::int32(left.mul(right)))
    }
}

fn is_bitwise(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::ShiftLeft
            | BinaryOperator::ShiftRight
            | BinaryOperator::ShiftRightZeroFill
    )
}

fn imul_arguments<'c, 'a>(
    call: &'c CallExpression<'a>,
    leaves: &dyn Leaves,
) -> Option<[&'c Expression<'a>; 2]> {
    if call.optional || !leaves.is_imul(&call.callee) {
        return None;
    }
    match call.arguments.as_slice() {
        [Argument::SpreadElement(_), _] | [_, Argument::SpreadElement(_)] => None,
        [left, right] => Some([left.as_expression()?, right.as_expression()?]),
        _ => None,
    }
}

pub fn low_determined(
    expr: &Expression<'_>,
    leaves: &dyn Leaves,
    seen: &mut Vec<(SymbolId, Bits)>,
) -> bool {
    match expr.without_parentheses() {
        Expression::NumericLiteral(literal) => Val::from_number(literal.value).is_some(),
        Expression::Identifier(id) => match leaves.leaf(id) {
            Some((symbol, value)) => {
                if !seen.iter().any(|(known, _)| *known == symbol) {
                    seen.push((symbol, value.bits));
                }
                true
            }
            None => false,
        },
        Expression::BinaryExpression(binary) => match binary.operator {
            BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOR
            | BinaryOperator::BitwiseXOR
            | BinaryOperator::Addition
            | BinaryOperator::Subtraction => {
                low_determined(&binary.left, leaves, seen)
                    && low_determined(&binary.right, leaves, seen)
            }
            BinaryOperator::ShiftLeft => {
                matches!(
                    binary.right.without_parentheses(),
                    Expression::NumericLiteral(_)
                ) && low_determined(&binary.left, leaves, seen)
                    && low_determined(&binary.right, leaves, seen)
            }
            _ => false,
        },
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::BitwiseNot | UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && low_determined(&unary.argument, leaves, seen)
        }
        Expression::CallExpression(call) => {
            imul_arguments(call, leaves).is_some_and(|[left, right]| {
                low_determined(left, leaves, seen) && low_determined(right, leaves, seen)
            })
        }
        _ => false,
    }
}

pub fn concrete(
    expr: &Expression<'_>,
    leaves: &dyn Leaves,
    env: &dyn Fn(SymbolId) -> Option<u32>,
) -> Option<u32> {
    match expr.without_parentheses() {
        Expression::NumericLiteral(literal) => Val::from_number(literal.value)?.bits.value(),
        Expression::Identifier(id) => env(leaves.leaf(id)?.0),
        Expression::BinaryExpression(binary) => {
            let left = concrete(&binary.left, leaves, env)?;
            let right = concrete(&binary.right, leaves, env)?;
            match binary.operator {
                BinaryOperator::BitwiseAnd => Some(left & right),
                BinaryOperator::BitwiseOR => Some(left | right),
                BinaryOperator::BitwiseXOR => Some(left ^ right),
                BinaryOperator::Addition => Some(left.wrapping_add(right)),
                BinaryOperator::Subtraction => Some(left.wrapping_sub(right)),
                BinaryOperator::ShiftLeft => Some(left << (right & 31)),
                _ => None,
            }
        }
        Expression::UnaryExpression(unary) => {
            let argument = concrete(&unary.argument, leaves, env)?;
            match unary.operator {
                UnaryOperator::BitwiseNot => Some(!argument),
                UnaryOperator::UnaryNegation => Some(argument.wrapping_neg()),
                UnaryOperator::UnaryPlus => Some(argument),
                _ => None,
            }
        }
        Expression::CallExpression(call) => {
            let [left, right] = imul_arguments(call, leaves)?;
            let left = concrete(left, leaves, env)?;
            let right = concrete(right, leaves, env)?;
            Some(left.wrapping_mul(right))
        }
        _ => None,
    }
}
