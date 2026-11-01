use oxc_span::Span;
use rustc_hash::FxHashMap;
use untwist_eval::object::{Consulted, ProtoRead};
use untwist_eval::value::{IntrinsicProto, JsString, PropKey};

use crate::proof::{PatchKey, ProtoRoot, ProtoWrite};

fn key_matches_consulted(consulted: &Consulted, name: &str) -> bool {
    if name == "@@index" {
        return matches!(
            consulted,
            Consulted::Key(PropKey::Index(_)) | Consulted::AnyIndex | Consulted::Any
        );
    }
    match consulted {
        Consulted::Key(PropKey::Name(text)) => text.eq_str(name),
        Consulted::Key(PropKey::Index(index)) => {
            name.parse::<u32>().is_ok_and(|parsed| parsed == *index)
        }
        Consulted::AnyIndex => name.parse::<u32>().is_ok(),
        Consulted::Any => true,
    }
}

fn root_matches(read: IntrinsicProto, root: ProtoRoot) -> bool {
    matches!(
        (read, root),
        (_, ProtoRoot::Either)
            | (IntrinsicProto::Array, ProtoRoot::Array)
            | (IntrinsicProto::Object, ProtoRoot::Object)
            | (IntrinsicProto::Function, ProtoRoot::Function)
            | (IntrinsicProto::String, ProtoRoot::String)
            | (IntrinsicProto::Number, ProtoRoot::Number)
            | (IntrinsicProto::RegExp, ProtoRoot::RegExp)
    )
}

#[derive(Clone, Copy)]
enum WrittenKey<'k> {
    Name(&'k str),
    Text(&'k JsString),
    Any,
}

fn written_key<'k>(key: PatchKey<'k>, value_map: &'k FxHashMap<Span, JsString>) -> WrittenKey<'k> {
    match key {
        PatchKey::Named(name) => WrittenKey::Name(name),
        PatchKey::Pending(span) => value_map
            .get(&span)
            .map_or(WrittenKey::Any, WrittenKey::Text),
        PatchKey::Dynamic | PatchKey::Whole => WrittenKey::Any,
    }
}

fn text_matches_consulted(consulted: &Consulted, text: &JsString) -> bool {
    match consulted {
        Consulted::Key(PropKey::Name(name)) => name == text,
        Consulted::Key(PropKey::Index(index)) => text.eq_str(&index.to_string()),
        Consulted::AnyIndex => text
            .as_string()
            .is_none_or(|name| name.parse::<u32>().is_ok()),
        Consulted::Any => true,
    }
}

fn write_key_hits(key: WrittenKey<'_>, consulted: &Consulted) -> bool {
    match key {
        WrittenKey::Name(name) => key_matches_consulted(consulted, name),
        WrittenKey::Text(text) => text_matches_consulted(consulted, text),
        WrittenKey::Any => true,
    }
}

fn may_set_the_prototype(key: WrittenKey<'_>) -> bool {
    match key {
        WrittenKey::Name(name) => name == "__proto__",
        WrittenKey::Text(text) => text.eq_str("__proto__"),
        WrittenKey::Any => true,
    }
}

fn write_hits_read(write: &ProtoWrite<'_>, reads: &[ProtoRead], key: WrittenKey<'_>) -> bool {
    if may_set_the_prototype(key) {
        return true;
    }
    reads
        .iter()
        .any(|(read, consulted)| root_matches(*read, write.root) && write_key_hits(key, consulted))
}

pub(super) fn prototype_conflict<'w, 'a>(
    reads: &[ProtoRead],
    writes: &'w [ProtoWrite<'a>],
    value_map: &FxHashMap<Span, JsString>,
) -> Option<&'w ProtoWrite<'a>> {
    if reads.is_empty() {
        return None;
    }
    writes
        .iter()
        .find(|write| write_hits_read(write, reads, written_key(write.key, value_map)))
}

pub(super) fn describe_write(
    write: &ProtoWrite<'_>,
    value_map: &FxHashMap<Span, JsString>,
) -> String {
    let root = match write.root {
        ProtoRoot::Array => "Array.prototype",
        ProtoRoot::Object => "Object.prototype",
        ProtoRoot::Function => "Function.prototype",
        ProtoRoot::String => "String.prototype",
        ProtoRoot::Number => "Number.prototype",
        ProtoRoot::RegExp => "RegExp.prototype",
        ProtoRoot::Either => "a prototype",
    };
    match written_key(write.key, value_map) {
        WrittenKey::Name(name) => format!("{root}.{name}"),
        WrittenKey::Text(text) => match text.as_string() {
            Some(name) => format!("{root}.{name}"),
            None => format!("{root}[a key]"),
        },
        WrittenKey::Any => format!("{root}[a computed key]"),
    }
}
