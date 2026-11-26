use crate::builtins::args::{charge_for_length, reserve_text};
use crate::builtins::string::receiver;
use crate::builtins::{NativeFn, NativeTable};
use crate::coerce::to_integer_or_infinity;
use crate::host::Host;
use crate::object::{array_element, array_length};
use crate::ops::{to_js_string, to_number_value};
use crate::value::{Eval, JsString, NativeId, Utf16Builder, Value};

pub(crate) const STRING_OWNER: &str = "sink:String.prototype";

pub(crate) const ARRAY_OWNER: &str = "sink:Array.prototype";

const QUOTE: &str = "&quot;";

#[derive(Default)]
pub(crate) struct SinkMembers {
    pub(crate) strings: Vec<(&'static str, NativeId)>,
    pub(crate) arrays: Vec<(&'static str, NativeId)>,
}

pub(crate) fn register<'a>(table: &mut NativeTable<'a>) -> SinkMembers {
    let strings: [(&'static str, NativeFn<'a>); 14] = [
        ("anchor", native_anchor),
        ("at", native_string_at),
        ("big", native_big),
        ("blink", native_blink),
        ("bold", native_bold),
        ("fixed", native_fixed),
        ("fontcolor", native_fontcolor),
        ("fontsize", native_fontsize),
        ("italics", native_italics),
        ("link", native_link),
        ("small", native_small),
        ("strike", native_strike),
        ("sub", native_sub),
        ("sup", native_sup),
    ];
    let arrays: [(&'static str, NativeFn<'a>); 1] = [("at", native_array_at)];
    SinkMembers {
        strings: strings
            .into_iter()
            .map(|(name, function)| (name, table.register(STRING_OWNER, name, function)))
            .collect(),
        arrays: arrays
            .into_iter()
            .map(|(name, function)| (name, table.register(ARRAY_OWNER, name, function)))
            .collect(),
    }
}

fn html<'a>(
    host: &mut dyn Host<'a>,
    this: &Value<'a>,
    tag: &str,
    attribute: Option<(&str, &Value<'a>)>,
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, this)?;
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    let open: Vec<u16> = format!("<{tag}").encode_utf16().collect();
    out.push(&JsString::from_units(open))?;
    if let Some((name, value)) = attribute {
        let value = to_js_string(value, host)?;
        charge_for_length(host, value.len())?;
        let head: Vec<u16> = format!(" {name}=\"").encode_utf16().collect();
        out.push(&JsString::from_units(head))?;
        let quote: Vec<u16> = QUOTE.encode_utf16().collect();
        let escaped: Vec<u16> = value
            .units()
            .iter()
            .flat_map(|unit| match *unit == u16::from(b'"') {
                true => quote.clone(),
                false => vec![*unit],
            })
            .collect();
        out.push(&JsString::from_units(escaped))?;
        out.push(&JsString::from_str("\""))?;
    }
    charge_for_length(host, text.len())?;
    out.push(&JsString::from_str(">"))?;
    out.push(&text)?;
    let close: Vec<u16> = format!("</{tag}>").encode_utf16().collect();
    out.push(&JsString::from_units(close))?;
    Ok(Value::Str(out.finish()))
}

fn first<'a>(args: &[Value<'a>]) -> Value<'a> {
    args.first().cloned().unwrap_or(Value::Undefined)
}

fn native_anchor<'a>(h: &mut dyn Host<'a>, t: Value<'a>, a: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "a", Some(("name", &first(a))))
}

fn native_big<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "big", None)
}

fn native_blink<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "blink", None)
}

fn native_bold<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "b", None)
}

fn native_fixed<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "tt", None)
}

fn native_fontcolor<'a>(
    h: &mut dyn Host<'a>,
    t: Value<'a>,
    a: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    html(h, &t, "font", Some(("color", &first(a))))
}

fn native_fontsize<'a>(h: &mut dyn Host<'a>, t: Value<'a>, a: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "font", Some(("size", &first(a))))
}

fn native_italics<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "i", None)
}

fn native_link<'a>(h: &mut dyn Host<'a>, t: Value<'a>, a: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "a", Some(("href", &first(a))))
}

fn native_small<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "small", None)
}

fn native_strike<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "strike", None)
}

fn native_sub<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "sub", None)
}

fn native_sup<'a>(h: &mut dyn Host<'a>, t: Value<'a>, _: &[Value<'a>]) -> Eval<'a, Value<'a>> {
    html(h, &t, "sup", None)
}

fn relative_index<'a>(
    host: &mut dyn Host<'a>,
    args: &[Value<'a>],
    len: usize,
) -> Eval<'a, Option<usize>> {
    let raw = match args.first() {
        None | Some(Value::Undefined) => 0.0,
        Some(value) => to_number_value(value, host)?,
    };
    let relative = to_integer_or_infinity(raw);
    let index = match relative >= 0.0 {
        true => relative,
        false => len as f64 + relative,
    };
    Ok((index >= 0.0 && index < len as f64).then_some(index as usize))
}

fn native_string_at<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let Some(index) = relative_index(host, args, text.len())? else {
        return Ok(Value::Undefined);
    };
    let unit = text.code_unit_at(index).unwrap_or(0);
    Ok(Value::Str(host.unit_string(unit)?))
}

fn native_array_at<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let Some(obj) = this.as_object().cloned() else {
        return Ok(Value::Undefined);
    };
    let len = array_length(&obj).unwrap_or(0);
    let Some(index) = relative_index(host, args, len)? else {
        return Ok(Value::Undefined);
    };
    Ok(array_element(&obj, index).unwrap_or(Value::Undefined))
}
