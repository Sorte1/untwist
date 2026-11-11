use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::STRING_LEN_CAP;
use crate::builtins::args::{
    adopt_array, argument, charge_for_length, charge_for_scan, check_array_growth,
    check_string_growth, live_array, live_slice, live_text, reserve_array, reserve_text,
};
use crate::builtins::pattern_hooks::{note_is_regexp, note_pattern_hook, note_regexp_split, SPLIT};
use crate::builtins::regexp::regexp_data;
use crate::builtins::string_match::{self, split_with_regexp};
use crate::builtins::{define_builtin, Globals, NativeFn, NativeTable, Realm};
use crate::coerce::{is_js_whitespace_unit, to_integer_or_infinity, to_uint16, to_uint32};
use crate::host::Host;
use crate::ops::{to_js_string, to_number_value};
use crate::value::{Eval, JsString, Utf16Builder, Value};

pub fn clamp_index(raw: f64, len: usize) -> usize {
    let len_f = len as f64;
    let index = to_integer_or_infinity(raw);
    let resolved = if index < 0.0 { len_f + index } else { index };
    resolved.clamp(0.0, len_f) as usize
}

fn plain_clamp(raw: f64, len: usize) -> usize {
    to_integer_or_infinity(raw).clamp(0.0, len as f64) as usize
}

fn to_length(raw: f64) -> f64 {
    to_integer_or_infinity(raw).clamp(0.0, 9_007_199_254_740_991.0)
}

pub(crate) fn receiver<'a>(host: &mut dyn Host<'a>, this: &Value<'a>) -> Eval<'a, JsString> {
    if this.is_nullish() {
        return Err(host.throw(
            ErrorKind::TypeError,
            "String.prototype method called on null or undefined",
        ));
    }
    to_js_string(this, host)
}

fn number_argument<'a>(
    host: &mut dyn Host<'a>,
    args: &[Value<'a>],
    index: usize,
    fallback: f64,
) -> Eval<'a, f64> {
    match args.get(index) {
        None | Some(Value::Undefined) => Ok(fallback),
        Some(value) => to_number_value(value, host),
    }
}

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let from_char_code =
        realm.native_pure_value(table, ("String", "fromCharCode"), native_from_char_code);
    let string_global = realm.native_pure_value(table, ("String", ""), native_string_ctor);
    if let Some(obj) = string_global.as_object() {
        define_builtin(obj, "fromCharCode", from_char_code);
    }
    globals.define("String", string_global);

    let members: [(&str, NativeFn<'a>); 25] = [
        ("charAt", native_char_at),
        ("charCodeAt", native_char_code_at),
        ("codePointAt", native_code_point_at),
        ("indexOf", native_index_of),
        ("lastIndexOf", native_last_index_of),
        ("slice", native_slice),
        ("substring", native_substring),
        ("substr", native_substr),
        ("split", native_split),
        ("concat", native_concat),
        ("repeat", native_repeat),
        ("trim", native_trim),
        ("toUpperCase", native_to_upper),
        ("toLowerCase", native_to_lower),
        ("startsWith", native_starts_with),
        ("endsWith", native_ends_with),
        ("includes", native_includes),
        ("padStart", native_pad_start),
        ("padEnd", native_pad_end),
        ("replace", string_match::native_replace),
        ("replaceAll", string_match::native_replace_all),
        ("search", string_match::native_search),
        ("match", string_match::native_match),
        ("toString", native_to_string),
        ("valueOf", native_to_string),
    ];
    let impure_members = ["replace", "replaceAll", "search", "match"];
    for (name, function) in members {
        let value = realm.native_value(table, ("String.prototype", name), function);
        if !impure_members.contains(&name) {
            realm.mark_pure_value(&value);
        }
        realm.define_string_member(name, value);
    }
}

fn native_string_ctor<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    if args.is_empty() {
        return Ok(Value::Str(JsString::empty()));
    }
    Ok(Value::Str(to_js_string(&argument(args, 0), host)?))
}

fn native_from_char_code<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let charge = check_string_growth(host, args.len())?;
    let mut units = Vec::with_capacity(args.len());
    for value in args {
        units.push(to_uint16(to_number_value(value, host)?));
    }
    Ok(Value::Str(JsString::charged(units, charge)))
}

fn native_char_at<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let index = to_integer_or_infinity(number_argument(host, args, 0, 0.0)?);
    if index < 0.0 || index >= text.len() as f64 {
        return Ok(Value::Str(JsString::empty()));
    }
    let unit = text.code_unit_at(index as usize).unwrap_or(0);
    Ok(Value::Str(host.unit_string(unit)?))
}

fn native_char_code_at<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let index = to_integer_or_infinity(number_argument(host, args, 0, 0.0)?);
    if index < 0.0 || index >= text.len() as f64 {
        return Ok(Value::Number(f64::NAN));
    }
    match text.code_unit_at(index as usize) {
        Some(unit) => Ok(Value::Number(f64::from(unit))),
        None => Ok(Value::Number(f64::NAN)),
    }
}

fn native_code_point_at<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let pos = to_integer_or_infinity(number_argument(host, args, 0, 0.0)?);
    if pos < 0.0 || pos >= text.len() as f64 {
        return Ok(Value::Undefined);
    }
    let pos = pos as usize;
    let units = text.units();
    let Some(&first) = units.get(pos) else {
        return Ok(Value::Undefined);
    };
    if !(0xD800..=0xDBFF).contains(&first) || pos + 1 == text.len() {
        return Ok(Value::Number(f64::from(first)));
    }
    let Some(&second) = units.get(pos + 1) else {
        return Ok(Value::Number(f64::from(first)));
    };
    if !(0xDC00..=0xDFFF).contains(&second) {
        return Ok(Value::Number(f64::from(first)));
    }
    let code = (u32::from(first) - 0xD800) * 0x400 + (u32::from(second) - 0xDC00) + 0x10000;
    Ok(Value::Number(f64::from(code)))
}

pub(crate) fn find_units(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from.min(haystack.len()));
    }
    if needle.len() > haystack.len() {
        return None;
    }
    let (first, rest) = needle.split_first()?;
    let last_start = haystack.len() - needle.len();
    let candidates = haystack.get(from..=last_start)?;
    candidates
        .iter()
        .enumerate()
        .filter(|(_, unit)| *unit == first)
        .map(|(offset, _)| from + offset)
        .find(|start| haystack.get(start + 1..start + needle.len()) == Some(rest))
}

fn index_of_core<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Option<usize>> {
    let text = receiver(host, &this)?;
    let needle = to_js_string(&argument(args, 0), host)?;
    let from = plain_clamp(number_argument(host, args, 1, 0.0)?, text.len());
    charge_for_scan(host, text.len().saturating_sub(from), needle.len())?;
    Ok(find_units(text.units(), needle.units(), from))
}

fn native_index_of<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let found = index_of_core(host, this, args)?;
    Ok(Value::Number(found.map_or(-1.0, |index| index as f64)))
}

fn native_includes<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    note_is_regexp(host, &argument(args, 0))?;
    let found = index_of_core(host, this, args)?;
    Ok(Value::Bool(found.is_some()))
}

fn native_starts_with<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    note_is_regexp(host, &argument(args, 0))?;
    let search = to_js_string(&argument(args, 0), host)?;
    let pos = plain_clamp(number_argument(host, args, 1, 0.0)?, text.len());
    charge_for_length(host, search.len())?;
    let tail = text.units().get(pos..).unwrap_or(&[]);
    Ok(Value::Bool(tail.starts_with(search.units())))
}

fn native_ends_with<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    note_is_regexp(host, &argument(args, 0))?;
    let search = to_js_string(&argument(args, 0), host)?;
    let len = text.len();
    let end = plain_clamp(number_argument(host, args, 1, len as f64)?, len);
    charge_for_length(host, search.len())?;
    let Some(start) = end.checked_sub(search.len()) else {
        return Ok(Value::Bool(false));
    };
    let matched = text.units().get(start..end) == Some(search.units());
    Ok(Value::Bool(matched))
}

fn native_last_index_of<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let needle = to_js_string(&argument(args, 0), host)?;
    let raw_pos = to_number_value(&argument(args, 1), host)?;
    let pos = if raw_pos.is_nan() {
        f64::INFINITY
    } else {
        raw_pos
    };
    let len = text.len();
    let start = plain_clamp(pos, len);
    let bound = start.min(len.saturating_sub(needle.len()));
    charge_for_scan(host, len, needle.len())?;
    let mut found: f64 = -1.0;
    let mut cursor = 0;
    while cursor <= bound {
        let Some(index) = find_units(text.units(), needle.units(), cursor) else {
            break;
        };
        if index > bound {
            break;
        }
        found = index as f64;
        cursor = index + 1;
    }
    Ok(Value::Number(found))
}

fn slice_units<'a>(
    host: &mut dyn Host<'a>,
    units: &[u16],
    start: usize,
    end: usize,
) -> EvalResult<JsString> {
    if start >= end {
        return Ok(JsString::empty());
    }
    live_slice(host, units, start, end)
}

fn native_slice<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let len = text.len();
    let start = clamp_index(number_argument(host, args, 0, 0.0)?, len);
    let end = clamp_index(number_argument(host, args, 1, len as f64)?, len);
    charge_for_length(host, end.saturating_sub(start))?;
    Ok(Value::Str(slice_units(host, text.units(), start, end)?))
}

fn native_substring<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let len = text.len() as f64;
    let first = to_integer_or_infinity(number_argument(host, args, 0, 0.0)?).clamp(0.0, len);
    let second = to_integer_or_infinity(number_argument(host, args, 1, len)?).clamp(0.0, len);
    let (start, end) = if first <= second {
        (first, second)
    } else {
        (second, first)
    };
    charge_for_length(host, (end - start) as usize)?;
    Ok(Value::Str(slice_units(
        host,
        text.units(),
        start as usize,
        end as usize,
    )?))
}

fn native_substr<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let len = text.len();
    let start = clamp_index(number_argument(host, args, 0, 0.0)?, len);
    let count = to_integer_or_infinity(number_argument(host, args, 1, len as f64)?).max(0.0);
    let end = (start as f64 + count).min(len as f64) as usize;
    charge_for_length(host, end.saturating_sub(start))?;
    Ok(Value::Str(slice_units(host, text.units(), start, end)?))
}

fn native_split<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let separator_arg = argument(args, 0);
    let limit = match args.get(1) {
        None | Some(Value::Undefined) => u32::MAX,
        Some(value) => to_uint32(to_number_value(value, host)?),
    };
    note_pattern_hook(host, &separator_arg, SPLIT)?;
    let regexp = regexp_data(&separator_arg);
    if regexp.is_some() {
        note_regexp_split();
    }
    let separator = match &regexp {
        Some(_) => JsString::empty(),
        None => to_js_string(&separator_arg, host)?,
    };
    if limit == 0 {
        return Ok(host.new_array(Vec::new()));
    }
    if matches!(separator_arg, Value::Undefined) {
        return Ok(host.new_array(vec![Value::Str(text)]));
    }
    if let Some(data) = regexp {
        return split_with_regexp(host, &text, &data, limit as usize);
    }
    split_by_string(host, &text, &separator, limit as usize)
}

fn split_by_string<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    separator: &JsString,
    limit: usize,
) -> Eval<'a, Value<'a>> {
    charge_for_scan(host, text.len(), separator.len())?;
    let mut parts: Vec<Value<'a>> = Vec::new();
    if separator.is_empty() {
        return split_units(host, text, limit);
    }
    let mut cursor = 0;
    while parts.len() < limit {
        let Some(index) = find_units(text.units(), separator.units(), cursor) else {
            break;
        };
        check_array_growth(parts.len() + 1)?;
        parts.push(Value::Str(slice_units(host, text.units(), cursor, index)?));
        cursor = index + separator.len();
    }
    if parts.len() < limit {
        check_array_growth(parts.len() + 1)?;
        parts.push(Value::Str(slice_units(
            host,
            text.units(),
            cursor,
            text.len(),
        )?));
    }
    live_array(host, parts)
}

fn split_units<'a>(host: &mut dyn Host<'a>, text: &JsString, limit: usize) -> Eval<'a, Value<'a>> {
    let count = text.len().min(limit);
    check_array_growth(count)?;
    let reserved = reserve_array(host, count)?;
    let mut parts: Vec<Value<'a>> = Vec::with_capacity(count);
    for unit in text.units().iter().take(count) {
        parts.push(Value::Str(host.unit_string(*unit)?));
    }
    Ok(adopt_array(host, parts, reserved, Some(count))?)
}

fn native_concat<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let head = receiver(host, &this)?;
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    charge_for_length(host, head.len())?;
    out.push(&head)?;
    for value in args {
        let tail = to_js_string(value, host)?;
        charge_for_length(host, tail.len())?;
        out.push(&tail)?;
    }
    Ok(Value::Str(out.finish()))
}

fn native_repeat<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let count = to_integer_or_infinity(number_argument(host, args, 0, 0.0)?);
    if count < 0.0 || !count.is_finite() {
        return Err(host.throw(ErrorKind::RangeError, "invalid repeat count"));
    }
    if count > STRING_LEN_CAP as f64 {
        return Err(Bail::StringLength.into());
    }
    let count = count as usize;
    let total = text.len().saturating_mul(count);
    let charge = check_string_growth(host, total)?;
    host.spend(count as u64)?;
    let mut units = Vec::with_capacity(total);
    for _ in 0..count {
        units.extend_from_slice(text.units());
    }
    Ok(Value::Str(JsString::charged(units, charge)))
}

fn native_trim<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    charge_for_length(host, text.len())?;
    let units = text.units();
    let start = units
        .iter()
        .position(|unit| !is_js_whitespace_unit(*unit))
        .unwrap_or(units.len());
    let end = units
        .iter()
        .rposition(|unit| !is_js_whitespace_unit(*unit))
        .map_or(start, |index| index + 1);
    Ok(Value::Str(slice_units(host, units, start, end)?))
}

const NON_ASCII_CASE: &str =
    "case mapping of a non-ASCII character, which depends on the engine's Unicode version and context";

fn map_case<'a>(host: &mut dyn Host<'a>, this: Value<'a>, upper: bool) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    charge_for_length(host, text.len())?;
    let units = text.units();
    if units.iter().any(|unit| *unit >= 0x80) {
        return Err(Bail::Unsupported(&NON_ASCII_CASE).into());
    }
    let mapped = units
        .iter()
        .map(|unit| {
            let byte = *unit as u8;
            u16::from(if upper {
                byte.to_ascii_uppercase()
            } else {
                byte.to_ascii_lowercase()
            })
        })
        .collect();
    Ok(Value::Str(live_text(host, mapped)?))
}

fn native_to_upper<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    map_case(host, this, true)
}

fn native_to_lower<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    map_case(host, this, false)
}

fn pad<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
    at_start: bool,
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let len = text.len();
    let int_max = to_length(number_argument(host, args, 0, 0.0)?);
    if int_max <= len as f64 {
        return Ok(Value::Str(text));
    }
    let charge = check_string_growth(host, int_max as usize)?;
    let filler = match args.get(1) {
        None | Some(Value::Undefined) => JsString::from_str(" "),
        Some(value) => to_js_string(value, host)?,
    };
    if filler.is_empty() {
        return Ok(Value::Str(text));
    }
    let fill_len = int_max as usize - len;
    host.spend(fill_len as u64)?;
    let mut padding: Vec<u16> = Vec::with_capacity(fill_len);
    while padding.len() < fill_len {
        let remaining = fill_len - padding.len();
        let take = remaining.min(filler.len());
        padding.extend_from_slice(filler.units().get(..take).unwrap_or(&[]));
    }
    let mut units = Vec::with_capacity(int_max as usize);
    if at_start {
        units.extend_from_slice(&padding);
        units.extend_from_slice(text.units());
    } else {
        units.extend_from_slice(text.units());
        units.extend_from_slice(&padding);
    }
    Ok(Value::Str(JsString::charged(units, charge)))
}

fn native_pad_start<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    pad(host, this, args, true)
}

fn native_pad_end<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    pad(host, this, args, false)
}

fn native_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    Ok(Value::Str(receiver(host, &this)?))
}

#[cfg(test)]
mod tests {
    use super::find_units;

    type Case = (&'static [u16], &'static [u16], usize, Option<usize>);

    fn naive(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
        let from = from.min(haystack.len());
        (from..=haystack.len()).find(|start| haystack[*start..].starts_with(needle))
    }

    fn words(alphabet: &[u16], max_len: u32) -> Vec<Vec<u16>> {
        let mut all = vec![Vec::new()];
        let mut layer = vec![Vec::new()];
        for _ in 0..max_len {
            layer = layer
                .iter()
                .flat_map(|word: &Vec<u16>| {
                    alphabet.iter().map(move |unit| {
                        let mut next = word.clone();
                        next.push(*unit);
                        next
                    })
                })
                .collect();
            all.extend(layer.iter().cloned());
        }
        all
    }

    #[test]
    fn find_units_agrees_with_a_naive_search_over_every_small_input() {
        let haystacks = words(&[0x61, 0x62], 6);
        let needles = words(&[0x61, 0x62], 3);
        for haystack in &haystacks {
            for needle in &needles {
                for from in 0..9 {
                    assert_eq!(
                        find_units(haystack, needle, from),
                        naive(haystack, needle, from),
                        "haystack {haystack:?} needle {needle:?} from {from}"
                    );
                }
            }
        }
    }

    #[test]
    fn find_units_handles_the_edges_of_its_range() {
        let cases: [Case; 12] = [
            (&[], &[], 0, Some(0)),
            (&[], &[], 5, Some(0)),
            (&[], &[0x61], 0, None),
            (&[0x61], &[0x61, 0x61], 0, None),
            (&[0x61, 0x62], &[], 7, Some(2)),
            (&[0x61, 0x62], &[0x62], 2, None),
            (&[0x61, 0x62], &[0x62], usize::MAX, None),
            (&[0x61, 0x62, 0x63], &[0x61, 0x62, 0x63], 0, Some(0)),
            (&[0x61, 0x62, 0x63], &[0x61, 0x62, 0x63], 1, None),
            (&[0x61, 0x61, 0x61, 0x62], &[0x61, 0x61, 0x62], 0, Some(1)),
            (&[0xD83D, 0xDE00, 0xD83D], &[0xDE00, 0xD83D], 0, Some(1)),
            (&[0xD83D, 0xDE00], &[0xDE00], 1, Some(1)),
        ];
        for (haystack, needle, from, expected) in cases {
            assert_eq!(
                find_units(haystack, needle, from),
                expected,
                "haystack {haystack:?} needle {needle:?} from {from}"
            );
            assert_eq!(naive(haystack, needle, from), expected);
        }
    }
}
