use oxc_ast::ast::*;
use oxc_semantic::Scoping;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};
use oxc_syntax::symbol::SymbolId;

use super::super::resolution::Spans;

const XOR_STEP: f64 = 2654435769.0;
const PRINTABLE: i32 = 95;
const FIRST_PRINTABLE: i32 = 32;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Val {
    Num(f64),
    Bool(bool),
}

impl Val {
    fn truthy(self) -> bool {
        match self {
            Val::Num(n) => n != 0.0 && !n.is_nan(),
            Val::Bool(b) => b,
        }
    }

    pub(super) fn num(self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(n),
            Val::Bool(_) => None,
        }
    }
}

pub(super) struct StateEnv<'k> {
    pub(super) spans: &'k Spans,
    pub(super) states: SymbolId,
    pub(super) sum: SymbolId,
    pub(super) values: &'k [f64],
}

pub(super) fn symbol_at(expr: &Expression<'_>, spans: &Spans) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => spans.get(&id.span).copied(),
        _ => None,
    }
}

pub(super) fn symbol_of(expr: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match expr.without_parentheses() {
        Expression::Identifier(id) => scoping.get_reference(id.reference_id.get()?).symbol_id(),
        _ => None,
    }
}

fn compare(op: BinaryOperator, l: f64, r: f64) -> Option<bool> {
    Some(match op {
        BinaryOperator::Equality | BinaryOperator::StrictEquality => l == r,
        BinaryOperator::Inequality | BinaryOperator::StrictInequality => l != r,
        BinaryOperator::LessThan => l < r,
        BinaryOperator::GreaterThan => l > r,
        BinaryOperator::LessEqualThan => l <= r,
        BinaryOperator::GreaterEqualThan => l >= r,
        _ => return None,
    })
}

impl StateEnv<'_> {
    pub(super) fn is_states(&self, expr: &Expression<'_>) -> bool {
        symbol_at(expr, self.spans) == Some(self.states)
    }

    pub(super) fn sum_call(&self, expr: &Expression<'_>) -> bool {
        matches!(expr.without_parentheses(), Expression::CallExpression(call)
            if symbol_at(&call.callee, self.spans) == Some(self.sum)
                && matches!(call.arguments.as_slice(), [arg]
                    if arg.as_expression().is_some_and(|e| self.is_states(e))))
    }

    pub(super) fn index(&self, expr: &Expression<'_>) -> Option<usize> {
        let index = self.eval(expr)?.0.num()?;
        (index.fract() == 0.0 && index >= 0.0 && (index as usize) < self.values.len())
            .then_some(index as usize)
    }

    pub(super) fn eval(&self, expr: &Expression<'_>) -> Option<(Val, bool)> {
        match expr.without_parentheses() {
            Expression::NumericLiteral(n) => Some((Val::Num(n.value), false)),
            Expression::UnaryExpression(unary) => {
                let (value, touched) = self.eval(&unary.argument)?;
                match unary.operator {
                    UnaryOperator::UnaryNegation => Some((Val::Num(-value.num()?), touched)),
                    UnaryOperator::LogicalNot => Some((Val::Bool(!value.truthy()), touched)),
                    _ => None,
                }
            }
            Expression::ComputedMemberExpression(member) if self.is_states(&member.object) => {
                let at = self.index(&member.expression)?;
                Some((Val::Num(self.values[at]), true))
            }
            Expression::CallExpression(_) if self.sum_call(expr) => {
                Some((Val::Num(self.values.iter().sum()), true))
            }
            Expression::BinaryExpression(binary) => {
                let (l, lt) = self.eval(&binary.left)?;
                let (r, rt) = self.eval(&binary.right)?;
                let (l, r) = (l.num()?, r.num()?);
                let value = match binary.operator {
                    BinaryOperator::Addition => Val::Num(l + r),
                    BinaryOperator::Subtraction => Val::Num(l - r),
                    op => Val::Bool(compare(op, l, r)?),
                };
                Some((value, lt || rt))
            }
            Expression::LogicalExpression(logical) => {
                let (l, lt) = self.eval(&logical.left)?;
                let take_left = match logical.operator {
                    LogicalOperator::And => !l.truthy(),
                    LogicalOperator::Or => l.truthy(),
                    LogicalOperator::Coalesce => return None,
                };
                if take_left {
                    return Some((l, lt));
                }
                let (r, rt) = self.eval(&logical.right)?;
                Some((r, lt || rt))
            }
            _ => None,
        }
    }
}

fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let wrapped = value.trunc().rem_euclid(4294967296.0);
    (wrapped as u32) as i32
}

pub(super) fn xor_decode(strings: &[u16], key: f64, start: f64, length: f64) -> Option<String> {
    let whole = |v: f64| v.fract() == 0.0 && v >= 0.0;
    if !whole(start) || !whole(length) || key.fract() != 0.0 {
        return None;
    }
    let (start, length) = (start as usize, length as usize);
    let mut key = key;
    let mut out = String::with_capacity(length);
    for i in 0..length {
        let next = to_int32(key + XOR_STEP);
        key = f64::from(next);
        let mixed = next ^ (((next as u32) >> 13) as i32);
        let ks = (mixed % PRINTABLE + PRINTABLE) % PRINTABLE;
        let unit = i32::from(*strings.get(start.checked_add(i)?)?);
        let normalized = unit - FIRST_PRINTABLE;
        let shifted = ((normalized - ks) % PRINTABLE + PRINTABLE) % PRINTABLE;
        out.push(char::from_u32(
            u32::try_from(shifted + FIRST_PRINTABLE).ok()?,
        )?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(text: &str, key: i64) -> Vec<u16> {
        let mut key = key as f64;
        text.chars()
            .map(|c| {
                let next = to_int32(key + XOR_STEP);
                key = f64::from(next);
                let mixed = next ^ (((next as u32) >> 13) as i32);
                let ks = (mixed % PRINTABLE + PRINTABLE) % PRINTABLE;
                let normalized = c as i32 - FIRST_PRINTABLE;
                ((normalized + ks) % PRINTABLE + PRINTABLE) % PRINTABLE + FIRST_PRINTABLE
            })
            .map(|unit| unit as u16)
            .collect()
    }

    #[test]
    fn xor_decodes_what_the_position_cipher_encodes() {
        for key in [0, 183357, 999999, 410403] {
            let mut strings: Vec<u16> = "ab".encode_utf16().collect();
            strings.extend(encode("hello, world", key));
            assert_eq!(
                xor_decode(&strings, key as f64, 2.0, 12.0).as_deref(),
                Some("hello, world")
            );
        }
    }

    #[test]
    fn xor_refuses_reads_past_the_strings() {
        let strings: Vec<u16> = "abc".encode_utf16().collect();
        assert!(xor_decode(&strings, 1.0, 2.0, 5.0).is_none());
        assert!(xor_decode(&strings, 1.5, 0.0, 1.0).is_none());
    }
}
