use std::rc::Rc;

use crate::bail::{Bail, ErrorKind};
use crate::budget::ARRAY_LEN_CAP;
use crate::builtins::args::{
    argument, charge_for_length, check_string_cap, heap_checkpoint, live_array, live_text,
    reserve_text, write_property,
};
use crate::builtins::object::own_keys;
use crate::builtins::{define_builtin, Globals, NativeTable, Realm};
use crate::coerce::number_to_string;
use crate::host::Host;
use crate::ops::{string_key, to_js_string};
use crate::stack;
use crate::unmodelled::throw_or_refuse;
use crate::value::{Eval, JsString, ObjKind, ObjRef, Utf16Builder, Value};

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let json = realm.new_object(ObjKind::Plain(0), realm.object_prototype());
    if let Some(obj) = json.as_object().cloned() {
        let parse = realm.native_pure_value(table, ("JSON", "parse"), native_parse);
        define_builtin(&obj, "parse", parse);
        let stringify = realm.native_pure_value(table, ("JSON", "stringify"), native_stringify);
        define_builtin(&obj, "stringify", stringify);
    }
    globals.define("JSON", json);
}

fn native_parse<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let Some(rendered) = text.as_string() else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::SyntaxError,
            "invalid JSON",
            &"text the evaluator's JSON parser rejects",
        ));
    };
    let mut cursor = Cursor {
        bytes: rendered.as_bytes(),
        index: 0,
    };
    cursor.skip_whitespace();
    let Some(value) = cursor.parse(host)? else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::SyntaxError,
            "invalid JSON",
            &"text the evaluator's JSON parser rejects",
        ));
    };
    cursor.skip_whitespace();
    if cursor.index != cursor.bytes.len() {
        return Err(throw_or_refuse(
            host,
            ErrorKind::SyntaxError,
            "invalid JSON",
            &"text the evaluator's JSON parser rejects",
        ));
    }
    Ok(value)
}

struct Cursor<'t> {
    bytes: &'t [u8],
    index: usize,
}

impl<'t> Cursor<'t> {
    fn skip_whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.index),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.index += 1;
        }
    }

    fn parse<'a>(&mut self, host: &mut dyn Host<'a>) -> Eval<'a, Option<Value<'a>>> {
        host.enter_nesting()?;
        let result = stack::maybe_grow(|| self.parse_inner(host));
        host.leave_nesting();
        result
    }

    fn parse_inner<'a>(&mut self, host: &mut dyn Host<'a>) -> Eval<'a, Option<Value<'a>>> {
        self.skip_whitespace();
        let Some(byte) = self.bytes.get(self.index) else {
            return Ok(None);
        };
        host.spend(1)?;
        match byte {
            b'{' => self.parse_object(host),
            b'[' => self.parse_array(host),
            b'"' => Ok(self.parse_string(host)?.map(Value::Str)),
            b't' => Ok(self.literal("true", Value::Bool(true))),
            b'f' => Ok(self.literal("false", Value::Bool(false))),
            b'n' => Ok(self.literal("null", Value::Null)),
            _ => Ok(self.parse_number()),
        }
    }

    fn literal<'a>(&mut self, text: &str, value: Value<'a>) -> Option<Value<'a>> {
        if self.bytes.get(self.index..self.index + text.len())? == text.as_bytes() {
            self.index += text.len();
            return Some(value);
        }
        None
    }

    fn parse_number<'a>(&mut self) -> Option<Value<'a>> {
        let start = self.index;
        if self.bytes.get(self.index) == Some(&b'-') {
            self.index += 1;
        }
        self.parse_integer_digits()?;
        if self.bytes.get(self.index) == Some(&b'.') {
            self.index += 1;
            self.parse_digit_run()?;
        }
        if matches!(self.bytes.get(self.index), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.bytes.get(self.index), Some(b'+' | b'-')) {
                self.index += 1;
            }
            self.parse_digit_run()?;
        }
        let text = std::str::from_utf8(self.bytes.get(start..self.index)?).ok()?;
        text.parse::<f64>().ok().map(Value::Number)
    }

    fn parse_integer_digits(&mut self) -> Option<()> {
        match self.bytes.get(self.index) {
            Some(b'0') => {
                self.index += 1;
                Some(())
            }
            Some(b'1'..=b'9') => self.parse_digit_run(),
            _ => None,
        }
    }

    fn parse_digit_run(&mut self) -> Option<()> {
        let start = self.index;
        while matches!(self.bytes.get(self.index), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index == start {
            return None;
        }
        Some(())
    }

    fn parse_string<'a>(&mut self, host: &mut dyn Host<'a>) -> Eval<'a, Option<JsString>> {
        if self.bytes.get(self.index) != Some(&b'"') {
            return Ok(None);
        }
        self.index += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let Some(byte) = self.bytes.get(self.index).copied() else {
                return Ok(None);
            };
            self.index += 1;
            if byte == b'"' {
                return Ok(Some(live_text(host, units)?));
            }
            if byte < 0x20 {
                return Ok(None);
            }
            let ok = if byte == b'\\' {
                self.parse_escape(host, &mut units)?
            } else {
                self.parse_char(host, &mut units)?
            };
            if !ok {
                return Ok(None);
            }
        }
    }

    fn push_unit<'a>(
        &mut self,
        host: &mut dyn Host<'a>,
        units: &mut Vec<u16>,
        unit: u16,
    ) -> Eval<'a, ()> {
        host.spend(1)?;
        check_string_cap(units.len() + 1)?;
        units.push(unit);
        Ok(())
    }

    fn parse_escape<'a>(
        &mut self,
        host: &mut dyn Host<'a>,
        units: &mut Vec<u16>,
    ) -> Eval<'a, bool> {
        let Some(escape) = self.bytes.get(self.index).copied() else {
            return Ok(false);
        };
        self.index += 1;
        let unit = match escape {
            b'n' => 0x0A,
            b't' => 0x09,
            b'r' => 0x0D,
            b'b' => 0x08,
            b'f' => 0x0C,
            b'"' => 0x22,
            b'\\' => 0x5C,
            b'/' => 0x2F,
            b'u' => {
                let Some(unit) = self.parse_unicode_escape() else {
                    return Ok(false);
                };
                unit
            }
            _ => return Ok(false),
        };
        self.push_unit(host, units, unit)?;
        Ok(true)
    }

    fn parse_unicode_escape(&mut self) -> Option<u16> {
        let hex = std::str::from_utf8(self.bytes.get(self.index..self.index + 4)?).ok()?;
        let unit = u16::from_str_radix(hex, 16).ok()?;
        self.index += 4;
        Some(unit)
    }

    fn parse_char<'a>(&mut self, host: &mut dyn Host<'a>, units: &mut Vec<u16>) -> Eval<'a, bool> {
        let start = self.index - 1;
        let Some(end) = self.next_char_boundary(start) else {
            return Ok(false);
        };
        let Some(slice) = self.bytes.get(start..end) else {
            return Ok(false);
        };
        let Ok(text) = std::str::from_utf8(slice) else {
            return Ok(false);
        };
        for unit in text.encode_utf16() {
            self.push_unit(host, units, unit)?;
        }
        self.index = end;
        Ok(true)
    }

    fn next_char_boundary(&self, start: usize) -> Option<usize> {
        let width = match self.bytes.get(start)? {
            0x00..=0x7F => 1,
            0xC0..=0xDF => 2,
            0xE0..=0xEF => 3,
            _ => 4,
        };
        Some(start + width)
    }

    fn parse_array<'a>(&mut self, host: &mut dyn Host<'a>) -> Eval<'a, Option<Value<'a>>> {
        self.index += 1;
        let mut items = Vec::new();
        loop {
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b']') => {
                    self.index += 1;
                    return Ok(Some(live_array(host, items)?));
                }
                Some(_) => {}
                None => return Ok(None),
            }
            let Some(value) = self.parse(host)? else {
                return Ok(None);
            };
            if items.len() + 1 > ARRAY_LEN_CAP {
                return Err(Bail::ArrayLength.into());
            }
            heap_checkpoint(host, items.len())?;
            items.push(value);
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b']') => {}
                _ => return Ok(None),
            }
        }
    }

    fn parse_object<'a>(&mut self, host: &mut dyn Host<'a>) -> Eval<'a, Option<Value<'a>>> {
        self.index += 1;
        let built = host.new_object(ObjKind::Plain(0), None);
        let Some(obj) = built.as_object().cloned() else {
            return Ok(None);
        };
        let mut count = 0usize;
        loop {
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b'}') => {
                    self.index += 1;
                    return Ok(Some(built));
                }
                Some(_) => {}
                None => return Ok(None),
            }
            let Some(key) = self.parse_string(host)? else {
                return Ok(None);
            };
            self.skip_whitespace();
            if self.bytes.get(self.index) != Some(&b':') {
                return Ok(None);
            }
            self.index += 1;
            let Some(value) = self.parse(host)? else {
                return Ok(None);
            };
            if count + 1 > ARRAY_LEN_CAP {
                return Err(Bail::ArrayLength.into());
            }
            heap_checkpoint(host, count)?;
            count += 1;
            write_property(host, &obj, string_key(key), value)?;
            self.skip_whitespace();
            match self.bytes.get(self.index) {
                Some(b',') => self.index += 1,
                Some(b'}') => {}
                _ => return Ok(None),
            }
        }
    }
}

fn native_stringify<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let value = argument(args, 0);
    let mut stack: Vec<ObjRef<'a>> = Vec::new();
    match write_json(host, &value, &mut stack)? {
        Some(text) => Ok(Value::Str(text)),
        None => Ok(Value::Undefined),
    }
}

fn write_json<'a>(
    host: &mut dyn Host<'a>,
    value: &Value<'a>,
    stack: &mut Vec<ObjRef<'a>>,
) -> Eval<'a, Option<JsString>> {
    host.enter_nesting()?;
    let result = stack::maybe_grow(|| write_json_inner(host, value, stack));
    host.leave_nesting();
    result
}

fn write_json_inner<'a>(
    host: &mut dyn Host<'a>,
    value: &Value<'a>,
    stack: &mut Vec<ObjRef<'a>>,
) -> Eval<'a, Option<JsString>> {
    host.spend(1)?;
    match value {
        Value::Undefined => Ok(None),
        Value::Null => Ok(Some(JsString::from_str("null"))),
        Value::Bool(b) => Ok(Some(JsString::from_str(if *b { "true" } else { "false" }))),
        Value::Number(n) if n.is_finite() => Ok(Some(JsString::from_str(&number_to_string(*n)))),
        Value::Number(_) => Ok(Some(JsString::from_str("null"))),
        Value::Str(text) => {
            charge_for_length(host, text.len())?;
            let quoted = quote_json(text);
            Ok(Some(live_text(host, quoted.units().to_vec())?))
        }
        Value::Object(obj) => write_json_composite(host, value, obj, stack),
    }
}

fn write_json_composite<'a>(
    host: &mut dyn Host<'a>,
    value: &Value<'a>,
    obj: &ObjRef<'a>,
    stack: &mut Vec<ObjRef<'a>>,
) -> Eval<'a, Option<JsString>> {
    let to_json_key = string_key(JsString::from_str("toJSON"));
    if crate::object::decoding() {
        host.note_protocol_member(value, &to_json_key)?;
    }
    let to_json = host.get_property(value, &to_json_key)?;
    if to_json.is_callable() {
        return Err(Bail::Unsupported(&"toJSON").into());
    }
    if value.is_callable() {
        return Ok(None);
    }
    if stack.iter().any(|seen| Rc::ptr_eq(seen, obj)) {
        return Err(host.throw(
            ErrorKind::TypeError,
            "Converting circular structure to JSON",
        ));
    }
    stack.push(obj.clone());
    let is_array = matches!(obj.borrow().kind, ObjKind::Array(_));
    let rendered = if is_array {
        write_json_array(host, obj, stack)
    } else {
        write_json_object(host, value, stack)
    };
    stack.pop();
    Ok(Some(rendered?))
}

fn write_json_array<'a>(
    host: &mut dyn Host<'a>,
    obj: &ObjRef<'a>,
    stack: &mut Vec<ObjRef<'a>>,
) -> Eval<'a, JsString> {
    let items = crate::object::array_items(obj).unwrap_or_default();
    charge_for_length(host, items.len())?;
    let comma = JsString::from_str(",");
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    out.push(&JsString::from_str("["))?;
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push(&comma)?;
        }
        let rendered = write_json(host, item, stack)?.unwrap_or_else(|| JsString::from_str("null"));
        charge_for_length(host, rendered.len())?;
        out.push(&rendered)?;
    }
    out.push(&JsString::from_str("]"))?;
    Ok(out.finish())
}

fn write_json_object<'a>(
    host: &mut dyn Host<'a>,
    value: &Value<'a>,
    stack: &mut Vec<ObjRef<'a>>,
) -> Eval<'a, JsString> {
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    out.push(&JsString::from_str("{"))?;
    let mut first = true;
    for key in own_keys(host, value)? {
        let member = host.get_property(value, &string_key(key.clone()))?;
        let Some(rendered) = write_json(host, &member, stack)? else {
            continue;
        };
        if !first {
            out.push(&JsString::from_str(","))?;
        }
        first = false;
        let quoted = quote_json(&key);
        charge_for_length(host, quoted.len() + rendered.len())?;
        out.push(&quoted)?;
        out.push(&JsString::from_str(":"))?;
        out.push(&rendered)?;
    }
    out.push(&JsString::from_str("}"))?;
    Ok(out.finish())
}

fn quote_json(text: &JsString) -> JsString {
    let source = text.units();
    let mut units: Vec<u16> = vec![u16::from(b'"')];
    let mut index = 0;
    while index < source.len() {
        let unit = source[index];
        index += 1;
        match unit {
            0x22 => units.extend_from_slice(&[0x5C, 0x22]),
            0x5C => units.extend_from_slice(&[0x5C, 0x5C]),
            0x0A => units.extend_from_slice(&[0x5C, u16::from(b'n')]),
            0x0D => units.extend_from_slice(&[0x5C, u16::from(b'r')]),
            0x09 => units.extend_from_slice(&[0x5C, u16::from(b't')]),
            0x08 => units.extend_from_slice(&[0x5C, u16::from(b'b')]),
            0x0C => units.extend_from_slice(&[0x5C, u16::from(b'f')]),
            other if other < 0x20 => push_unicode_escape(&mut units, other),
            other if is_lone_surrogate(source, index - 1) => push_unicode_escape(&mut units, other),
            other => units.push(other),
        }
    }
    units.push(u16::from(b'"'));
    JsString::from_units(units)
}

fn is_lone_surrogate(units: &[u16], at: usize) -> bool {
    let Some(unit) = units.get(at).copied() else {
        return false;
    };
    if (0xD800..=0xDBFF).contains(&unit) {
        return !units
            .get(at + 1)
            .is_some_and(|next| (0xDC00..=0xDFFF).contains(next));
    }
    if (0xDC00..=0xDFFF).contains(&unit) {
        return at == 0 || !(0xD800..=0xDBFF).contains(&units[at - 1]);
    }
    false
}

fn push_unicode_escape(units: &mut Vec<u16>, unit: u16) {
    units.extend_from_slice(&[0x5C, u16::from(b'u')]);
    for digit in format!("{unit:04x}").chars() {
        units.push(digit as u16);
    }
}
