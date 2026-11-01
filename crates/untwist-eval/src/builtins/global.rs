use crate::bail::ErrorKind;
use crate::builtins::args::{argument, check_string_growth, fitted};
use crate::builtins::{Globals, NativeTable, Realm};
use crate::coerce::{parse_float, parse_int};
use crate::host::Host;
use crate::ops::{to_boolean, to_js_string, to_number_value};
use crate::unmodelled::throw_or_refuse;
use crate::value::{Abrupt, Eval, Value};

const URI_COMPONENT_UNRESERVED: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
const URI_UNRESERVED: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'();/?:@&=+$,#";
const URI_RESERVED: &[u8] = b";/?:@&=+$,#";
const ESCAPE_UNRESERVED: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789@*_+-./";
const BASE64_STANDARD: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const NOT_BASE64: u8 = 0x80;

const BASE64_INDEX: [u8; 256] = base64_index();

const fn base64_index() -> [u8; 256] {
    let alphabet = BASE64_STANDARD.as_bytes();
    let mut table = [NOT_BASE64; 256];
    let mut index = 0;
    while index < alphabet.len() {
        table[alphabet[index] as usize] = index as u8;
        index += 1;
    }
    table
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MalformedUri;

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let entries: [(&'static str, crate::builtins::NativeFn<'a>); 13] = [
        ("parseInt", native_parse_int),
        ("parseFloat", native_parse_float),
        ("Boolean", native_boolean),
        ("isNaN", native_is_nan),
        ("isFinite", native_is_finite),
        ("decodeURIComponent", native_decode_uri_component),
        ("decodeURI", native_decode_uri),
        ("encodeURIComponent", native_encode_uri_component),
        ("encodeURI", native_encode_uri),
        ("escape", native_escape),
        ("unescape", native_unescape),
        ("atob", native_atob),
        ("btoa", native_btoa),
    ];
    for (name, function) in entries {
        let value = realm.native_pure_value(table, ("global", name), function);
        globals.define(name, value);
    }
}

pub(crate) fn native_parse_int<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let radix = match args.get(1) {
        None | Some(Value::Undefined) => None,
        Some(value) => Some(to_number_value(value, host)?),
    };
    match parse_int(text.units(), radix) {
        Some(value) => Ok(Value::Number(value)),
        None => Err(crate::bail::Bail::Unsupported(
            &"a parseInt result in a radix node rounds differently",
        )
        .into()),
    }
}

pub(crate) fn native_parse_float<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    Ok(Value::Number(parse_float(text.units())))
}

pub(crate) fn native_number<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if args.is_empty() {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(to_number_value(&argument(args, 0), host)?))
}

fn native_boolean<'a>(
    _host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(to_boolean(&argument(args, 0))))
}

fn native_is_nan<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(
        to_number_value(&argument(args, 0), host)?.is_nan(),
    ))
}

fn native_is_finite<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Bool(
        to_number_value(&argument(args, 0), host)?.is_finite(),
    ))
}

pub fn percent_decode_utf8(
    units: &[u16],
    preserve_reserved: bool,
) -> Result<Vec<u16>, MalformedUri> {
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let unit = *units.get(index).ok_or(MalformedUri)?;
        if unit != u16::from(b'%') {
            out.push(unit);
            index += 1;
            continue;
        }
        let (decoded, consumed) = decode_percent_run(units, index, preserve_reserved)?;
        out.extend_from_slice(&decoded);
        index += consumed;
    }
    Ok(out)
}

fn decode_percent_run(
    units: &[u16],
    start: usize,
    preserve_reserved: bool,
) -> Result<(Vec<u16>, usize), MalformedUri> {
    let first_byte = percent_byte(units, start)?;
    if preserve_reserved && first_byte < 0x80 && URI_RESERVED.contains(&first_byte) {
        let literal = units.get(start..start + 3).ok_or(MalformedUri)?;
        return Ok((literal.to_vec(), 3));
    }
    let extra = utf8_continuation_count(first_byte)?;
    let mut bytes = Vec::with_capacity(extra + 1);
    bytes.push(first_byte);
    let mut cursor = start + 3;
    for _ in 0..extra {
        let byte = percent_byte(units, cursor)?;
        if byte & 0xC0 != 0x80 {
            return Err(MalformedUri);
        }
        bytes.push(byte);
        cursor += 3;
    }
    let text = String::from_utf8(bytes).map_err(|_| MalformedUri)?;
    Ok((text.encode_utf16().collect(), cursor - start))
}

fn percent_byte(units: &[u16], at: usize) -> Result<u8, MalformedUri> {
    let marker = units.get(at).copied().ok_or(MalformedUri)?;
    if marker != u16::from(b'%') {
        return Err(MalformedUri);
    }
    let high = hex_digit(units.get(at + 1).copied())?;
    let low = hex_digit(units.get(at + 2).copied())?;
    Ok((high << 4) | low)
}

fn utf8_continuation_count(first_byte: u8) -> Result<usize, MalformedUri> {
    if first_byte & 0x80 == 0 {
        Ok(0)
    } else if first_byte & 0xE0 == 0xC0 {
        Ok(1)
    } else if first_byte & 0xF0 == 0xE0 {
        Ok(2)
    } else if first_byte & 0xF8 == 0xF0 {
        Ok(3)
    } else {
        Err(MalformedUri)
    }
}

fn hex_digit(unit: Option<u16>) -> Result<u8, MalformedUri> {
    let unit = unit.ok_or(MalformedUri)?;
    let ch = char::from_u32(u32::from(unit)).ok_or(MalformedUri)?;
    ch.to_digit(16).map(|d| d as u8).ok_or(MalformedUri)
}

pub fn percent_encode_utf8(units: &[u16], unreserved: &str) -> Result<Vec<u16>, MalformedUri> {
    let text = String::from_utf16(units).map_err(|_| MalformedUri)?;
    let mut out = String::with_capacity(text.len());
    for scalar in text.chars() {
        if scalar.is_ascii() && unreserved.contains(scalar) {
            out.push(scalar);
            continue;
        }
        let mut buffer = [0u8; 4];
        for byte in scalar.encode_utf8(&mut buffer).as_bytes() {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    Ok(out.encode_utf16().collect())
}

fn native_decode_uri_component<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len())?;
    match percent_decode_utf8(text.units(), false) {
        Ok(units) => Ok(Value::Str(fitted(units, charge))),
        Err(MalformedUri) => Err(host.throw(ErrorKind::UriError, "URI malformed")),
    }
}

fn native_decode_uri<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len())?;
    match percent_decode_utf8(text.units(), true) {
        Ok(units) => Ok(Value::Str(fitted(units, charge))),
        Err(MalformedUri) => Err(host.throw(ErrorKind::UriError, "URI malformed")),
    }
}

fn encode_with<'a>(
    host: &mut dyn Host<'a>,
    args: &[Value<'a>],
    unreserved: &str,
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len().saturating_mul(9))?;
    match percent_encode_utf8(text.units(), unreserved) {
        Ok(units) => Ok(Value::Str(fitted(units, charge))),
        Err(MalformedUri) => Err(host.throw(ErrorKind::UriError, "URI malformed")),
    }
}

fn native_encode_uri_component<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    encode_with(host, args, URI_COMPONENT_UNRESERVED)
}

fn native_encode_uri<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    encode_with(host, args, URI_UNRESERVED)
}

fn native_escape<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len().saturating_mul(6))?;
    let mut out = String::new();
    for unit in text.units() {
        let unit = *unit;
        let ascii = char::from_u32(u32::from(unit)).filter(char::is_ascii);
        match ascii {
            Some(ch) if ESCAPE_UNRESERVED.contains(ch) => out.push(ch),
            _ if unit < 0x100 => out.push_str(&format!("%{unit:02X}")),
            _ => out.push_str(&format!("%u{unit:04X}")),
        }
    }
    Ok(Value::Str(fitted(out.encode_utf16().collect(), charge)))
}

fn native_unescape<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len())?;
    let units = text.units();
    let mut out: Vec<u16> = Vec::with_capacity(units.len());
    let mut index = 0;
    while index < units.len() {
        let Some(&current) = units.get(index) else {
            break;
        };
        if current == u16::from(b'%') {
            let is_long = units.get(index + 1).copied() == Some(u16::from(b'u'));
            if let Some(long) = read_hex(units, index + 2, 4).filter(|_| is_long) {
                out.push(long);
                index += 6;
                continue;
            }
            if let Some(short) = read_hex(units, index + 1, 2) {
                out.push(short);
                index += 3;
                continue;
            }
        }
        out.push(current);
        index += 1;
    }
    Ok(Value::Str(fitted(out, charge)))
}

fn read_hex(units: &[u16], start: usize, count: usize) -> Option<u16> {
    let mut value: u32 = 0;
    for offset in 0..count {
        let unit = *units.get(start + offset)?;
        let digit = char::from_u32(u32::from(unit))?.to_digit(16)?;
        value = value * 16 + digit;
    }
    u16::try_from(value).ok()
}

fn sextet(alphabet: &[u8], packed: u32) -> char {
    let index = (packed & 63) as usize;
    char::from(alphabet.get(index).copied().unwrap_or(b'A'))
}

const DOM_EXCEPTION: &str = "a DOMException, which the evaluator does not model";

fn native_btoa<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len().div_ceil(3).saturating_mul(4))?;
    let alphabet = BASE64_STANDARD.as_bytes();
    let mut bytes = Vec::with_capacity(text.len());
    for unit in text.units() {
        if *unit > 0xFF {
            return Err(throw_or_refuse(
                host,
                ErrorKind::TypeError,
                "btoa input is not a byte string",
                &DOM_EXCEPTION,
            ));
        }
        bytes.push(*unit as u8);
    }
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk.first().copied().map(u32::from).unwrap_or(0);
        let b1 = chunk.get(1).copied().map(u32::from).unwrap_or(0);
        let b2 = chunk.get(2).copied().map(u32::from).unwrap_or(0);
        let packed = (b0 << 16) | (b1 << 8) | b2;
        out.push(sextet(alphabet, packed >> 18));
        out.push(sextet(alphabet, packed >> 12));
        out.push(if chunk.len() > 1 {
            sextet(alphabet, packed >> 6)
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            sextet(alphabet, packed)
        } else {
            '='
        });
    }
    Ok(Value::Str(fitted(out.encode_utf16().collect(), charge)))
}

fn not_base64<'a>(host: &dyn Host<'a>) -> Abrupt<'a> {
    throw_or_refuse(
        host,
        ErrorKind::TypeError,
        "atob input is not base64",
        &DOM_EXCEPTION,
    )
}

fn native_atob<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = to_js_string(&argument(args, 0), host)?;
    let charge = check_string_growth(host, text.len())?;
    match atob_units(text.units()) {
        Some(out) => Ok(Value::Str(fitted(out, charge))),
        None => Err(not_base64(host)),
    }
}

fn atob_units(raw: &[u16]) -> Option<Vec<u16>> {
    decode_base64(without_base64_padding(raw)).or_else(|| {
        let compact: Vec<u16> = raw
            .iter()
            .copied()
            .filter(|unit| !is_ascii_whitespace(*unit))
            .collect();
        (compact.len() < raw.len())
            .then(|| decode_base64(without_base64_padding(&compact)))
            .flatten()
    })
}

fn without_base64_padding(units: &[u16]) -> &[u16] {
    if !units.len().is_multiple_of(4) {
        return units;
    }
    let equals = u16::from(b'=');
    let strip = units
        .iter()
        .rev()
        .take(2)
        .take_while(|unit| **unit == equals)
        .count();
    &units[..units.len() - strip]
}

fn decode_base64(units: &[u16]) -> Option<Vec<u16>> {
    if units.len() % 4 == 1 {
        return None;
    }
    let (quads, tail) = units.as_chunks::<4>();
    let mut out = vec![0_u16; quads.len() * 3 + tail.len().saturating_sub(1)];
    let (body, rest) = out.split_at_mut(quads.len() * 3);
    let mut bad = 0_u32;
    for (quad, bytes) in quads.iter().zip(body.as_chunks_mut::<3>().0) {
        let [a, b, c, d] = quad.map(base64_value);
        bad |= a | b | c | d;
        let packed = (a << 18) | (b << 12) | (c << 6) | d;
        *bytes = [16, 8, 0].map(|shift| ((packed >> shift) & 0xFF) as u16);
    }
    let mut packed = 0_u32;
    for &unit in tail {
        let value = base64_value(unit);
        bad |= value;
        packed = (packed << 6) | value;
    }
    packed <<= 6 * (4 - tail.len() as u32) % 24;
    for (index, byte) in rest.iter_mut().enumerate() {
        *byte = ((packed >> (16 - 8 * index as u32)) & 0xFF) as u16;
    }
    (bad & u32::from(NOT_BASE64) == 0).then_some(out)
}

fn base64_value(unit: u16) -> u32 {
    let low = BASE64_INDEX[usize::from(unit & 0xFF)];
    u32::from(low) | (u32::from(unit > 0xFF) << 7)
}

fn is_ascii_whitespace(unit: u16) -> bool {
    matches!(unit, 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

#[cfg(test)]
mod tests {
    use super::{atob_units, is_ascii_whitespace, BASE64_STANDARD};

    fn reference(text: &[u16]) -> Option<Vec<u16>> {
        let mut units: Vec<u16> = text
            .iter()
            .copied()
            .filter(|unit| !is_ascii_whitespace(*unit))
            .collect();
        if units.len().is_multiple_of(4) {
            for _ in 0..2 {
                if units.last() == Some(&u16::from(b'=')) {
                    units.pop();
                }
            }
        }
        if units.len() % 4 == 1 {
            return None;
        }
        let (mut accumulator, mut bits, mut out) = (0_u32, 0, Vec::new());
        for unit in units {
            let index = BASE64_STANDARD.find(char::from_u32(u32::from(unit))?)?;
            accumulator = (accumulator << 6) | index as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(((accumulator >> bits) & 0xFF) as u16);
            }
        }
        Some(out)
    }

    #[test]
    fn the_quad_decoder_equals_the_unit_at_a_time_decoder_on_every_short_text() {
        let alphabet: [u16; 8] = [65, 81, 47, 61, 32, 33, 0xE9, 0x100];
        let mut text = Vec::new();
        for length in 0..=6_u32 {
            for mut code in 0..8_usize.pow(length) {
                text.clear();
                for _ in 0..length {
                    text.push(alphabet[code % 8]);
                    code /= 8;
                }
                assert_eq!(atob_units(&text), reference(&text), "{text:?}");
            }
        }
    }
}
