use std::ops::Range;
use std::rc::Rc;

use crate::bail::ErrorKind;
use crate::bail::{Bail, EvalResult};
use crate::builtins::args::check_array_growth;
use crate::builtins::args::{
    argument, charge_for_length, heap_checkpoint, live_array, reserve_text,
};
use crate::builtins::pattern_hooks::{
    note_flags_and_exec, note_is_regexp, note_pattern_hook, note_primitive_match,
    note_regexp_match, note_regexp_search, REPLACE, SEARCH,
};
use crate::builtins::regexp::{
    compile_regexp, exec_result, is_sticky, live_range, match_at, match_from, regexp_data, slice,
    RegExpMatch, Subject,
};
use crate::builtins::string::{find_units, receiver};
use crate::host::Host;
use crate::ops::to_js_string;
use crate::value::{Eval, JsString, PropKey, RegExpData, Utf16Builder, Value};

enum Matcher {
    Regex {
        data: Rc<RegExpData>,
        subject: Subject,
    },
    Literal(JsString),
}

fn pattern_regexp<'a>(host: &mut dyn Host<'a>, pattern: &Value<'a>) -> Eval<'a, Rc<RegExpData>> {
    if let Some(data) = regexp_data(pattern) {
        return Ok(data);
    }
    let source = match pattern {
        Value::Undefined => JsString::empty(),
        other => to_js_string(other, host)?,
    };
    let compiled = compile_regexp(host, &source, &JsString::empty(), None)?;
    match regexp_data(&compiled) {
        Some(data) => Ok(data),
        None => Err(Bail::Unsupported(&"a pattern that did not compile to a RegExp").into()),
    }
}

fn regexp_matcher<'a>(
    host: &mut dyn Host<'a>,
    pattern: &Value<'a>,
    text: &JsString,
) -> Eval<'a, Matcher> {
    let data = pattern_regexp(host, pattern)?;
    let subject = Subject::new(host, text)?;
    Ok(Matcher::Regex { data, subject })
}

fn resolve_matcher<'a>(
    host: &mut dyn Host<'a>,
    pattern: &Value<'a>,
    text: &JsString,
) -> Eval<'a, Matcher> {
    match regexp_data(pattern) {
        Some(data) => {
            let subject = Subject::new(host, text)?;
            Ok(Matcher::Regex { data, subject })
        }
        None => Ok(Matcher::Literal(to_js_string(pattern, host)?)),
    }
}

fn next_match<'a>(
    host: &mut dyn Host<'a>,
    matcher: &Matcher,
    text: &JsString,
    from: usize,
) -> Eval<'a, Option<RegExpMatch>> {
    match matcher {
        Matcher::Regex { data, subject } => match_at(host, subject, data, from),
        Matcher::Literal(needle) => Ok(find_units(text.units(), needle.units(), from).map(
            |start| RegExpMatch {
                range: start..start + needle.len(),
                groups: Vec::new(),
            },
        )),
    }
}

fn restart(matcher: &Matcher) {
    if let Matcher::Regex { data, .. } = matcher {
        data.last_index.set(0);
    }
}

fn single_match<'a>(
    host: &mut dyn Host<'a>,
    matcher: &Matcher,
    text: &JsString,
) -> Eval<'a, Option<RegExpMatch>> {
    let sticky = match matcher {
        Matcher::Regex { data, .. } if is_sticky(data) => data,
        _ => return next_match(host, matcher, text, 0),
    };
    let found = next_match(host, matcher, text, sticky.last_index.get())?;
    sticky
        .last_index
        .set(found.as_ref().map_or(0, |found| found.range.end));
    Ok(found)
}

const DOLLAR: u16 = b'$' as u16;

fn digit_at(units: &[u16], index: usize) -> Option<usize> {
    let unit = *units.get(index)?;
    (0x30..=0x39)
        .contains(&unit)
        .then(|| usize::from(unit - 0x30))
}

fn capture_reference(units: &[u16], at: usize, captures: usize) -> Option<(usize, usize)> {
    let first = digit_at(units, at)?;
    if let Some(second) = digit_at(units, at + 1) {
        let index = first * 10 + second;
        if index <= captures {
            return Some((index, 2));
        }
    }
    Some((first, 1))
}

fn expand_replacement(
    out: &mut Utf16Builder,
    template: &JsString,
    text: &JsString,
    found: &RegExpMatch,
) -> EvalResult<()> {
    let units = template.units();
    let mut index = 0;
    while index < units.len() {
        let literal = units[index..]
            .iter()
            .position(|&unit| unit == DOLLAR)
            .map_or(units.len(), |at| index + at);
        out.push_units(&units[index..literal])?;
        index = literal;
        if index + 1 >= units.len() {
            out.push_units(&units[index..])?;
            break;
        }
        index += expand_reference(out, units, index, text, found)?;
    }
    Ok(())
}

fn expand_reference(
    out: &mut Utf16Builder,
    units: &[u16],
    index: usize,
    text: &JsString,
    found: &RegExpMatch,
) -> EvalResult<usize> {
    let next = units[index + 1];
    if next == DOLLAR {
        out.push_units(&[DOLLAR])?;
        return Ok(2);
    }
    let range = match next {
        b if b == u16::from(b'&') => found.range.clone(),
        b if b == u16::from(b'`') => 0..found.range.start,
        b if b == u16::from(b'\'') => found.range.end..text.len(),
        _ => return expand_group(out, units, index, text, found),
    };
    push_range(out, text, &range)?;
    Ok(2)
}

fn expand_group(
    out: &mut Utf16Builder,
    units: &[u16],
    index: usize,
    text: &JsString,
    found: &RegExpMatch,
) -> EvalResult<usize> {
    let Some((group, width)) = capture_reference(units, index + 1, found.groups.len()) else {
        out.push_units(&[DOLLAR])?;
        return Ok(1);
    };
    match group.checked_sub(1).and_then(|at| found.groups.get(at)) {
        Some(Some(range)) => push_range(out, text, range)?,
        Some(None) => {}
        None => out.push_units(&units[index..index + 1 + width])?,
    }
    Ok(1 + width)
}

fn push_range(out: &mut Utf16Builder, text: &JsString, range: &Range<usize>) -> EvalResult<()> {
    let units = text.units();
    let start = range.start.min(units.len());
    let end = range.end.min(units.len()).max(start);
    out.push_units(&units[start..end])
}

fn callback_arguments<'a>(text: &JsString, found: &RegExpMatch) -> Vec<Value<'a>> {
    let mut call_args = vec![Value::Str(slice(text, &found.range))];
    for group in &found.groups {
        call_args.push(match group {
            Some(range) => Value::Str(slice(text, range)),
            None => Value::Undefined,
        });
    }
    call_args.push(Value::Number(found.range.start as f64));
    call_args.push(Value::Str(text.clone()));
    call_args
}

fn replace_impl<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
    force_global: bool,
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let raw_pattern = argument(args, 0);
    let replacement = argument(args, 1);
    if force_global {
        note_is_regexp(host, &raw_pattern)?;
    }
    note_pattern_hook(host, &raw_pattern, REPLACE)?;
    let existing = regexp_data(&raw_pattern);
    if existing.is_some() {
        note_flags_and_exec();
    }
    if force_global && existing.as_ref().is_some_and(|data| !data.global) {
        return Err(host.throw(
            ErrorKind::TypeError,
            "replaceAll must be called with a global RegExp",
        ));
    }
    let global = force_global || existing.is_some_and(|data| data.global);
    let matcher = resolve_matcher(host, &raw_pattern, &text)?;
    let template = if replacement.is_callable() {
        JsString::empty()
    } else {
        to_js_string(&replacement, host)?
    };
    replace_matches(host, &text, &matcher, &replacement, &template, global)
}

fn replace_matches<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    matcher: &Matcher,
    replacement: &Value<'a>,
    template: &JsString,
    global: bool,
) -> Eval<'a, Value<'a>> {
    let mut out = Utf16Builder::charged(reserve_text(host, 0)?);
    let mut cursor = 0;
    let mut copied = 0;
    if global {
        restart(matcher);
    }
    loop {
        let found = match global {
            true => next_match(host, matcher, text, cursor)?,
            false => single_match(host, matcher, text)?,
        };
        let Some(found) = found else {
            break;
        };
        host.spend(1)?;
        let head = slice(text, &(copied..found.range.start));
        charge_for_length(host, head.len())?;
        out.push(&head)?;
        let before = out.len();
        replacement_piece(host, &mut out, text, &found, replacement, template)?;
        charge_for_length(host, out.len() - before)?;
        copied = found.range.end;
        cursor = found.range.end.max(found.range.start + 1);
        if !global || cursor > text.len() {
            break;
        }
    }
    let tail: Range<usize> = copied.min(text.len())..text.len();
    let rest = slice(text, &tail);
    charge_for_length(host, rest.len())?;
    out.push(&rest)?;
    Ok(Value::Str(out.finish()))
}

fn replacement_piece<'a>(
    host: &mut dyn Host<'a>,
    out: &mut Utf16Builder,
    text: &JsString,
    found: &RegExpMatch,
    replacement: &Value<'a>,
    template: &JsString,
) -> Eval<'a, ()> {
    if replacement.is_callable() {
        let produced = host.call_value(
            replacement,
            Value::Undefined,
            &callback_arguments(text, found),
        )?;
        return Ok(out.push(&to_js_string(&produced, host)?)?);
    }
    Ok(expand_replacement(out, template, text, found)?)
}

pub(crate) fn native_replace<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    replace_impl(host, this, args, false)
}

pub(crate) fn native_replace_all<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    replace_impl(host, this, args, true)
}

pub(crate) fn native_search<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let raw_pattern = argument(args, 0);
    note_pattern_hook(host, &raw_pattern, SEARCH)?;
    note_regexp_search();
    let matcher = regexp_matcher(host, &raw_pattern, &text)?;
    match next_match(host, &matcher, &text, 0)? {
        Some(found) => Ok(Value::Number(found.range.start as f64)),
        None => Ok(Value::Number(-1.0)),
    }
}

fn single_match_result<'a>(
    host: &mut dyn Host<'a>,
    matcher: &Matcher,
    text: &JsString,
) -> Eval<'a, Value<'a>> {
    let Some(found) = single_match(host, matcher, text)? else {
        return Ok(Value::Null);
    };
    exec_result(host, text, &found)
}

fn match_all<'a>(
    host: &mut dyn Host<'a>,
    matcher: &Matcher,
    text: &JsString,
) -> Eval<'a, Value<'a>> {
    let mut items: Vec<Value<'a>> = Vec::new();
    let mut cursor = 0;
    while let Some(found) = next_match(host, matcher, text, cursor)? {
        host.spend(1)?;
        check_array_growth(items.len() + 1)?;
        heap_checkpoint(host, items.len())?;
        items.push(Value::Str(live_range(host, text, &found.range)?));
        cursor = found.range.end.max(found.range.start + 1);
        if cursor > text.len() {
            break;
        }
    }
    if items.is_empty() {
        return Ok(Value::Null);
    }
    live_array(host, items)
}

pub(crate) fn native_match<'a>(
    host: &mut dyn Host<'a>,
    this: Value<'a>,
    args: &[Value<'a>],
) -> Eval<'a, Value<'a>> {
    let text = receiver(host, &this)?;
    let raw_pattern = argument(args, 0);
    if crate::object::decoding() && raw_pattern.as_object().is_some() {
        let marker = PropKey::Name(JsString::from_str("@@match"));
        host.get_property(&raw_pattern, &marker)?;
    }
    note_primitive_match(&raw_pattern);
    note_regexp_match();
    let is_global = regexp_data(&raw_pattern).is_some_and(|data| data.global);
    let matcher = regexp_matcher(host, &raw_pattern, &text)?;
    if is_global {
        restart(&matcher);
        return match_all(host, &matcher, &text);
    }
    single_match_result(host, &matcher, &text)
}

pub(crate) fn split_with_regexp<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    data: &RegExpData,
    limit: usize,
) -> Eval<'a, Value<'a>> {
    let subject = Subject::new(host, text)?;
    let size = text.len();
    if size == 0 {
        if match_from(host, &subject, data, 0)?.is_some() {
            return Ok(host.new_array(Vec::new()));
        }
        return Ok(host.new_array(vec![Value::Str(text.clone())]));
    }
    let mut parts: Vec<Value<'a>> = Vec::new();
    let mut start = 0usize;
    let mut cursor = 0usize;
    while cursor < size {
        host.spend(1)?;
        heap_checkpoint(host, parts.len())?;
        let Some(found) = match_from(host, &subject, data, cursor)? else {
            break;
        };
        if found.range.start >= size {
            break;
        }
        let end = found.range.end.min(size);
        if end == start {
            cursor = found.range.start + 1;
            continue;
        }
        let head = live_range(host, text, &(start..found.range.start))?;
        push_part(&mut parts, Value::Str(head))?;
        if parts.len() == limit {
            return live_array(host, parts);
        }
        for group in &found.groups {
            let value = group_value(host, text, group)?;
            push_part(&mut parts, value)?;
            if parts.len() == limit {
                return live_array(host, parts);
            }
        }
        start = end;
        cursor = end;
    }
    let rest = live_range(host, text, &(start..size))?;
    push_part(&mut parts, Value::Str(rest))?;
    live_array(host, parts)
}

fn push_part<'a>(parts: &mut Vec<Value<'a>>, value: Value<'a>) -> Eval<'a, ()> {
    check_array_growth(parts.len() + 1)?;
    parts.push(value);
    Ok(())
}

fn group_value<'a>(
    host: &mut dyn Host<'a>,
    text: &JsString,
    group: &Option<Range<usize>>,
) -> EvalResult<Value<'a>> {
    Ok(match group {
        Some(range) => Value::Str(live_range(host, text, range)?),
        None => Value::Undefined,
    })
}
