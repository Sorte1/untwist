use crate::host::Host;
use crate::object::{
    decoding, lookup_property, note_intrinsic, note_intrinsic_constructor, recording,
};
use crate::value::{Eval, IntrinsicProto, JsString, PropKey, Value};

const MATCH: &str = "@@match";
pub(crate) const REPLACE: &str = "@@replace";
pub(crate) const SEARCH: &str = "@@search";
pub(crate) const SPLIT: &str = "@@split";

const EXEC: &str = "exec";
const SOURCE: &str = "source";
const FLAG_GETTERS: [&str; 9] = [
    "flags",
    "hasIndices",
    "global",
    "ignoreCase",
    "multiline",
    "dotAll",
    "unicode",
    "unicodeSets",
    "sticky",
];

fn name_key(name: &str) -> PropKey {
    PropKey::Name(JsString::from_str(name))
}

fn note_regexp_members(names: &[&str]) {
    if !recording() {
        return;
    }
    for name in names {
        note_intrinsic(IntrinsicProto::RegExp, &name_key(name));
    }
}

fn primitive_roots(pattern: &Value<'_>) -> &'static [IntrinsicProto] {
    match pattern {
        Value::Str(_) => &[IntrinsicProto::String, IntrinsicProto::Object],
        Value::Number(_) => &[IntrinsicProto::Number, IntrinsicProto::Object],
        Value::Bool(_) => &[IntrinsicProto::Object],
        Value::Undefined | Value::Null | Value::Object(_) => &[],
    }
}

fn note_primitive_hook(pattern: &Value<'_>, key: &PropKey) {
    for root in primitive_roots(pattern) {
        note_intrinsic(*root, key);
    }
}

pub(crate) fn note_pattern_hook<'a>(
    host: &mut dyn Host<'a>,
    pattern: &Value<'a>,
    hook: &str,
) -> Eval<'a, ()> {
    if !decoding() {
        return Ok(());
    }
    let key = name_key(hook);
    if let Some(object) = pattern.as_object() {
        if lookup_property(object, &key)?.is_none() {
            host.note_protocol_member(pattern, &key)?;
        }
    }
    note_primitive_hook(pattern, &key);
    Ok(())
}

pub(crate) fn note_primitive_match(pattern: &Value<'_>) {
    if decoding() {
        note_primitive_hook(pattern, &name_key(MATCH));
    }
}

pub(crate) fn note_is_regexp<'a>(host: &mut dyn Host<'a>, value: &Value<'a>) -> Eval<'a, ()> {
    if value.as_object().is_some() {
        note_pattern_hook(host, value, MATCH)?;
    }
    Ok(())
}

pub(crate) fn note_exec() {
    note_regexp_members(&[EXEC]);
}

pub(crate) fn note_flags_and_exec() {
    note_regexp_members(&FLAG_GETTERS);
    note_regexp_members(&[EXEC]);
}

pub(crate) fn note_regexp_search() {
    note_regexp_members(&[SEARCH, EXEC]);
}

pub(crate) fn note_regexp_match() {
    note_regexp_members(&[MATCH]);
    note_flags_and_exec();
}

pub(crate) fn note_regexp_split() {
    note_intrinsic_constructor(IntrinsicProto::RegExp);
    note_regexp_members(&[MATCH]);
    note_flags_and_exec();
}

pub(crate) fn note_regexp_text() {
    note_regexp_members(&[SOURCE]);
    note_regexp_members(&FLAG_GETTERS);
}
