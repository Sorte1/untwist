use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::builtins::args::{charge_for_length, reserve_text};
use crate::coerce::{number_to_js_string, to_int32, to_number_units, to_uint32};
use crate::host::Host;
use crate::value::{Eval, JsString, PropKey, Value};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    Number,
    String,
    Default,
}

pub fn to_boolean(value: &Value<'_>) -> bool {
    match value {
        Value::Undefined | Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => *n != 0.0 && !n.is_nan(),
        Value::Str(s) => !s.is_empty(),
        Value::Object(_) => true,
    }
}

pub fn type_of(value: &Value<'_>) -> &'static str {
    match value {
        Value::Undefined => "undefined",
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::Str(_) => "string",
        Value::Object(_) => {
            if value.is_callable() {
                "function"
            } else {
                "object"
            }
        }
    }
}

pub fn to_primitive<'a>(
    value: &Value<'a>,
    hint: Hint,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    let Value::Object(_) = value else {
        return Ok(value.clone());
    };
    if crate::object::decoding() {
        let marker = PropKey::Name(JsString::from_str("@@toPrimitive"));
        let method = host.get_property(value, &marker)?;
        if method.is_callable() {
            host.refuse_coercion_call(&method)?;
            let result = host.call_value(&method, value.clone(), &[])?;
            if !matches!(result, Value::Object(_)) {
                return Ok(result);
            }
        }
    }
    let order = match hint {
        Hint::String => ["toString", "valueOf"],
        Hint::Number | Hint::Default => ["valueOf", "toString"],
    };
    for name in order {
        let method = host.get_property(value, &PropKey::Name(JsString::from_str(name)))?;
        if !method.is_callable() {
            continue;
        }
        host.refuse_coercion_call(&method)?;
        let result = host.call_value(&method, value.clone(), &[])?;
        if !matches!(result, Value::Object(_)) {
            return Ok(result);
        }
    }
    Err(host.throw(
        ErrorKind::TypeError,
        "cannot convert object to primitive value",
    ))
}

pub fn to_number_value<'a>(value: &Value<'a>, host: &mut dyn Host<'a>) -> Eval<'a, f64> {
    match value {
        Value::Undefined => Ok(f64::NAN),
        Value::Null => Ok(0.0),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::Number(n) => Ok(*n),
        Value::Str(s) => {
            charge_for_length(host, s.len())?;
            Ok(to_number_units(s.units()))
        }
        Value::Object(_) => {
            let primitive = to_primitive(value, Hint::Number, host)?;
            to_number_value(&primitive, host)
        }
    }
}

pub fn to_js_string<'a>(value: &Value<'a>, host: &mut dyn Host<'a>) -> Eval<'a, JsString> {
    match value {
        Value::Undefined => Ok(JsString::from_str("undefined")),
        Value::Null => Ok(JsString::from_str("null")),
        Value::Bool(b) => Ok(JsString::from_str(if *b { "true" } else { "false" })),
        Value::Number(n) => Ok(number_to_js_string(*n)),
        Value::Str(s) => Ok(s.clone()),
        Value::Object(_) => {
            let primitive = to_primitive(value, Hint::String, host)?;
            to_js_string(&primitive, host)
        }
    }
}

pub fn to_property_key<'a>(value: &Value<'a>, host: &mut dyn Host<'a>) -> Eval<'a, PropKey> {
    if let Value::Number(n) = value {
        return Ok(number_key(*n));
    }
    let text = to_js_string(value, host)?;
    charge_for_length(host, text.len())?;
    Ok(string_key(text))
}

const MAX_INDEX_DIGITS: usize = 10;

fn canonical_index(units: &[u16]) -> Option<u32> {
    if units.is_empty() || units.len() > MAX_INDEX_DIGITS {
        return None;
    }
    if units.len() > 1 && units[0] == u16::from(b'0') {
        return None;
    }
    let mut value: u32 = 0;
    for unit in units {
        let digit = unit.checked_sub(u16::from(b'0'))?;
        if digit > 9 {
            return None;
        }
        value = value.checked_mul(10)?.checked_add(u32::from(digit))?;
    }
    Some(value)
}

pub fn string_key(text: JsString) -> PropKey {
    match canonical_index(text.units()) {
        Some(index) => PropKey::Index(index),
        None => PropKey::Name(text),
    }
}

pub fn number_key(n: f64) -> PropKey {
    let index = n as u32;
    if f64::from(index) == n {
        return PropKey::Index(index);
    }
    PropKey::Name(number_to_js_string(n))
}

pub fn unary_op<'a>(
    op: UnaryOperator,
    value: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    host.spend(1)?;
    match op {
        UnaryOperator::UnaryNegation => Ok(Value::Number(-to_number_value(value, host)?)),
        UnaryOperator::UnaryPlus => Ok(Value::Number(to_number_value(value, host)?)),
        UnaryOperator::LogicalNot => Ok(Value::Bool(!to_boolean(value))),
        UnaryOperator::BitwiseNot => {
            let n = to_number_value(value, host)?;
            Ok(Value::Number(f64::from(!to_int32(n))))
        }
        UnaryOperator::Typeof => Ok(Value::Str(JsString::from_str(type_of(value)))),
        UnaryOperator::Void => Ok(Value::Undefined),
        UnaryOperator::Delete => Err(Bail::Unsupported(&"delete on a non-member operand").into()),
    }
}

pub fn strict_equals<'a>(left: &Value<'a>, right: &Value<'a>) -> bool {
    match (left, right) {
        (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a.units() == b.units(),
        (Value::Object(a), Value::Object(b)) => std::rc::Rc::ptr_eq(a, b),
        _ => false,
    }
}

pub fn same_value_zero<'a>(left: &Value<'a>, right: &Value<'a>) -> bool {
    if let (Value::Number(a), Value::Number(b)) = (left, right) {
        return (a.is_nan() && b.is_nan()) || a == b;
    }
    strict_equals(left, right)
}

pub fn loose_equals<'a>(
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, bool> {
    host.spend(1)?;
    match (left, right) {
        (Value::Undefined | Value::Null, Value::Undefined | Value::Null) => Ok(true),
        (Value::Undefined | Value::Null, _) | (_, Value::Undefined | Value::Null) => Ok(false),
        (Value::Number(_), Value::Str(_)) | (Value::Str(_), Value::Number(_)) => {
            let a = to_number_value(left, host)?;
            let b = to_number_value(right, host)?;
            Ok(a == b)
        }
        (Value::Bool(_), _) => {
            let a = Value::Number(to_number_value(left, host)?);
            loose_equals(&a, right, host)
        }
        (_, Value::Bool(_)) => {
            let b = Value::Number(to_number_value(right, host)?);
            loose_equals(left, &b, host)
        }
        (Value::Object(_), Value::Number(_) | Value::Str(_)) => {
            let a = to_primitive(left, Hint::Default, host)?;
            loose_equals(&a, right, host)
        }
        (Value::Number(_) | Value::Str(_), Value::Object(_)) => {
            let b = to_primitive(right, Hint::Default, host)?;
            loose_equals(left, &b, host)
        }
        _ => Ok(strict_equals(left, right)),
    }
}

pub fn binary_op<'a>(
    op: BinaryOperator,
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    host.spend(1 + string_comparison_cost(left, right))?;
    match op {
        BinaryOperator::Addition => addition(left, right, host),
        BinaryOperator::Equality => Ok(Value::Bool(loose_equals(left, right, host)?)),
        BinaryOperator::Inequality => Ok(Value::Bool(!loose_equals(left, right, host)?)),
        BinaryOperator::StrictEquality => Ok(Value::Bool(strict_equals(left, right))),
        BinaryOperator::StrictInequality => Ok(Value::Bool(!strict_equals(left, right))),
        BinaryOperator::LessThan
        | BinaryOperator::LessEqualThan
        | BinaryOperator::GreaterThan
        | BinaryOperator::GreaterEqualThan => compare(op, left, right, host),
        BinaryOperator::In | BinaryOperator::Instanceof => {
            Err(Bail::Unsupported(&"`in` / `instanceof` outside the interpreter").into())
        }
        _ => numeric(op, left, right, host),
    }
}

pub(crate) fn charged_strict_equals<'a>(
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> EvalResult<bool> {
    host.spend(string_comparison_cost(left, right))?;
    Ok(strict_equals(left, right))
}

fn string_comparison_cost(left: &Value<'_>, right: &Value<'_>) -> u64 {
    match (left, right) {
        (Value::Str(a), Value::Str(b)) => (a.len().min(b.len()) as u64) / 64,
        _ => 0,
    }
}

fn addition<'a>(
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    let a = to_primitive(left, Hint::Default, host)?;
    let b = to_primitive(right, Hint::Default, host)?;
    if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) {
        let head = to_js_string(&a, host)?;
        let tail = to_js_string(&b, host)?;
        charge_for_length(host, head.len() + tail.len())?;
        let charge = reserve_text(host, head.len() + tail.len())?;
        return Ok(Value::Str(head.concat_charged(&tail, charge)?));
    }
    Ok(Value::Number(
        to_number_value(&a, host)? + to_number_value(&b, host)?,
    ))
}

fn numeric<'a>(
    op: BinaryOperator,
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    let a = to_number_value(left, host)?;
    let b = to_number_value(right, host)?;
    let value = match op {
        BinaryOperator::Subtraction => a - b,
        BinaryOperator::Multiplication => a * b,
        BinaryOperator::Division => a / b,
        BinaryOperator::Remainder => remainder(a, b),
        BinaryOperator::Exponential => exponentiate(a, b)?,
        BinaryOperator::BitwiseAnd => f64::from(to_int32(a) & to_int32(b)),
        BinaryOperator::BitwiseOR => f64::from(to_int32(a) | to_int32(b)),
        BinaryOperator::BitwiseXOR => f64::from(to_int32(a) ^ to_int32(b)),
        BinaryOperator::ShiftLeft => f64::from(to_int32(a).wrapping_shl(to_uint32(b) & 31)),
        BinaryOperator::ShiftRight => f64::from(to_int32(a).wrapping_shr(to_uint32(b) & 31)),
        BinaryOperator::ShiftRightZeroFill => {
            f64::from(to_uint32(a).wrapping_shr(to_uint32(b) & 31))
        }
        _ => return Err(Bail::Unsupported(&"binary operator").into()),
    };
    Ok(Value::Number(value))
}

const EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

#[inline(always)]
fn exact_integer(n: f64) -> bool {
    n.abs() < EXACT_INTEGER && (n as i64) as f64 == n
}

#[inline(always)]
pub fn remainder(a: f64, b: f64) -> f64 {
    if exact_integer(a) && exact_integer(b) && b != 0.0 {
        return (((a as i64).wrapping_rem(b as i64)) as f64).copysign(a);
    }
    a % b
}

#[inline(always)]
pub fn number_binary<'a>(op: BinaryOperator, a: f64, b: f64) -> Option<Value<'a>> {
    Some(match op {
        BinaryOperator::Addition => Value::Number(a + b),
        BinaryOperator::Subtraction => Value::Number(a - b),
        BinaryOperator::Multiplication => Value::Number(a * b),
        BinaryOperator::Division => Value::Number(a / b),
        BinaryOperator::Remainder => Value::Number(remainder(a, b)),
        BinaryOperator::BitwiseAnd => Value::Number(f64::from(to_int32(a) & to_int32(b))),
        BinaryOperator::BitwiseOR => Value::Number(f64::from(to_int32(a) | to_int32(b))),
        BinaryOperator::BitwiseXOR => Value::Number(f64::from(to_int32(a) ^ to_int32(b))),
        BinaryOperator::ShiftLeft => {
            Value::Number(f64::from(to_int32(a).wrapping_shl(to_uint32(b) & 31)))
        }
        BinaryOperator::ShiftRight => {
            Value::Number(f64::from(to_int32(a).wrapping_shr(to_uint32(b) & 31)))
        }
        BinaryOperator::ShiftRightZeroFill => {
            Value::Number(f64::from(to_uint32(a).wrapping_shr(to_uint32(b) & 31)))
        }
        BinaryOperator::LessThan => Value::Bool(a < b),
        BinaryOperator::LessEqualThan => Value::Bool(a <= b),
        BinaryOperator::GreaterThan => Value::Bool(a > b),
        BinaryOperator::GreaterEqualThan => Value::Bool(a >= b),
        BinaryOperator::Equality | BinaryOperator::StrictEquality => Value::Bool(a == b),
        BinaryOperator::Inequality | BinaryOperator::StrictInequality => Value::Bool(a != b),
        _ => return None,
    })
}

#[inline(always)]
pub fn binary_fast<'a>(
    op: BinaryOperator,
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut crate::interp::Interp<'a>,
) -> Eval<'a, Value<'a>> {
    if let (Value::Number(a), Value::Number(b)) = (left, right) {
        if let Some(value) = number_binary(op, *a, *b) {
            host.budget.spend(1)?;
            if matches!(op, BinaryOperator::Equality | BinaryOperator::Inequality) {
                host.budget.spend(1)?;
            }
            return Ok(value);
        }
    }
    binary_op(op, left, right, host)
}

const EXACT_LIMIT: i128 = 1 << 53;
const MAX_EXACT_EXPONENT: f64 = 64.0;

pub fn exponentiate(base: f64, exponent: f64) -> EvalResult<f64> {
    exact_power(base, exponent).ok_or(Bail::Unsupported(
        &"a power whose result the evaluator cannot prove exact",
    ))
}

fn exact_power(base: f64, exponent: f64) -> Option<f64> {
    if exponent.is_nan() {
        return Some(f64::NAN);
    }
    if exponent == 0.0 {
        return Some(1.0);
    }
    if base.is_nan() {
        return Some(f64::NAN);
    }
    if exponent.is_infinite() {
        return Some(infinite_power(base.abs(), exponent > 0.0));
    }
    if base == 0.0 || base.is_infinite() {
        return Some(edge_power(base, exponent));
    }
    if base < 0.0 && exponent.fract() != 0.0 {
        return Some(f64::NAN);
    }
    let integral = exponent > 0.0 && exponent.fract() == 0.0 && base.fract() == 0.0;
    if !integral || base.abs() >= EXACT_LIMIT as f64 {
        return None;
    }
    if base.abs() <= 1.0 {
        let odd = exponent % 2.0 == 1.0;
        return Some(if base < 0.0 && odd { -1.0 } else { base.abs() });
    }
    if exponent > MAX_EXACT_EXPONENT {
        return None;
    }
    integer_power(base as i128, exponent as u32)
}

fn edge_power(base: f64, exponent: f64) -> f64 {
    let odd = exponent.fract() == 0.0 && (exponent % 2.0).abs() == 1.0;
    let magnitude = if (base == 0.0) == (exponent < 0.0) {
        f64::INFINITY
    } else {
        0.0
    };
    if base.is_sign_negative() && odd {
        -magnitude
    } else {
        magnitude
    }
}

fn infinite_power(magnitude: f64, positive: bool) -> f64 {
    if magnitude == 1.0 {
        return f64::NAN;
    }
    if (magnitude > 1.0) == positive {
        f64::INFINITY
    } else {
        0.0
    }
}

fn integer_power(base: i128, exponent: u32) -> Option<f64> {
    let mut result: i128 = 1;
    for _ in 0..exponent {
        result = result.checked_mul(base)?;
        if result.abs() > EXACT_LIMIT {
            return None;
        }
    }
    Some(result as f64)
}

fn compare<'a>(
    op: BinaryOperator,
    left: &Value<'a>,
    right: &Value<'a>,
    host: &mut dyn Host<'a>,
) -> Eval<'a, Value<'a>> {
    let a = to_primitive(left, Hint::Number, host)?;
    let b = to_primitive(right, Hint::Number, host)?;
    if let (Value::Str(x), Value::Str(y)) = (&a, &b) {
        let ordering = x.units().cmp(y.units());
        return Ok(Value::Bool(match op {
            BinaryOperator::LessThan => ordering.is_lt(),
            BinaryOperator::LessEqualThan => ordering.is_le(),
            BinaryOperator::GreaterThan => ordering.is_gt(),
            _ => ordering.is_ge(),
        }));
    }
    let x = to_number_value(&a, host)?;
    let y = to_number_value(&b, host)?;
    if x.is_nan() || y.is_nan() {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(match op {
        BinaryOperator::LessThan => x < y,
        BinaryOperator::LessEqualThan => x <= y,
        BinaryOperator::GreaterThan => x > y,
        _ => x >= y,
    }))
}
