use oxc_ast::ast::*;
use untwist_core::ast_util::prop::member_parts;

use crate::generate::{lower_camel, strip_accessor_prefix};
use crate::human::looks_human_written;

const NAMED_RESULTS: [(&str, &str); 37] = [
    ("map", "mapped"),
    ("filter", "filtered"),
    ("sort", "sorted"),
    ("reduce", "reduced"),
    ("join", "joined"),
    ("find", "found"),
    ("split", "parts"),
    ("keys", "keys"),
    ("values", "values"),
    ("entries", "entries"),
    ("getBoundingClientRect", "rect"),
    ("getAttribute", "attr"),
    ("querySelectorAll", "elements"),
    ("getElementsByClassName", "elements"),
    ("getElementsByTagName", "elements"),
    ("now", "now"),
    ("stringify", "json"),
    ("indexOf", "index"),
    ("lastIndexOf", "index"),
    ("findIndex", "index"),
    ("getTime", "time"),
    ("charCodeAt", "code"),
    ("codePointAt", "code"),
    ("match", "match"),
    ("exec", "match"),
    ("getImageData", "imageData"),
    ("toDataURL", "dataUrl"),
    ("createObjectURL", "url"),
    ("measureText", "metrics"),
    ("createLinearGradient", "gradient"),
    ("createRadialGradient", "gradient"),
    ("createBufferSource", "source"),
    ("createMediaElementSource", "source"),
    ("createBiquadFilter", "filter"),
    ("getUniformLocation", "uniform"),
    ("getAttribLocation", "attrib"),
    ("cloneNode", "clone"),
];

const NAMED_GLOBAL_CALLS: [(&str, &str); 4] = [
    ("fetch", "response"),
    ("setTimeout", "timer"),
    ("setInterval", "timer"),
    ("requestAnimationFrame", "frameId"),
];

const NAMED_CONSTRUCTIONS: [(&str, &str); 28] = [
    ("WebSocket", "socket"),
    ("XMLHttpRequest", "xhr"),
    ("Float32Array", "floats"),
    ("Float64Array", "doubles"),
    ("Int8Array", "bytes"),
    ("Uint8Array", "bytes"),
    ("Uint8ClampedArray", "bytes"),
    ("Int16Array", "shorts"),
    ("Uint16Array", "shorts"),
    ("Int32Array", "ints"),
    ("Uint32Array", "ints"),
    ("ArrayBuffer", "buffer"),
    ("DataView", "view"),
    ("AudioContext", "audioCtx"),
    ("webkitAudioContext", "audioCtx"),
    ("Image", "img"),
    ("TextDecoder", "decoder"),
    ("TextEncoder", "encoder"),
    ("FileReader", "reader"),
    ("MutationObserver", "observer"),
    ("ResizeObserver", "observer"),
    ("IntersectionObserver", "observer"),
    ("AbortController", "controller"),
    ("URLSearchParams", "params"),
    ("RegExp", "re"),
    ("Object", "obj"),
    ("Array", "arr"),
    ("Function", "fn"),
];

const WEBGL_CONTEXTS: [&str; 3] = ["webgl", "webgl2", "experimental-webgl"];

const STORAGES: [&str; 2] = ["localStorage", "sessionStorage"];

const UNNAMING_METHODS: [&str; 24] = [
    "call", "apply", "bind", "then", "catch", "finally", "push", "pop", "shift", "unshift",
    "slice", "splice", "concat", "replace", "toString", "valueOf", "set", "get", "add", "has",
    "emit", "log", "assign", "create",
];

const IGNORED_PROPERTIES: [&str; 4] = ["prototype", "exports", "default", "__proto__"];

const MAX_KEY_LENGTH: usize = 32;

const MIN_KEY_LENGTH: usize = 3;

const MAX_TAG_LENGTH: usize = 24;

const MIN_SEGMENT: usize = 2;

pub fn is_word_like(name: &str) -> bool {
    let mut segments: Vec<String> = Vec::new();
    for ch in name.chars() {
        if !ch.is_ascii_alphabetic() {
            return false;
        }
        match segments.last_mut() {
            Some(segment) if !ch.is_ascii_uppercase() => segment.push(ch),
            _ => segments.push(ch.to_string()),
        }
    }
    let Some(first) = segments.first() else {
        return false;
    };
    first.starts_with(|ch: char| ch.is_ascii_lowercase())
        && segments.iter().all(|segment| is_word_segment(segment))
}

fn is_word_segment(segment: &str) -> bool {
    segment.len() >= MIN_SEGMENT
        && segment
            .chars()
            .any(|ch| matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))
}

pub fn is_naming_property(name: &str) -> bool {
    !IGNORED_PROPERTIES.contains(&name)
        && looks_human_written(name)
        && is_word_like(&lower_camel(name))
}

pub fn key_name(key: &str) -> Option<String> {
    let length = key.chars().count();
    if !(MIN_KEY_LENGTH..=MAX_KEY_LENGTH).contains(&length) {
        return None;
    }
    let mut out = String::with_capacity(key.len());
    let mut upper_next = false;
    for ch in key.chars() {
        if ch == '-' || ch == '_' {
            upper_next = !out.is_empty();
            continue;
        }
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        if upper_next {
            out.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    let name = lower_camel(&out);
    is_word_like(&name).then_some(name)
}

pub fn tag_name(tag: &str) -> Option<String> {
    let mut chars = tag.chars();
    let first = chars.next()?;
    let plain = first.is_ascii_lowercase()
        && tag.len() <= MAX_TAG_LENGTH
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit());
    plain.then(|| tag.to_string())
}

fn selector_name(selector: &str) -> Option<String> {
    if let Some(id) = selector.strip_prefix('#') {
        return key_name(id);
    }
    if let Some(class) = selector.strip_prefix('.') {
        return key_name(class);
    }
    tag_name(selector)
}

pub fn method_name<'k>(callee: &'k Expression<'_>) -> Option<std::borrow::Cow<'k, str>> {
    member_parts(callee.without_parentheses()).map(|(_, name)| name)
}

pub fn global_callee<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub fn string_argument<'k>(arguments: &'k [Argument<'_>]) -> Option<&'k str> {
    match arguments.first()? {
        Argument::StringLiteral(literal) => literal.value.as_str(),
        _ => None,
    }
}

fn is_named_object(callee: &Expression<'_>, names: &[&str]) -> bool {
    member_parts(callee.without_parentheses()).is_some_and(|(object, _)| {
        match object.without_parentheses() {
            Expression::Identifier(id) => names.contains(&id.name.as_str()),
            Expression::StaticMemberExpression(member) => {
                names.contains(&member.property.name.as_str())
            }
            _ => false,
        }
    })
}

pub fn is_storage_call(call: &CallExpression<'_>, method: &str) -> bool {
    method_name(&call.callee).is_some_and(|name| name == method)
        && is_named_object(&call.callee, &STORAGES)
}

fn context_name(arguments: &[Argument<'_>]) -> Option<String> {
    let kind = string_argument(arguments)?;
    if WEBGL_CONTEXTS.contains(&kind) {
        return Some("gl".to_string());
    }
    Some("ctx".to_string())
}

fn lookup_name(method: &str, arguments: &[Argument<'_>]) -> Option<Option<String>> {
    let found = match method {
        "createElement" => string_argument(arguments).and_then(tag_name),
        "getElementById" => string_argument(arguments).and_then(key_name),
        "querySelector" => string_argument(arguments).and_then(selector_name),
        "getContext" => context_name(arguments),
        _ => return None,
    };
    Some(found)
}

pub fn call_result_name(call: &CallExpression<'_>) -> Option<String> {
    if let Some(name) = global_callee(&call.callee) {
        return NAMED_GLOBAL_CALLS
            .iter()
            .find(|(callee, _)| *callee == name)
            .map(|(_, result)| result.to_string());
    }
    let method = method_name(&call.callee)?;
    if let Some(found) = lookup_name(&method, &call.arguments) {
        return found;
    }
    if method == "parse" && is_named_object(&call.callee, &["JSON"]) {
        return Some("data".to_string());
    }
    if method == "getItem" && is_named_object(&call.callee, &STORAGES) {
        return string_argument(&call.arguments).and_then(key_name);
    }
    NAMED_RESULTS
        .iter()
        .find(|(name, _)| *name == method)
        .map(|(_, result)| result.to_string())
}

fn constructor_name<'k>(callee: &'k Expression<'_>) -> Option<&'k str> {
    match callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

pub fn named_construction(new: &NewExpression<'_>) -> Option<String> {
    let name = constructor_name(&new.callee)?;
    NAMED_CONSTRUCTIONS
        .iter()
        .find(|(constructor, _)| *constructor == name)
        .map(|(_, result)| result.to_string())
}

fn construction_name(new: &NewExpression<'_>) -> Option<String> {
    if let Some(name) = named_construction(new) {
        return Some(name);
    }
    let name = constructor_name(&new.callee)?;
    if name.ends_with("Error") {
        return Some("err".to_string());
    }
    let capitalised = name.starts_with(|ch: char| ch.is_ascii_uppercase());
    let stem = name.trim_end_matches(|ch: char| ch.is_ascii_digit());
    let usable = capitalised && looks_human_written(name) && is_word_like(&lower_camel(stem));
    usable.then(|| lower_camel(name))
}

fn method_result_name(call: &CallExpression<'_>) -> Option<String> {
    if let Some(name) = call_result_name(call) {
        return Some(name);
    }
    let method = method_name(&call.callee)?;
    if UNNAMING_METHODS.contains(&method.as_ref()) || !is_naming_property(&method) {
        return None;
    }
    strip_accessor_prefix(&method)
        .or_else(|| Some(lower_camel(&method)))
        .filter(|name| is_word_like(name))
}

fn singular(plural: &str) -> Option<String> {
    let stem = if let Some(stem) = plural.strip_suffix("ies") {
        format!("{stem}y")
    } else if ["sses", "shes", "ches", "xes"]
        .iter()
        .any(|end| plural.ends_with(end))
    {
        plural[..plural.len() - 2].to_string()
    } else if plural.ends_with("ss") || plural.ends_with("us") || plural.ends_with("is") {
        return None;
    } else {
        plural.strip_suffix('s')?.to_string()
    };
    (stem.len() >= MIN_KEY_LENGTH && is_word_like(&stem)).then_some(stem)
}

fn collection_name<'k>(collection: &'k Expression<'_>) -> Option<&'k str> {
    match collection.without_parentheses() {
        Expression::Identifier(id) if looks_human_written(&id.name) => Some(id.name.as_str()),
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

pub fn element_name(collection: &Expression<'_>) -> Option<String> {
    let name = collection_name(collection)?;
    if !is_naming_property(name) {
        return None;
    }
    singular(&lower_camel(name))
}

fn indexed_element_name(member: &ComputedMemberExpression<'_>) -> Option<String> {
    if matches!(member.expression, Expression::StringLiteral(_)) {
        return None;
    }
    element_name(&member.object)
}

pub fn declared_value_name(init: &Expression<'_>) -> Option<String> {
    match init.without_parentheses() {
        Expression::ComputedMemberExpression(member) => indexed_element_name(member),
        Expression::CallExpression(call) => call_result_name(call),
        Expression::NewExpression(new) => named_construction(new),
        Expression::AwaitExpression(awaited) => declared_value_name(&awaited.argument),
        _ => None,
    }
}

pub fn assigned_value_name(value: &Expression<'_>) -> Option<String> {
    match value.without_parentheses() {
        Expression::CallExpression(call) => method_result_name(call),
        Expression::NewExpression(new) => construction_name(new),
        Expression::AwaitExpression(awaited) => assigned_value_name(&awaited.argument),
        Expression::StaticMemberExpression(member) => {
            let name = member.property.name.as_str();
            is_naming_property(name).then(|| name.to_string())
        }
        Expression::ComputedMemberExpression(member) => indexed_element_name(member),
        _ => None,
    }
}

pub fn is_informative_init(init: &Expression<'_>) -> bool {
    matches!(
        init.without_parentheses(),
        Expression::CallExpression(_)
            | Expression::NewExpression(_)
            | Expression::AwaitExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::Identifier(_)
    )
}
