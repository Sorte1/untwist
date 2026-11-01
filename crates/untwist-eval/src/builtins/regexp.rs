use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use regress::{backends, Flags, Regex};

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::budget::{REGEXP_STEPS_PER_FUEL, REGEXP_STEP_CAP};
use crate::builtins::args::{
    argument, charge_for_length, charge_for_scan, grow_object, live_array, live_slice,
    write_property,
};
use crate::builtins::{regexp_syntax, Globals, NativeFn, NativeTable, Realm};
use crate::host::Host;
use crate::ops::to_js_string;
use crate::unmodelled::throw_or_refuse;
use crate::value::{Abrupt, Eval, JsString, ObjKind, ObjRef, PropKey, RegExpData, Value};

pub const PATTERN_LEN_CAP: usize = 4 << 10;

pub struct RegExpMatch {
    pub range: Range<usize>,
    pub groups: Vec<Option<Range<usize>>>,
}

struct OffsetMap {
    byte_offsets: Vec<usize>,
    unit_offsets: Vec<usize>,
}

impl OffsetMap {
    fn new(text: &str) -> Self {
        let mut byte_offsets = Vec::new();
        let mut unit_offsets = Vec::new();
        let mut units = 0usize;
        for (byte, ch) in text.char_indices() {
            byte_offsets.push(byte);
            unit_offsets.push(units);
            units += ch.len_utf16();
        }
        byte_offsets.push(text.len());
        unit_offsets.push(units);
        Self {
            byte_offsets,
            unit_offsets,
        }
    }

    fn byte_at_unit(&self, unit: usize) -> Option<usize> {
        let index = self.unit_offsets.partition_point(|&at| at < unit);
        self.byte_offsets.get(index).copied()
    }

    fn unit_at_byte(&self, byte: usize) -> Option<usize> {
        let index = self.byte_offsets.binary_search(&byte).ok()?;
        self.unit_offsets.get(index).copied()
    }

    fn unit_range(&self, range: &Range<usize>) -> Option<Range<usize>> {
        Some(self.unit_at_byte(range.start)?..self.unit_at_byte(range.end)?)
    }
}

pub struct Subject {
    ascii: bool,
    text: String,
    map: OffsetMap,
    unit_len: usize,
    scan_charged: Cell<bool>,
}

impl Subject {
    pub fn new<'a>(host: &mut dyn Host<'a>, js: &JsString) -> Eval<'a, Self> {
        let units = js.units();
        if units.iter().any(|unit| (0xD800..=0xDFFF).contains(unit)) {
            return Err(Bail::Unsupported(
                &"a regular expression over surrogates, which the evaluator matches by code point",
            )
            .into());
        }
        let Ok(text) = String::from_utf16(units) else {
            return Err(Bail::Unsupported(&"regex over ill-formed UTF-16").into());
        };
        host.spend(1 + (units.len() as u64) / 64)?;
        let map = OffsetMap::new(&text);
        Ok(Self {
            ascii: units.iter().all(|unit| *unit < 0x80),
            text,
            map,
            unit_len: units.len(),
            scan_charged: Cell::new(false),
        })
    }
}

fn is_sticky(data: &RegExpData) -> bool {
    data.flags
        .as_string()
        .is_some_and(|text| text.contains('y'))
}

fn convert_range<'a>(map: &OffsetMap, range: &Range<usize>) -> Eval<'a, Range<usize>> {
    map.unit_range(range)
        .ok_or_else(|| Bail::Unsupported(&"regex match offset misaligned").into())
}

fn convert_groups<'a>(
    map: &OffsetMap,
    captures: &[Option<Range<usize>>],
) -> Eval<'a, Vec<Option<Range<usize>>>> {
    let mut groups = Vec::with_capacity(captures.len());
    for capture in captures {
        groups.push(match capture {
            Some(range) => Some(convert_range(map, range)?),
            None => None,
        });
    }
    Ok(groups)
}

fn rejected<'a>(host: &dyn Host<'a>, message: &str) -> Abrupt<'a> {
    throw_or_refuse(
        host,
        ErrorKind::SyntaxError,
        message,
        &"a pattern the evaluator's regular expression engine rejects",
    )
}

fn refuse_unmodelled_pattern(pattern: &str, flag_text: &str) -> EvalResult<bool> {
    let unicode = flag_text.contains('u') || flag_text.contains('v');
    if !unicode && pattern.contains("\\u{") {
        return Err(Bail::Unsupported(
            &"a braced unicode escape in a pattern without the u flag",
        ));
    }
    let ascii_subject_only = !unicode && flag_text.contains('i');
    if ascii_subject_only && !is_plain_ascii_pattern(pattern) {
        return Err(Bail::Unsupported(
            &"a case-insensitive pattern without the u flag that names a non-ASCII character",
        ));
    }
    if crate::object::decoding() {
        if !regexp_syntax::admitted(pattern, flag_text) {
            return Err(Bail::Unsupported(
                &"regular expression syntax outside the allowlist the evaluator admits while decoding",
            ));
        }
        return Ok(ascii_subject_only || unicode);
    }
    Ok(ascii_subject_only)
}

pub fn compile_regexp<'a>(
    host: &mut dyn Host<'a>,
    source: &JsString,
    flags: &JsString,
    proto: Option<ObjRef<'a>>,
) -> Eval<'a, Value<'a>> {
    if source.len() > PATTERN_LEN_CAP {
        return Err(Bail::Unsupported(&"regular expression pattern too long").into());
    }
    let (Some(pattern), Some(flag_text)) = (source.as_string(), flags.as_string()) else {
        return Err(rejected(host, "invalid regular expression"));
    };
    if !flags_are_valid(&flag_text) {
        return Err(rejected(host, "invalid regular expression flags"));
    }
    let ascii_subject_only = refuse_unmodelled_pattern(&pattern, &flag_text)?;
    let regex = match host.shared_regex(&pattern, &flag_text) {
        Some(regex) => regex,
        None => {
            let Ok(regex) = Regex::with_flags(&pattern, Flags::from(flag_text.as_str())) else {
                return Err(rejected(host, "invalid regular expression"));
            };
            host.share_regex(&pattern, &flag_text, &regex);
            regex
        }
    };
    let compiled = host.new_object(
        ObjKind::RegExp(Rc::new(RegExpData {
            source: source.clone(),
            flags: flags.clone(),
            regex,
            global: flag_text.contains('g'),
            ascii_subject_only,
            last_index: Cell::new(0),
        })),
        proto,
    );
    if let Some(obj) = compiled.as_object() {
        let units = (source.len() as u64).saturating_add(flags.len() as u64);
        grow_object(host, obj, units.saturating_mul(COMPILED_BYTES_PER_UNIT))?;
    }
    Ok(compiled)
}

fn is_plain_ascii_pattern(pattern: &str) -> bool {
    let bytes = pattern.as_bytes();
    pattern.is_ascii()
        && !bytes
            .windows(2)
            .any(|pair| pair[0] == b'\\' && matches!(pair[1], b'u' | b'x' | b'0'..=b'9' | b'c'))
}

fn flags_are_valid(text: &str) -> bool {
    let mut seen: Vec<char> = Vec::new();
    for flag in text.chars() {
        if !matches!(flag, 'd' | 'g' | 'i' | 'm' | 's' | 'u' | 'v' | 'y') || seen.contains(&flag) {
            return false;
        }
        seen.push(flag);
    }
    !(seen.contains(&'u') && seen.contains(&'v'))
}

pub fn regexp_data<'a>(value: &Value<'a>) -> Option<Rc<RegExpData>> {
    let obj = value.as_object()?;
    let ObjKind::RegExp(data) = &obj.borrow().kind else {
        return None;
    };
    Some(data.clone())
}

pub fn match_from<'a>(
    host: &mut dyn Host<'a>,
    subject: &Subject,
    data: &RegExpData,
    start: usize,
) -> Eval<'a, Option<RegExpMatch>> {
    if data.ascii_subject_only && !subject.ascii {
        return Err(Bail::Unsupported(
            &"a case-insensitive match without the u flag over a non-ASCII subject",
        )
        .into());
    }
    if start > subject.unit_len {
        return Ok(None);
    }
    if !subject.scan_charged.replace(true) {
        charge_for_scan(host, subject.unit_len, data.source.len())?;
    }
    let Some(byte_start) = subject.map.byte_at_unit(start) else {
        return Ok(None);
    };
    let search =
        backends::find_with_step_budget(&data.regex, &subject.text, byte_start, REGEXP_STEP_CAP);
    let charge = search.steps / REGEXP_STEPS_PER_FUEL;
    if charge > 0 {
        host.spend(charge as u64)?;
    }
    if search.exhausted {
        return Err(Bail::Fuel.into());
    }
    let Some(found) = search.found else {
        return Ok(None);
    };
    let range = convert_range(&subject.map, &found.range)?;
    let groups = convert_groups(&subject.map, &found.captures)?;
    Ok(Some(RegExpMatch { range, groups }))
}

pub fn match_at<'a>(
    host: &mut dyn Host<'a>,
    subject: &Subject,
    data: &RegExpData,
    start: usize,
) -> Eval<'a, Option<RegExpMatch>> {
    let Some(found) = match_from(host, subject, data, start)? else {
        return Ok(None);
    };
    if is_sticky(data) && found.range.start != start {
        return Ok(None);
    }
    Ok(Some(found))
}

pub fn slice(text: &JsString, range: &Range<usize>) -> JsString {
    let units = text.units();
    let start = range.start.min(units.len());
    let end = range.end.min(units.len()).max(start);
    JsString::from_units(units[start..end].to_vec())
}

pub(crate) fn live_range<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    range: &Range<usize>,
) -> EvalResult<JsString> {
    let len = text.len();
    let start = range.start.min(len);
    live_slice(host, text.units(), start, range.end.min(len))
}

pub fn exec_result<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    found: &RegExpMatch,
) -> Eval<'a, Value<'a>> {
    let mut items = vec![Value::Str(live_range(host, text, &found.range)?)];
    for group in &found.groups {
        items.push(match group {
            Some(range) => Value::Str(live_range(host, text, range)?),
            None => Value::Undefined,
        });
    }
    let result = live_array(host, items)?;
    if let Some(obj) = result.as_object() {
        write_property(
            host,
            obj,
            PropKey::Name(JsString::from_str("index")),
            Value::Number(found.range.start as f64),
        )?;
        write_property(
            host,
            obj,
            PropKey::Name(JsString::from_str("input")),
            Value::Str(text.clone()),
        )?;
        write_property(
            host,
            obj,
            PropKey::Name(JsString::from_str("groups")),
            Value::Undefined,
        )?;
    }
    Ok(result)
}

pub fn install<'a>(realm: &mut Realm<'a>, globals: &mut Globals<'a>, table: &mut NativeTable<'a>) {
    let members: [(&str, NativeFn<'a>); 3] = [
        ("test", native_test),
        ("exec", native_exec),
        ("toString", native_to_string),
    ];
    for (name, function) in members {
        let value = realm.native_value(table, ("RegExp.prototype", name), function);
        realm.define_regexp_member(name, value);
    }
    let constructor = realm.native_value(table, ("RegExp", ""), native_regexp_ctor);
    let id = constructor
        .as_object()
        .and_then(|obj| match obj.borrow().kind {
            ObjKind::Native(id, _) => Some(id),
            _ => None,
        });
    if let Some(id) = id {
        realm.mark_constructible(id);
    }
    globals.define("RegExp", constructor);
}

fn native_regexp_ctor<'a>(
    host: &mut dyn Host<'a>,
    _this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let existing = args.first().and_then(regexp_data);
    let source = match &existing {
        Some(data) => data.source.clone(),
        None => match args.first() {
            None | Some(Value::Undefined) => JsString::empty(),
            Some(value) => to_js_string(value, host)?,
        },
    };
    let flags = match args.get(1) {
        None | Some(Value::Undefined) => {
            existing.map_or_else(JsString::empty, |data| data.flags.clone())
        }
        Some(value) => to_js_string(value, host)?,
    };
    compile_regexp(host, &source, &flags, None)
}

fn advance(data: &RegExpData, found: &RegExpMatch) {
    if data.global || is_sticky(data) {
        data.last_index.set(found.range.end);
    }
}

fn native_test<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let Some(data) = regexp_data(&this) else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::TypeError,
            "not a RegExp",
            &"a generic receiver the evaluator does not model",
        ));
    };
    let text = to_js_string(&argument(args, 0), host)?;
    let start = if data.global || is_sticky(&data) {
        data.last_index.get()
    } else {
        0
    };
    let subject = Subject::new(host, &text)?;
    match match_at(host, &subject, &data, start)? {
        Some(found) => {
            advance(&data, &found);
            Ok(Value::Bool(true))
        }
        None => {
            data.last_index.set(0);
            Ok(Value::Bool(false))
        }
    }
}

fn native_exec<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let Some(data) = regexp_data(&this) else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::TypeError,
            "not a RegExp",
            &"a generic receiver the evaluator does not model",
        ));
    };
    let text = to_js_string(&argument(args, 0), host)?;
    let start = if data.global || is_sticky(&data) {
        data.last_index.get()
    } else {
        0
    };
    let subject = Subject::new(host, &text)?;
    let Some(found) = match_at(host, &subject, &data, start)? else {
        data.last_index.set(0);
        return Ok(Value::Null);
    };
    advance(&data, &found);
    exec_result(host, &text, &found)
}

fn native_to_string<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    _args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let Some(data) = regexp_data(&this) else {
        return Err(throw_or_refuse(
            host,
            ErrorKind::TypeError,
            "not a RegExp",
            &"a generic receiver the evaluator does not model",
        ));
    };
    charge_for_length(host, data.source.len() + data.flags.len())?;
    Ok(Value::Str(source_text(&data)))
}

pub fn source_text(data: &RegExpData) -> JsString {
    let mut units: Vec<u16> = vec![SLASH];
    if data.source.is_empty() {
        units.extend("(?:)".encode_utf16());
    } else {
        escape_source(data.source.units(), &mut units);
    }
    units.push(SLASH);
    for flag in FLAG_ORDER.encode_utf16() {
        if data.flags.units().contains(&flag) {
            units.push(flag);
        }
    }
    JsString::from_units(units)
}

const COMPILED_BYTES_PER_UNIT: u64 = 64;
const SLASH: u16 = b'/' as u16;
const BACKSLASH: u16 = b'\\' as u16;
const FLAG_ORDER: &str = "dgimsuvy";

fn line_terminator_escape(unit: u16) -> Option<&'static str> {
    match unit {
        0x0A => Some("n"),
        0x0D => Some("r"),
        0x2028 => Some("u2028"),
        0x2029 => Some("u2029"),
        _ => None,
    }
}

fn escape_source(source: &[u16], out: &mut Vec<u16>) {
    let mut in_class = false;
    let mut index = 0;
    while index < source.len() {
        let unit = source[index];
        index += 1;
        if unit == BACKSLASH && index < source.len() {
            let escaped = source[index];
            index += 1;
            out.push(BACKSLASH);
            match line_terminator_escape(escaped) {
                Some(text) => out.extend(text.encode_utf16()),
                None => out.push(escaped),
            }
            continue;
        }
        if let Some(text) = line_terminator_escape(unit) {
            out.push(BACKSLASH);
            out.extend(text.encode_utf16());
            continue;
        }
        if unit == SLASH && !in_class {
            out.push(BACKSLASH);
        } else if unit == u16::from(b'[') {
            in_class = true;
        } else if unit == u16::from(b']') {
            in_class = false;
        }
        out.push(unit);
    }
}
