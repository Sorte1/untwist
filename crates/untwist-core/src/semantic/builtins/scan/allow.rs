use oxc_ast::ast::*;

use super::reach::{member_of, Key, CODE_RUNNERS, REACHING_KEYS};
use super::{Scan, This, GLOBAL_OBJECTS, LEGACY_DEFINERS};

pub(super) const BUILTIN_ROOTS: &[&str] = &[
    "Object",
    "Array",
    "Reflect",
    "Symbol",
    "String",
    "Number",
    "Boolean",
    "BigInt",
    "JSON",
    "Math",
    "RegExp",
    "Promise",
    "Date",
    "Error",
    "TypeError",
    "Map",
    "Set",
    "WeakMap",
    "WeakSet",
    "Proxy",
    "Intl",
    "ArrayBuffer",
    "atob",
    "btoa",
];

const COMMON_STATICS: &[&str] = &["name", "length", "prototype"];

const STATICS: &[(&str, &[&str])] = &[
    (
        "Object",
        &[
            "keys",
            "values",
            "entries",
            "assign",
            "create",
            "freeze",
            "isFrozen",
            "seal",
            "isSealed",
            "preventExtensions",
            "isExtensible",
            "is",
            "defineProperty",
            "defineProperties",
            "getOwnPropertyNames",
            "getOwnPropertySymbols",
            "getOwnPropertyDescriptor",
            "getOwnPropertyDescriptors",
            "fromEntries",
            "hasOwn",
            "setPrototypeOf",
            "groupBy",
        ],
    ),
    ("Array", &["isArray", "from", "of", "fromAsync"]),
    ("String", &["fromCharCode", "fromCodePoint", "raw"]),
    (
        "Number",
        &[
            "isNaN",
            "isFinite",
            "isInteger",
            "isSafeInteger",
            "parseFloat",
            "parseInt",
            "EPSILON",
            "MAX_SAFE_INTEGER",
            "MIN_SAFE_INTEGER",
            "MAX_VALUE",
            "MIN_VALUE",
            "NaN",
            "NEGATIVE_INFINITY",
            "POSITIVE_INFINITY",
        ],
    ),
    ("BigInt", &["asIntN", "asUintN"]),
    ("JSON", &["parse", "stringify"]),
    (
        "Math",
        &[
            "E", "LN10", "LN2", "LOG10E", "LOG2E", "PI", "SQRT1_2", "SQRT2", "abs", "acos",
            "acosh", "asin", "asinh", "atan", "atan2", "atanh", "cbrt", "ceil", "clz32", "cos",
            "cosh", "exp", "expm1", "floor", "fround", "hypot", "imul", "log", "log10", "log1p",
            "log2", "max", "min", "pow", "random", "round", "sign", "sin", "sinh", "sqrt", "tan",
            "tanh", "trunc",
        ],
    ),
    (
        "Symbol",
        &[
            "for",
            "keyFor",
            "iterator",
            "asyncIterator",
            "hasInstance",
            "isConcatSpreadable",
            "match",
            "matchAll",
            "replace",
            "search",
            "species",
            "split",
            "toPrimitive",
            "toStringTag",
            "unscopables",
            "dispose",
            "asyncDispose",
        ],
    ),
    (
        "Promise",
        &[
            "resolve",
            "reject",
            "all",
            "allSettled",
            "race",
            "any",
            "withResolvers",
        ],
    ),
    ("Date", &["now", "parse", "UTC"]),
    (
        "Reflect",
        &[
            "apply",
            "construct",
            "defineProperty",
            "deleteProperty",
            "has",
            "isExtensible",
            "ownKeys",
            "preventExtensions",
            "set",
            "setPrototypeOf",
            "getOwnPropertyDescriptor",
        ],
    ),
    ("Error", &["captureStackTrace", "stackTraceLimit"]),
    ("TypeError", &[]),
    ("Map", &["groupBy"]),
    ("Proxy", &["revocable"]),
    (
        "Intl",
        &[
            "Collator",
            "DateTimeFormat",
            "DisplayNames",
            "ListFormat",
            "Locale",
            "NumberFormat",
            "PluralRules",
            "RelativeTimeFormat",
            "Segmenter",
            "getCanonicalLocales",
            "supportedValuesOf",
        ],
    ),
    ("ArrayBuffer", &["isView"]),
    ("Boolean", &[]),
];

const PROTOTYPE_METHODS: &[&str] = &[
    "toString",
    "toLocaleString",
    "valueOf",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "call",
    "apply",
    "bind",
    "at",
    "concat",
    "copyWithin",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "unshift",
    "values",
    "toReversed",
    "toSorted",
    "toSpliced",
    "with",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "endsWith",
    "localeCompare",
    "match",
    "matchAll",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "replace",
    "replaceAll",
    "search",
    "split",
    "startsWith",
    "substring",
    "substr",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "trim",
    "trimStart",
    "trimEnd",
    "trimLeft",
    "trimRight",
    "toFixed",
    "toExponential",
    "toPrecision",
    "exec",
    "test",
    "then",
    "catch",
    "finally",
    "get",
    "set",
    "has",
    "delete",
    "clear",
    "add",
    "getTime",
    "getFullYear",
    "getMonth",
    "getDate",
    "getDay",
    "getHours",
    "getMinutes",
    "getSeconds",
    "getMilliseconds",
    "getTimezoneOffset",
    "getUTCFullYear",
    "getUTCMonth",
    "getUTCDate",
    "getUTCDay",
    "getUTCHours",
    "getUTCMinutes",
    "getUTCSeconds",
    "getUTCMilliseconds",
    "toISOString",
    "toJSON",
    "toDateString",
    "toTimeString",
    "toUTCString",
    "toLocaleDateString",
    "toLocaleTimeString",
    "byteLength",
    "description",
    "message",
    "stack",
    "flags",
    "source",
    "global",
    "size",
];

const PROTOTYPE_OWNER: &str = "Function";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Root<'n> {
    Builtin(&'n str),
    Prototype(&'n str),
    Global,
    Runner(&'n str),
}

fn static_names(root: &str) -> &'static [&'static str] {
    STATICS
        .iter()
        .find(|(owner, _)| *owner == root)
        .map_or(&[], |(_, names)| *names)
}

impl Scan<'_> {
    pub(super) fn is_builtin_root_name(&self, name: &str) -> bool {
        BUILTIN_ROOTS.contains(&name) || self.reliance.roots.contains(&name)
    }

    fn root_named<'n>(&self, name: &'n str) -> Option<Root<'n>> {
        if CODE_RUNNERS.contains(&name) {
            return Some(Root::Runner(name));
        }
        if GLOBAL_OBJECTS.contains(&name) {
            return Some(Root::Global);
        }
        self.is_builtin_root_name(name)
            .then_some(Root::Builtin(name))
    }

    pub(super) fn allow_root<'e>(&self, expression: &'e Expression<'_>) -> Option<Root<'e>> {
        match expression.without_parentheses() {
            Expression::Identifier(id) => {
                let root = self.root_named(id.name.as_str())?;
                self.global_named(id, &[id.name.as_str()]).then_some(root)
            }
            Expression::ThisExpression(_) => {
                (self.this_value() == This::Global).then_some(Root::Global)
            }
            other => {
                let (object, Key::Name(name)) = member_of(other)? else {
                    return None;
                };
                match self.allow_root(object)? {
                    Root::Global => self.root_named(name),
                    Root::Builtin(owner) if name == "prototype" => Some(Root::Prototype(owner)),
                    Root::Runner(owner) if name == "prototype" && owner == PROTOTYPE_OWNER => {
                        Some(Root::Prototype(owner))
                    }
                    _ => None,
                }
            }
        }
    }

    pub(super) fn read_allowed(&self, root: Root<'_>, key: Key<'_>) -> bool {
        let name = match key {
            Key::Name(name) => name.to_string(),
            Key::Other => return root == Root::Global,
            Key::Unknown(expression) => match self.known_key(expression) {
                Some(name) => name,
                None => return false,
            },
        };
        let name = name.as_str();
        if REACHING_KEYS.contains(&name) || LEGACY_DEFINERS.contains(&name) {
            return false;
        }
        match root {
            Root::Global => true,
            Root::Builtin(owner) => {
                COMMON_STATICS.contains(&name) || static_names(owner).contains(&name)
            }
            Root::Prototype(_) => PROTOTYPE_METHODS.contains(&name),
            Root::Runner(owner) => owner == PROTOTYPE_OWNER && name == "prototype",
        }
    }
}
