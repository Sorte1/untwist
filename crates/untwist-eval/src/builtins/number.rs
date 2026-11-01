use crate::bail::{Bail, ErrorKind};
use crate::builtins::global::{native_number, native_parse_float, native_parse_int};
use crate::builtins::{define_builtin, define_constant, Globals, NativeFn, NativeTable, Realm};
use crate::coerce::{number_to_radix_string, number_to_string, to_integer_or_infinity};
use crate::host::Host;
use crate::ops::to_number_value;
use crate::value::{Eval, JsString, ObjRef, Value};

const SAFE_LIMIT: f64 = 9_007_199_254_740_992.0;
const FIXED_LIMIT: f64 = 1e21;
const EXACT_DIGITS: usize = 1100;

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let to_string =
        realm.native_pure_value(table, ("Number.prototype", "toString"), native_to_string);
    realm.define_number_member("toString", to_string);
    let value_of = realm.native_pure_value(table, ("Number.prototype", "valueOf"), native_value_of);
    realm.define_number_member("valueOf", value_of);
    let to_fixed = realm.native_pure_value(table, ("Number.prototype", "toFixed"), native_to_fixed);
    realm.define_number_member("toFixed", to_fixed);

    let number_global = realm.native_pure_value(table, ("Number", ""), native_number);
    if let Some(obj) = number_global.as_object().cloned() {
        install_number_statics(realm, table, &obj);
    }
    globals.define("Number", number_global);
}

fn install_number_statics<'a>(
    realm: &mut Realm<'a>,
    table: &mut NativeTable<'a>,
    obj: &ObjRef<'a>,
) {
    let statics: [(&str, NativeFn<'a>); 5] = [
        ("isInteger", native_is_integer),
        ("isNaN", native_is_nan),
        ("isFinite", native_is_finite),
        ("parseInt", native_parse_int),
        ("parseFloat", native_parse_float),
    ];
    for (name, function) in statics {
        let value = realm.native_pure_value(table, ("Number", name), function);
        define_builtin(obj, name, value);
    }
    for (name, value) in [
        ("MAX_SAFE_INTEGER", 9_007_199_254_740_991.0f64),
        ("MIN_SAFE_INTEGER", -9_007_199_254_740_991.0f64),
        ("EPSILON", f64::EPSILON),
        ("MAX_VALUE", f64::MAX),
        ("MIN_VALUE", f64::from_bits(1)),
    ] {
        define_constant(obj, name, Value::Number(value));
    }
}

fn native_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let value = this_number(host, &this)?;
    let radix = match args.first() {
        None | Some(Value::Undefined) => 10.0,
        Some(argument) => to_number_value(argument, host)?,
    };
    let radix = to_integer_or_infinity(radix);
    if !(2.0..=36.0).contains(&radix) {
        return Err(host.throw(ErrorKind::RangeError, "radix out of range"));
    }
    let shortest_digits = value.is_finite() && (value.fract() != 0.0 || value.abs() >= SAFE_LIMIT);
    if radix != 10.0 && shortest_digits {
        return Err(Bail::Unsupported(&"a radix string node shortens to round-trip digits").into());
    }
    Ok(Value::Str(JsString::from_str(&number_to_radix_string(
        value,
        radix as u32,
    ))))
}

fn native_value_of<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Number(this_number(host, &this)?))
}

fn native_to_fixed<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let value = this_number(host, &this)?;
    let digits = match args.first() {
        None | Some(Value::Undefined) => 0.0,
        Some(argument) => to_integer_or_infinity(to_number_value(argument, host)?),
    };
    if !(0.0..=100.0).contains(&digits) {
        return Err(host.throw(ErrorKind::RangeError, "digits out of range"));
    }
    if !value.is_finite() || value.abs() >= FIXED_LIMIT {
        return Ok(Value::Str(JsString::from_str(&number_to_string(value))));
    }
    let digits = digits as usize;
    if is_decimal_tie(value, digits) {
        return Err(Bail::Unsupported(&"a toFixed tie, which node rounds up").into());
    }
    let value = if value == 0.0 { 0.0 } else { value };
    Ok(Value::Str(JsString::from_str(&format!("{value:.digits$}"))))
}

fn is_decimal_tie(value: f64, digits: usize) -> bool {
    let exact = format!("{:.EXACT_DIGITS$}", value.abs());
    let Some((_, fraction)) = exact.split_once('.') else {
        return false;
    };
    let tail = &fraction.as_bytes()[digits.min(fraction.len())..];
    tail.first() == Some(&b'5') && tail[1..].iter().all(|digit| *digit == b'0')
}

fn native_is_integer<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(matches!(
        args.first(),
        Some(Value::Number(n)) if n.is_finite() && n.fract() == 0.0
    )))
}

fn native_is_nan<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(
        matches!(args.first(), Some(Value::Number(n)) if n.is_nan()),
    ))
}

fn native_is_finite<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(
        matches!(args.first(), Some(Value::Number(n)) if n.is_finite()),
    ))
}

fn this_number<'a>(host: &mut dyn Host<'a>, this: &Value<'a>) -> Eval<'a, f64> {
    if !matches!(this, Value::Number(_)) {
        crate::unmodelled::refuse_while_decoding(
            &"a Number method on a value that is not a number",
        )?;
    }
    to_number_value(this, host)
}
