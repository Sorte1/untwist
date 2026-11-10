use std::rc::Rc;

use crate::bail::{Bail, ErrorKind, EvalResult};
use crate::builtins::shape::SHAPES;
use crate::builtins::Realm;
use crate::host::Host;
use crate::object::{
    decoding, is_arguments, is_constructor_key, lookup_root_address, OBJECT_PROTOTYPE_KEYS,
};
use crate::value::{Abrupt, Eval, FnCode, IntrinsicProto, ObjKind, ObjRef, PropKey, Value};

const FUNCTION_OWN_KEYS: [&str; 5] = ["length", "name", "arguments", "caller", "prototype"];

const FUNCTION_FIXED_KEYS: [&str; 5] = ["length", "name", "arguments", "caller", "constructor"];

const ARGUMENTS_OWN_KEYS: [&str; 2] = ["length", "callee"];

const FUNCTION_PROTOTYPE_UNINSTALLED: [&str; 1] = ["constructor"];

const ARRAY_UNINSTALLED: [&str; 24] = [
    "length",
    "constructor",
    "at",
    "copyWithin",
    "fill",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "lastIndexOf",
    "sort",
    "keys",
    "entries",
    "values",
    "flat",
    "flatMap",
    "every",
    "some",
    "reduceRight",
    "toReversed",
    "toSorted",
    "toSpliced",
    "with",
    "toLocaleString",
];

const REGEXP_OWN_KEYS: [&str; 1] = ["lastIndex"];

const REGEXP_PROTOTYPE_UNINSTALLED: [&str; 12] = [
    "constructor",
    "dotAll",
    "flags",
    "global",
    "hasIndices",
    "ignoreCase",
    "multiline",
    "source",
    "sticky",
    "unicode",
    "compile",
    "unicodeSets",
];

const STRING_UNINSTALLED: [&str; 27] = [
    "length",
    "constructor",
    "anchor",
    "at",
    "big",
    "blink",
    "bold",
    "fontcolor",
    "fontsize",
    "fixed",
    "isWellFormed",
    "italics",
    "link",
    "localeCompare",
    "matchAll",
    "normalize",
    "small",
    "strike",
    "sub",
    "sup",
    "toWellFormed",
    "trimStart",
    "trimLeft",
    "trimEnd",
    "trimRight",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
];

const NUMBER_UNINSTALLED: [&str; 4] = [
    "constructor",
    "toExponential",
    "toPrecision",
    "toLocaleString",
];

const NODE_GLOBALS: [&str; 134] = [
    "AbortController",
    "AbortSignal",
    "AggregateError",
    "Array",
    "ArrayBuffer",
    "Atomics",
    "BigInt",
    "BigInt64Array",
    "BigUint64Array",
    "Blob",
    "Boolean",
    "BroadcastChannel",
    "Buffer",
    "ByteLengthQueuingStrategy",
    "CompressionStream",
    "CountQueuingStrategy",
    "Crypto",
    "CryptoKey",
    "CustomEvent",
    "DOMException",
    "DataView",
    "Date",
    "DecompressionStream",
    "Error",
    "EvalError",
    "Event",
    "EventTarget",
    "File",
    "FinalizationRegistry",
    "Float32Array",
    "Float64Array",
    "FormData",
    "Function",
    "Headers",
    "Infinity",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "Intl",
    "Iterator",
    "JSON",
    "Map",
    "Math",
    "MessageChannel",
    "MessageEvent",
    "MessagePort",
    "NaN",
    "Navigator",
    "Number",
    "Object",
    "Performance",
    "PerformanceEntry",
    "PerformanceMark",
    "PerformanceMeasure",
    "PerformanceObserver",
    "PerformanceObserverEntryList",
    "PerformanceResourceTiming",
    "Promise",
    "Proxy",
    "RangeError",
    "ReadableByteStreamController",
    "ReadableStream",
    "ReadableStreamBYOBReader",
    "ReadableStreamBYOBRequest",
    "ReadableStreamDefaultController",
    "ReadableStreamDefaultReader",
    "ReferenceError",
    "Reflect",
    "RegExp",
    "Request",
    "Response",
    "Set",
    "SharedArrayBuffer",
    "String",
    "SubtleCrypto",
    "Symbol",
    "SyntaxError",
    "TextDecoder",
    "TextDecoderStream",
    "TextEncoder",
    "TextEncoderStream",
    "TransformStream",
    "TransformStreamDefaultController",
    "TypeError",
    "URIError",
    "URL",
    "URLSearchParams",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "WeakMap",
    "WeakRef",
    "WeakSet",
    "WebAssembly",
    "WebSocket",
    "WritableStream",
    "WritableStreamDefaultController",
    "WritableStreamDefaultWriter",
    "__dirname",
    "__filename",
    "atob",
    "btoa",
    "clearImmediate",
    "clearInterval",
    "clearTimeout",
    "console",
    "crypto",
    "decodeURI",
    "decodeURIComponent",
    "encodeURI",
    "encodeURIComponent",
    "escape",
    "eval",
    "exports",
    "fetch",
    "global",
    "globalThis",
    "isFinite",
    "isNaN",
    "module",
    "navigator",
    "parseFloat",
    "parseInt",
    "performance",
    "process",
    "queueMicrotask",
    "require",
    "setImmediate",
    "setInterval",
    "setTimeout",
    "structuredClone",
    "undefined",
    "unescape",
];

const HAS_INSTANCE: &str = "@@hasInstance";

const INTRINSIC_PROTO_TAGS: std::ops::Range<u8> = 1..0x40;

fn listed(list: &[&str], key: &PropKey) -> bool {
    match key {
        PropKey::Name(name) => list.iter().any(|known| ascii_eq(name.units(), known)),
        PropKey::Index(_) => false,
    }
}

fn ascii_eq(units: &[u16], known: &str) -> bool {
    units.len() == known.len()
        && units
            .iter()
            .zip(known.bytes())
            .all(|(unit, byte)| *unit == u16::from(byte))
}

fn object_standard(uninstalled: &[&str], key: &PropKey) -> bool {
    listed(uninstalled, key) || listed(&OBJECT_PROTOTYPE_KEYS, key)
}

const STANDARD_OWNERS: [(&str, &[&str]); 5] = [
    ("Array.prototype", &ARRAY_UNINSTALLED),
    ("Function.prototype", &FUNCTION_PROTOTYPE_UNINSTALLED),
    ("Number.prototype", &NUMBER_UNINSTALLED),
    ("RegExp.prototype", &REGEXP_PROTOTYPE_UNINSTALLED),
    ("String.prototype", &STRING_UNINSTALLED),
];

pub fn uninstalled_members(owner: &str) -> &'static [&'static str] {
    STANDARD_OWNERS
        .iter()
        .find(|(name, _)| *name == owner)
        .map_or(&[], |(_, list)| list)
}

pub fn installed_members(owner: &str) -> Vec<&'static str> {
    SHAPES
        .iter()
        .filter(|entry| entry.owner == owner)
        .map(|entry| entry.name)
        .collect()
}

pub fn standard_members(owner: &str) -> Vec<&'static str> {
    let mut members = installed_members(owner);
    members.extend_from_slice(uninstalled_members(owner));
    members
}

fn plain_is_intrinsic(obj: &ObjRef<'_>, tag: u8) -> bool {
    INTRINSIC_PROTO_TAGS.contains(&tag) || lookup_root_address(Rc::as_ptr(obj) as usize).is_some()
}

pub fn is_intrinsic_object(obj: &ObjRef<'_>) -> bool {
    let borrowed = obj.borrow();
    match borrowed.kind {
        ObjKind::Native(_, _) => true,
        ObjKind::Plain(tag) => plain_is_intrinsic(obj, tag),
        _ => false,
    }
}

fn intrinsic_miss(key: &PropKey) -> bool {
    matches!(key, PropKey::Name(name) if !ascii_eq(name.units(), HAS_INSTANCE))
}

fn object_miss(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    if is_intrinsic_object(obj) {
        return intrinsic_miss(key);
    }
    match &obj.borrow().kind {
        ObjKind::Error(_) => true,
        kind @ (ObjKind::Function(_) | ObjKind::Bound(_)) => {
            (listed(&FUNCTION_OWN_KEYS, key) && !lacks_prototype(kind, key))
                || object_standard(&FUNCTION_PROTOTYPE_UNINSTALLED, key)
        }
        ObjKind::Array(_) => object_standard(&ARRAY_UNINSTALLED, key),
        ObjKind::RegExp(_) => {
            listed(&REGEXP_OWN_KEYS, key) || object_standard(&REGEXP_PROTOTYPE_UNINSTALLED, key)
        }
        ObjKind::Plain(_) | ObjKind::Native(_, _) => listed(&OBJECT_PROTOTYPE_KEYS, key),
        ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => false,
    }
}

fn lacks_prototype(kind: &ObjKind<'_>, key: &PropKey) -> bool {
    let without = match kind {
        ObjKind::Bound(_) => true,
        ObjKind::Function(closure) => match closure.code {
            FnCode::Arrow(_) => true,
            FnCode::Function(function) => !function.generator,
        },
        _ => false,
    };
    without && !decoding() && crate::object::is_prototype_key(key)
}

pub fn miss_is_unmodelled(target: &Value<'_>, key: &PropKey) -> bool {
    match target {
        Value::Str(_) => object_standard(&STRING_UNINSTALLED, key),
        Value::Number(_) => object_standard(&NUMBER_UNINSTALLED, key),
        Value::Object(obj) => object_miss(obj, key),
        _ => false,
    }
}

const UNMODELLED_MEMBER: &str = "a property node has that the evaluator does not model";

pub fn refuse_unmodelled_miss(target: &Value<'_>, key: &PropKey) -> EvalResult<()> {
    if miss_is_unmodelled(target, key) {
        return Err(Bail::Unsupported(&UNMODELLED_MEMBER));
    }
    Ok(())
}

pub(crate) const UNMODELLED_CONSTRUCTOR: &str =
    "a constructor the evaluator does not model, read through a built-in prototype";

const BOOLEAN_MEMBER: &str = "a boolean has no modelled prototype";

pub fn refuse_boolean_member<'a>() -> Eval<'a, Value<'a>> {
    Err(Bail::Unsupported(&BOOLEAN_MEMBER).into())
}

pub fn refuse_primitive_miss<'a>(
    target: &Value<'a>,
    key: &PropKey,
    found: Value<'a>,
) -> Eval<'a, Value<'a>> {
    if matches!(found, Value::Undefined) {
        if is_constructor_key(key) {
            return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR).into());
        }
        refuse_unmodelled_miss(target, key)?;
    }
    Ok(found)
}

pub fn refuse_own_undefined(obj: &ObjRef<'_>, key: &PropKey) -> EvalResult<()> {
    if !is_constructor_key(key) {
        return Ok(());
    }
    if !obj.borrow().props.contains_key(key) || is_intrinsic_object(obj) {
        return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR));
    }
    Ok(())
}

pub fn refuse_inherited_miss<'a>(
    realm: &Realm<'a>,
    (target, obj, root): (&Value<'a>, &ObjRef<'a>, bool),
    key: &PropKey,
    inherited: Option<&Value<'a>>,
) -> EvalResult<()> {
    if decoding() && is_arguments(obj) {
        crate::interp::refuse_arguments_miss(key, inherited)?;
    }
    if is_constructor_key(key) {
        refuse_unmodelled_constructor(obj, inherited)?;
    }
    if inherited.is_none() {
        realm.refuse_global_object(obj)?;
        if root && intrinsic_miss(key) {
            return Err(Bail::Unsupported(&UNMODELLED_MEMBER));
        }
        refuse_unmodelled_miss(target, key)?;
    }
    Ok(())
}

fn refuse_unmodelled_constructor(obj: &ObjRef<'_>, found: Option<&Value<'_>>) -> EvalResult<()> {
    let unmodelled = matches!(found, Some(Value::Undefined))
        || match &obj.borrow().kind {
            ObjKind::Function(closure) => closure.is_generator_or_async(),
            ObjKind::Bound(_) => true,
            _ => false,
        };
    if unmodelled {
        return Err(Bail::Unsupported(&UNMODELLED_CONSTRUCTOR));
    }
    Ok(())
}

pub fn refuse_unmodelled_own_miss(target: &Value<'_>, key: &PropKey) -> EvalResult<()> {
    let unmodelled = match target {
        Value::Str(_) | Value::Undefined | Value::Null => true,
        Value::Object(obj) if is_intrinsic_object(obj) => intrinsic_miss(key),
        Value::Object(obj) => match &obj.borrow().kind {
            ObjKind::Error(_) => true,
            kind @ (ObjKind::Function(_) | ObjKind::Bound(_)) => {
                listed(&FUNCTION_OWN_KEYS, key) && !lacks_prototype(kind, key)
            }
            ObjKind::RegExp(_) => listed(&REGEXP_OWN_KEYS, key),
            ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => listed(&ARGUMENTS_OWN_KEYS, key),
            _ => false,
        },
        _ => false,
    };
    if unmodelled {
        return Err(Bail::Unsupported(
            &"an own property node has that the evaluator does not model",
        ));
    }
    Ok(())
}

fn write_is_unmodelled(obj: &ObjRef<'_>, key: &PropKey) -> bool {
    match &obj.borrow().kind {
        ObjKind::Plain(0) => false,
        ObjKind::Plain(tag) => plain_is_intrinsic(obj, *tag),
        ObjKind::Function(_) => listed(&FUNCTION_FIXED_KEYS, key),
        ObjKind::Bound(_) | ObjKind::Native(_, _) | ObjKind::RegExp(_) | ObjKind::Error(_) => true,
        ObjKind::Arguments(_) | ObjKind::MappedArguments(_) => listed(&ARGUMENTS_OWN_KEYS, key),
        ObjKind::Array(_) => false,
    }
}

#[inline(never)]
pub fn refuse_unmodelled_write(obj: &ObjRef<'_>, key: &PropKey) -> EvalResult<()> {
    if write_is_unmodelled(obj, key) {
        return Err(Bail::Unsupported(
            &"a write to a property whose attributes or accessors the evaluator does not model",
        ));
    }
    Ok(())
}

#[inline(never)]
pub fn refuse_intrinsic_index_write(obj: &ObjRef<'_>, key: &PropKey) -> EvalResult<()> {
    let PropKey::Index(_) = key else {
        return Ok(());
    };
    let intrinsic = match obj.borrow().kind {
        ObjKind::Plain(tag) => tag != 0 && plain_is_intrinsic(obj, tag),
        _ => false,
    };
    if intrinsic {
        return Err(Bail::Unsupported(
            &"a write to a property whose attributes or accessors the evaluator does not model",
        ));
    }
    Ok(())
}

const ERROR_PROTOTYPE_KEYS: [&str; 3] = ["message", "name", "toString"];

#[cold]
#[inline(never)]
pub fn refuse_error_prototype_member(key: &PropKey) -> EvalResult<()> {
    if listed(&ERROR_PROTOTYPE_KEYS, key) {
        return Err(Bail::Unsupported(
            &"a member of Error.prototype, which the evaluator does not model",
        ));
    }
    Ok(())
}

const CALLABLE_READ_ONLY_KEYS: [&str; 2] = ["name", "length"];

#[inline(never)]
pub fn refuse_callable_name_or_length(obj: &ObjRef<'_>, key: &PropKey) -> EvalResult<()> {
    let callable = matches!(
        obj.borrow().kind,
        ObjKind::Function(_) | ObjKind::Bound(_) | ObjKind::Native(_, _)
    );
    if callable && listed(&CALLABLE_READ_ONLY_KEYS, key) {
        return Err(Bail::Unsupported(
            &"a write to a function's name or length, which the evaluator does not model",
        ));
    }
    Ok(())
}

pub fn refuse_unmodelled_prototype(proto: &ObjRef<'_>) -> EvalResult<()> {
    if !decoding() {
        return Ok(());
    }
    let object_prototype = proto.borrow().trace_tag() == IntrinsicProto::Object.bit();
    let plain = matches!(proto.borrow().kind, ObjKind::Plain(_));
    if !plain || (!object_prototype && is_intrinsic_object(proto)) {
        return Err(Bail::Unsupported(
            &"a prototype that is not an ordinary object",
        ));
    }
    Ok(())
}

pub fn refuse_unmodelled_keys(target: &Value<'_>) -> EvalResult<()> {
    let Some(obj) = target.as_object() else {
        return Ok(());
    };
    if !decoding() {
        return Ok(());
    }
    let callable = matches!(
        obj.borrow().kind,
        ObjKind::Function(_) | ObjKind::Bound(_) | ObjKind::Native(_, _)
    );
    if callable || is_intrinsic_object(obj) {
        return Err(Bail::Unsupported(
            &"the own keys of a built-in or a function, which the evaluator does not model",
        ));
    }
    Ok(())
}

pub fn throw_or_refuse<'a>(
    host: &dyn Host<'a>,
    kind: ErrorKind,
    message: &str,
    why: &'static &'static str,
) -> Abrupt<'a> {
    if decoding() {
        return Bail::Unsupported(why).into();
    }
    host.throw(kind, message)
}

pub fn is_node_global(name: &str) -> bool {
    NODE_GLOBALS.binary_search(&name).is_ok()
}

pub fn refuse_while_decoding(why: &'static &'static str) -> EvalResult<()> {
    if decoding() {
        return Err(Bail::Unsupported(why));
    }
    Ok(())
}
