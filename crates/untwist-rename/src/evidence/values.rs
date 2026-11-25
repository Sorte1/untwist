use oxc_ast::ast::*;
use oxc_syntax::operator::BinaryOperator;
use untwist_core::ast_util::prop::member_parts;

use crate::generate::{lower_camel, strip_accessor_prefix};
use crate::human::looks_human_written;
use crate::reserved::is_reserved;

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

const UNNAMING_METHODS: [&str; 15] = [
    "call",
    "apply",
    "bind",
    "then",
    "catch",
    "finally",
    "push",
    "unshift",
    "splice",
    "valueOf",
    "has",
    "emit",
    "log",
    "build",
    "subscribe",
];

const ELEMENT_METHODS: [&str; 5] = ["get", "at", "pop", "shift", "peek"];

const RECEIVER_METHODS: [&str; 14] = [
    "set",
    "add",
    "sub",
    "mul",
    "div",
    "slice",
    "concat",
    "replace",
    "trim",
    "assign",
    "create",
    "copy",
    "normalize",
    "negate",
];

const SHORT_PROPERTIES: [&str; 15] = [
    "x", "y", "z", "w", "i", "dx", "dy", "dz", "vx", "vy", "vz", "r", "g", "b", "id",
];

const CONVERSION_PREFIX: &str = "to";

const AWAITED_PREFIXES: [&str; 4] = ["get", "fetch", "load", "read"];

const ASYNC_SUFFIX: &str = "Async";

const LAST_INDEX: &str = "lastIndex";

const LOOKED_UP: &str = "value";

const CALL_RESULT: &str = "result";

const THIS_ALIAS: &str = "self";

const FUNCTION_VALUE: &str = "fn";

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
    if let Some(name) = verb_result_name(call) {
        return (name != CALL_RESULT).then_some(name);
    }
    callee_word(&method_name(&call.callee)?)
}

fn callee_word(callee: &str) -> Option<String> {
    if !is_naming_property(callee) {
        return None;
    }
    strip_accessor_prefix(callee)
        .or_else(|| Some(lower_camel(callee)))
        .filter(|name| is_word_like(name))
}

fn conversion_name(method: &str) -> Option<String> {
    let rest = method.strip_prefix(CONVERSION_PREFIX)?;
    if !rest.starts_with(|ch: char| ch.is_ascii_uppercase()) {
        return None;
    }
    let name = if rest.chars().all(|ch| ch.is_ascii_uppercase()) {
        rest.to_ascii_lowercase()
    } else {
        lower_camel(rest)
    };
    (is_word_like(&name) && !is_reserved(&name)).then_some(name)
}

fn receiver_name(callee: &Expression<'_>) -> Option<String> {
    let (receiver, _) = member_parts(callee.without_parentheses())?;
    let name = collection_name(receiver)?;
    is_naming_property(name).then(|| lower_camel(name))
}

fn verb_result_name(call: &CallExpression<'_>) -> Option<String> {
    let method = method_name(&call.callee)?;
    let method = method.as_ref();
    if ELEMENT_METHODS.contains(&method) {
        let element = member_parts(call.callee.without_parentheses())
            .and_then(|(receiver, _)| element_name(receiver));
        return Some(element.unwrap_or_else(|| LOOKED_UP.to_string()));
    }
    let is_conversion = method
        .strip_prefix(CONVERSION_PREFIX)
        .is_some_and(|rest| rest.starts_with(|ch: char| ch.is_ascii_uppercase()));
    if is_conversion {
        if let Some(name) = conversion_name(method) {
            return Some(name);
        }
    }
    if is_conversion || RECEIVER_METHODS.contains(&method) {
        return Some(receiver_name(&call.callee).unwrap_or_else(|| CALL_RESULT.to_string()));
    }
    UNNAMING_METHODS
        .contains(&method)
        .then(|| CALL_RESULT.to_string())
}

fn singular(plural: &str) -> Option<String> {
    let cache_like =
        plural.ends_with("aches") && !plural.ends_with("eaches") && !plural.ends_with("oaches");
    let stem = if let Some(stem) = plural.strip_suffix("ies") {
        format!("{stem}y")
    } else if cache_like {
        plural[..plural.len() - 1].to_string()
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

fn singular_property_element(member: &ComputedMemberExpression<'_>) -> Option<String> {
    if !matches!(member.expression, Expression::NumericLiteral(_)) {
        return None;
    }
    let Expression::StaticMemberExpression(collection) = member.object.without_parentheses() else {
        return None;
    };
    member_value_name(collection)
}

fn member_value_name(member: &StaticMemberExpression<'_>) -> Option<String> {
    let name = member.property.name.as_str();
    (SHORT_PROPERTIES.contains(&name) || is_naming_property(name)).then(|| name.to_string())
}

fn last_index_name(binary: &BinaryExpression<'_>) -> Option<String> {
    let is_length = matches!(binary.left.without_parentheses(), Expression::StaticMemberExpression(member) if member.property.name == "length");
    let is_one =
        matches!(&binary.right, Expression::NumericLiteral(literal) if literal.value == 1.0);
    let last = binary.operator == BinaryOperator::Subtraction && is_length && is_one;
    last.then(|| LAST_INDEX.to_string())
}

fn awaited_call_name(call: &CallExpression<'_>) -> Option<String> {
    if let Some(name) = verb_result_name(call) {
        return Some(name);
    }
    let callee = match method_name(&call.callee) {
        Some(method) => method,
        None => std::borrow::Cow::Borrowed(global_callee(&call.callee)?),
    };
    awaited_word(&callee)
}

fn awaited_word(callee: &str) -> Option<String> {
    let stem = callee.strip_suffix(ASYNC_SUFFIX).unwrap_or(callee);
    let rest = AWAITED_PREFIXES
        .iter()
        .find_map(|prefix| stem.strip_prefix(prefix))
        .filter(|rest| rest.starts_with(|ch: char| ch.is_ascii_uppercase()));
    let name = lower_camel(rest.unwrap_or(stem));
    let usable = is_naming_property(&name) && !AWAITED_PREFIXES.contains(&name.as_str());
    usable.then_some(name)
}

pub fn declared_fallback_name(init: &Expression<'_>) -> Option<String> {
    match init.without_parentheses() {
        Expression::StaticMemberExpression(member) => member_value_name(member),
        Expression::ComputedMemberExpression(member) => singular_property_element(member),
        Expression::BinaryExpression(binary) => last_index_name(binary),
        Expression::ThisExpression(_) => Some(THIS_ALIAS.to_string()),
        Expression::CallExpression(call) => verb_result_name(call),
        Expression::AwaitExpression(awaited) => match awaited.argument.without_parentheses() {
            Expression::CallExpression(call) => awaited_call_name(call),
            _ => None,
        },
        _ => None,
    }
}

pub fn is_element_lookup(init: &Expression<'_>) -> bool {
    let Expression::CallExpression(call) = init.without_parentheses() else {
        return false;
    };
    method_name(&call.callee).is_some_and(|method| ELEMENT_METHODS.contains(&method.as_ref()))
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
        Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_) => {
            Some(FUNCTION_VALUE.to_string())
        }
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
