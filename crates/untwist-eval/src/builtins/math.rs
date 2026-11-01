use crate::bail::Bail;
use crate::builtins::{define_builtin, define_constant, Globals, NativeFn, NativeTable, Realm};
use crate::coerce::to_int32;
use crate::host::Host;
use crate::ops::{exponentiate, to_number_value};
use crate::value::{Eval, ObjKind, Value};

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let math = realm.new_object(ObjKind::Plain(0), realm.object_prototype());
    let Some(obj) = math.as_object().cloned() else {
        return;
    };
    let members: [(&str, NativeFn<'a>); 12] = [
        ("floor", native_floor),
        ("ceil", native_ceil),
        ("round", native_round),
        ("abs", native_abs),
        ("trunc", native_trunc),
        ("sign", native_sign),
        ("sqrt", native_sqrt),
        ("log", native_log),
        ("pow", native_pow),
        ("imul", native_imul),
        ("min", native_min),
        ("max", native_max),
    ];
    for (name, function) in members {
        let value = realm.native_pure_value(table, ("Math", name), function);
        define_builtin(&obj, name, value);
    }
    let random = realm.native_value(table, ("Math", "random"), native_random);
    define_builtin(&obj, "random", random);
    for (name, value) in [
        ("PI", std::f64::consts::PI),
        ("E", std::f64::consts::E),
        ("LN2", std::f64::consts::LN_2),
    ] {
        define_constant(&obj, name, Value::Number(value));
    }
    globals.define("Math", math);
}

fn first<'a>(host: &mut dyn Host<'a>, args: &[Value<'a>]) -> Eval<'a, f64> {
    match args.first() {
        None => Ok(f64::NAN),
        Some(value) => to_number_value(value, host),
    }
}

macro_rules! unary_math {
    ($name:ident, $body:expr) => {
        fn $name<'a>(
            host: &mut dyn Host<'a>,
            _this: Value<'a>,
            args: &[Value<'a>],
        ) -> Eval<'a, Value<'a>> {
            let value = first(host, args)?;
            let compute: fn(f64) -> f64 = $body;
            Ok(Value::Number(compute(value)))
        }
    };
}

unary_math!(native_floor, |n| n.floor());
unary_math!(native_ceil, |n| n.ceil());
unary_math!(native_abs, |n| n.abs());
unary_math!(native_trunc, |n| n.trunc());
unary_math!(native_sqrt, |n| n.sqrt());
unary_math!(native_round, |n| {
    if n.is_nan() || n.is_infinite() || n == 0.0 {
        return n;
    }
    let floor = n.floor();
    let rounded = if n - floor >= 0.5 { floor + 1.0 } else { floor };
    if rounded == 0.0 && n < 0.0 {
        -0.0
    } else {
        rounded
    }
});

fn native_log<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let value = first(host, args)?;
    let exact = value.is_nan() || value <= 0.0 || value == 1.0 || value == f64::INFINITY;
    if !exact {
        return Err(Bail::Unsupported(&"a logarithm the evaluator cannot prove exact").into());
    }
    Ok(Value::Number(value.ln()))
}
unary_math!(native_sign, |n| {
    if n.is_nan() {
        f64::NAN
    } else if n > 0.0 {
        1.0
    } else if n < 0.0 {
        -1.0
    } else {
        n
    }
});

fn native_pow<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let base = first(host, args)?;
    let exponent = match args.get(1) {
        None => f64::NAN,
        Some(value) => to_number_value(value, host)?,
    };
    Ok(Value::Number(exponentiate(base, exponent)?))
}

fn native_imul<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let a = to_int32(first(host, args)?);
    let b = match args.get(1) {
        None => 0,
        Some(value) => to_int32(to_number_value(value, host)?),
    };
    Ok(Value::Number(f64::from(a.wrapping_mul(b))))
}

fn fold_extreme<'a>(
    host: &mut dyn Host<'a>,
    args: &[Value<'a>],
    seed: f64,
    take_larger: bool,
) -> Eval<'a, Value<'a>> {
    let mut best = seed;
    for value in args {
        let candidate = to_number_value(value, host)?;
        if best.is_nan() || candidate.is_nan() {
            best = f64::NAN;
            continue;
        }
        if beats(candidate, best, take_larger) {
            best = candidate;
        }
    }
    Ok(Value::Number(best))
}

fn beats(candidate: f64, best: f64, take_larger: bool) -> bool {
    if candidate == 0.0 && best == 0.0 {
        return candidate.is_sign_negative() != take_larger
            && best.is_sign_negative() == take_larger;
    }
    if take_larger {
        candidate > best
    } else {
        candidate < best
    }
}

fn native_min<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    fold_extreme(host, args, f64::INFINITY, false)
}

fn native_max<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    fold_extreme(host, args, f64::NEG_INFINITY, true)
}

fn native_random<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Err(Bail::Unsupported(&"Math.random, whose value the evaluator cannot know").into())
}
