use oxc_ast::ast::Expression;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

use crate::coerce::{number_to_js_string, to_number_units};
use crate::ops::{number_key, string_key};
use crate::value::{js_string_from_literal, JsString, PropKey, Value};

#[derive(Clone, Debug)]
pub(crate) enum Prim {
    Num(f64),
    Str(JsString),
}

impl Prim {
    pub(crate) fn of(value: &Value<'_>) -> Option<Self> {
        match value {
            Value::Number(number) => Some(Prim::Num(*number)),
            Value::Str(text) => Some(Prim::Str(text.clone())),
            _ => None,
        }
    }

    pub(crate) fn value<'a>(&self) -> Value<'a> {
        match self {
            Prim::Num(number) => Value::Number(*number),
            Prim::Str(text) => Value::Str(text.clone()),
        }
    }

    pub(crate) fn key(&self) -> PropKey {
        match self {
            Prim::Num(number) => number_key(*number),
            Prim::Str(text) => string_key(text.clone()),
        }
    }

    pub(crate) fn number(&self) -> f64 {
        match self {
            Prim::Num(number) => *number,
            Prim::Str(text) => to_number_units(text.units()),
        }
    }

    fn text(&self) -> JsString {
        match self {
            Prim::Num(number) => number_to_js_string(*number),
            Prim::Str(text) => text.clone(),
        }
    }

    pub(crate) fn same(&self, other: &Prim) -> bool {
        match (self, other) {
            (Prim::Num(a), Prim::Num(b)) => a.to_bits() == b.to_bits(),
            (Prim::Str(a), Prim::Str(b)) => a.units() == b.units(),
            _ => false,
        }
    }
}

pub(crate) fn add(left: &Prim, right: &Prim) -> Option<Prim> {
    if let (Prim::Num(a), Prim::Num(b)) = (left, right) {
        return Some(Prim::Num(a + b));
    }
    left.text().concat(&right.text()).ok().map(Prim::Str)
}

pub(crate) fn binary(op: BinaryOperator, left: &Prim, right: &Prim) -> Option<Prim> {
    let (a, b) = (left.number(), right.number());
    let value = match op {
        BinaryOperator::Addition => return add(left, right),
        BinaryOperator::Subtraction => a - b,
        BinaryOperator::Multiplication => a * b,
        BinaryOperator::Division => a / b,
        BinaryOperator::Remainder => a % b,
        _ => return None,
    };
    Some(Prim::Num(value))
}

pub(crate) type Bindings<'s, 'a> = &'s [(&'a str, Option<Prim>)];

pub(crate) fn fold_constant(expr: &Expression<'_>, bindings: Bindings<'_, '_>) -> Option<Prim> {
    match expr {
        Expression::NumericLiteral(literal) => Some(Prim::Num(literal.value)),
        Expression::StringLiteral(literal) => Some(Prim::Str(js_string_from_literal(literal))),
        Expression::ParenthesizedExpression(node) => fold_constant(&node.expression, bindings),
        Expression::Identifier(id) => bindings
            .iter()
            .rev()
            .find(|(name, _)| *name == id.name.as_str())
            .and_then(|(_, value)| value.clone()),
        Expression::UnaryExpression(node) => {
            let operand = fold_constant(&node.argument, bindings)?.number();
            match node.operator {
                UnaryOperator::UnaryNegation => Some(Prim::Num(-operand)),
                UnaryOperator::UnaryPlus => Some(Prim::Num(operand)),
                _ => None,
            }
        }
        Expression::BinaryExpression(node) => {
            let left = fold_constant(&node.left, bindings)?;
            let right = fold_constant(&node.right, bindings)?;
            binary(node.operator, &left, &right)
        }
        _ => None,
    }
}
