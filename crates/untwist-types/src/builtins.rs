use untwist_core::semantic::builtins::Reliance;

use crate::ty::{Kind, Ty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Out {
    Num,
    Str,
    Bool,
    StrOrUndefined,
    NumOrUndefined,
}

impl Out {
    pub fn ty(self) -> Ty {
        match self {
            Out::Num => Ty::number(),
            Out::Str => Ty::string(),
            Out::Bool => Ty::boolean(),
            Out::StrOrUndefined => Ty::string().join(&Ty::undefined()),
            Out::NumOrUndefined => Ty::number().join(&Ty::undefined()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Param {
    Num,
    Str,
    Any,
    Primitive,
    Pattern,
    Replacement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    Global,
    Static(&'static str),
    Proto(Kind),
}

#[derive(Clone, Copy, Debug)]
pub struct Row {
    pub owner: Owner,
    pub key: &'static str,
    pub proof: Out,
    pub lib: Out,
    pub params: &'static [Param],
    pub call: bool,
}

const fn call(owner: Owner, key: &'static str, out: Out, params: &'static [Param]) -> Row {
    Row {
        owner,
        key,
        proof: out,
        lib: out,
        params,
        call: true,
    }
}

const fn read(owner: Owner, key: &'static str) -> Row {
    Row {
        owner,
        key,
        proof: Out::Num,
        lib: Out::Num,
        params: &[],
        call: false,
    }
}

const G: Owner = Owner::Global;
const MATH: Owner = Owner::Static("Math");
const NUMBER: Owner = Owner::Static("Number");
const STRING: Owner = Owner::Static("String");
const STR: Owner = Owner::Proto(Kind::String);
const NUM: Owner = Owner::Proto(Kind::Number);
const BOOL: Owner = Owner::Proto(Kind::Boolean);

const N: &[Param] = &[Param::Num];
const NN: &[Param] = &[Param::Num, Param::Num];
const S: &[Param] = &[Param::Str];
const SN: &[Param] = &[Param::Str, Param::Num];
const A: &[Param] = &[Param::Any];
const AA: &[Param] = &[Param::Any, Param::Any];
const NONE: &[Param] = &[];
const MANY_N: &[Param] = &[
    Param::Num,
    Param::Num,
    Param::Num,
    Param::Num,
    Param::Num,
    Param::Num,
];

pub const ROWS: &[Row] = &[
    call(G, "parseInt", Out::Num, SN),
    call(G, "parseFloat", Out::Num, S),
    call(G, "isNaN", Out::Bool, N),
    call(G, "isFinite", Out::Bool, N),
    call(G, "String", Out::Str, A),
    call(G, "Number", Out::Num, A),
    call(G, "Boolean", Out::Bool, A),
    call(G, "encodeURIComponent", Out::Str, &[Param::Primitive]),
    call(G, "encodeURI", Out::Str, S),
    call(G, "decodeURIComponent", Out::Str, S),
    call(G, "decodeURI", Out::Str, S),
    call(G, "escape", Out::Str, S),
    call(G, "unescape", Out::Str, S),
    call(MATH, "abs", Out::Num, N),
    call(MATH, "acos", Out::Num, N),
    call(MATH, "asin", Out::Num, N),
    call(MATH, "atan", Out::Num, N),
    call(MATH, "atan2", Out::Num, NN),
    call(MATH, "cbrt", Out::Num, N),
    call(MATH, "ceil", Out::Num, N),
    call(MATH, "clz32", Out::Num, N),
    call(MATH, "cos", Out::Num, N),
    call(MATH, "exp", Out::Num, N),
    call(MATH, "floor", Out::Num, N),
    call(MATH, "fround", Out::Num, N),
    call(MATH, "hypot", Out::Num, MANY_N),
    call(MATH, "imul", Out::Num, NN),
    call(MATH, "log", Out::Num, N),
    call(MATH, "log10", Out::Num, N),
    call(MATH, "log2", Out::Num, N),
    call(MATH, "max", Out::Num, MANY_N),
    call(MATH, "min", Out::Num, MANY_N),
    call(MATH, "pow", Out::Num, NN),
    call(MATH, "random", Out::Num, NONE),
    call(MATH, "round", Out::Num, N),
    call(MATH, "sign", Out::Num, N),
    call(MATH, "sin", Out::Num, N),
    call(MATH, "sqrt", Out::Num, N),
    call(MATH, "tan", Out::Num, N),
    call(MATH, "trunc", Out::Num, N),
    read(MATH, "E"),
    read(MATH, "LN2"),
    read(MATH, "LN10"),
    read(MATH, "LOG2E"),
    read(MATH, "LOG10E"),
    read(MATH, "PI"),
    read(MATH, "SQRT1_2"),
    read(MATH, "SQRT2"),
    Row {
        owner: Owner::Static("JSON"),
        key: "stringify",
        proof: Out::StrOrUndefined,
        lib: Out::Str,
        params: A,
        call: true,
    },
    call(NUMBER, "isInteger", Out::Bool, A),
    call(NUMBER, "isFinite", Out::Bool, A),
    call(NUMBER, "isNaN", Out::Bool, A),
    call(NUMBER, "isSafeInteger", Out::Bool, A),
    call(NUMBER, "parseFloat", Out::Num, S),
    call(NUMBER, "parseInt", Out::Num, SN),
    read(NUMBER, "MAX_SAFE_INTEGER"),
    read(NUMBER, "MIN_SAFE_INTEGER"),
    read(NUMBER, "EPSILON"),
    read(NUMBER, "MAX_VALUE"),
    read(NUMBER, "MIN_VALUE"),
    read(NUMBER, "POSITIVE_INFINITY"),
    read(NUMBER, "NEGATIVE_INFINITY"),
    call(STRING, "fromCharCode", Out::Str, MANY_N),
    call(Owner::Static("Array"), "isArray", Out::Bool, A),
    call(Owner::Static("Date"), "now", Out::Num, NONE),
    call(Owner::Static("Object"), "is", Out::Bool, AA),
    call(STR, "charAt", Out::Str, N),
    call(STR, "charCodeAt", Out::Num, N),
    call(STR, "codePointAt", Out::NumOrUndefined, N),
    call(STR, "at", Out::StrOrUndefined, N),
    call(STR, "indexOf", Out::Num, SN),
    call(STR, "lastIndexOf", Out::Num, SN),
    call(STR, "includes", Out::Bool, SN),
    call(STR, "startsWith", Out::Bool, SN),
    call(STR, "endsWith", Out::Bool, SN),
    call(STR, "slice", Out::Str, NN),
    call(STR, "substring", Out::Str, NN),
    call(STR, "substr", Out::Str, NN),
    call(STR, "toUpperCase", Out::Str, NONE),
    call(STR, "toLowerCase", Out::Str, NONE),
    call(STR, "trim", Out::Str, NONE),
    call(STR, "trimStart", Out::Str, NONE),
    call(STR, "trimEnd", Out::Str, NONE),
    call(STR, "padStart", Out::Str, &[Param::Num, Param::Str]),
    call(STR, "padEnd", Out::Str, &[Param::Num, Param::Str]),
    call(STR, "repeat", Out::Str, N),
    call(
        STR,
        "concat",
        Out::Str,
        &[Param::Str, Param::Str, Param::Str, Param::Str],
    ),
    call(STR, "localeCompare", Out::Num, S),
    call(STR, "toString", Out::Str, NONE),
    call(STR, "valueOf", Out::Str, NONE),
    call(
        STR,
        "replace",
        Out::Str,
        &[Param::Pattern, Param::Replacement],
    ),
    call(
        STR,
        "replaceAll",
        Out::Str,
        &[Param::Pattern, Param::Replacement],
    ),
    read(STR, "length"),
    call(NUM, "toFixed", Out::Str, N),
    call(NUM, "toPrecision", Out::Str, N),
    call(NUM, "toExponential", Out::Str, N),
    call(NUM, "toString", Out::Str, N),
    call(NUM, "valueOf", Out::Num, NONE),
    call(BOOL, "toString", Out::Str, NONE),
    call(BOOL, "valueOf", Out::Bool, NONE),
];

pub const RELIANCE: Reliance = Reliance {
    roots: &[
        "Math",
        "JSON",
        "Number",
        "String",
        "Boolean",
        "Array",
        "Object",
        "Date",
        "RegExp",
        "parseInt",
        "parseFloat",
        "isNaN",
        "isFinite",
        "encodeURIComponent",
        "encodeURI",
        "decodeURIComponent",
        "decodeURI",
        "escape",
        "unescape",
    ],
    members: &[
        (
            "Math",
            false,
            &[
                "abs", "acos", "asin", "atan", "atan2", "cbrt", "ceil", "clz32", "cos", "exp",
                "floor", "fround", "hypot", "imul", "log", "log10", "log2", "max", "min", "pow",
                "random", "round", "sign", "sin", "sqrt", "tan", "trunc",
            ],
        ),
        ("JSON", false, &["stringify"]),
        (
            "Number",
            false,
            &[
                "isInteger",
                "isFinite",
                "isNaN",
                "isSafeInteger",
                "parseFloat",
                "parseInt",
            ],
        ),
        (
            "Number",
            true,
            &[
                "toFixed",
                "toPrecision",
                "toExponential",
                "toString",
                "valueOf",
            ],
        ),
        ("String", false, &["fromCharCode"]),
        (
            "String",
            true,
            &[
                "charAt",
                "charCodeAt",
                "codePointAt",
                "at",
                "indexOf",
                "lastIndexOf",
                "includes",
                "startsWith",
                "endsWith",
                "slice",
                "substring",
                "substr",
                "toUpperCase",
                "toLowerCase",
                "trim",
                "trimStart",
                "trimEnd",
                "padStart",
                "padEnd",
                "repeat",
                "concat",
                "localeCompare",
                "toString",
                "valueOf",
                "replace",
                "replaceAll",
            ],
        ),
        ("Boolean", false, &[]),
        ("Boolean", true, &["toString", "valueOf"]),
        ("Array", false, &["isArray"]),
        ("Array", true, &[]),
        ("Object", false, &["is"]),
        ("Object", true, &[]),
        ("Date", false, &["now"]),
        ("Date", true, &[]),
        ("RegExp", false, &[]),
        ("RegExp", true, &[]),
    ],
};

pub fn find(owner: Owner, key: &str, called: bool) -> Option<&'static Row> {
    ROWS.iter()
        .find(|row| row.owner == owner && row.key == key && row.call == called)
}

pub const ANY_ARGUMENT_CALLEES: &[(&str, &str)] = &[
    ("console", "log"),
    ("console", "warn"),
    ("console", "error"),
    ("console", "info"),
    ("console", "debug"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_key_is_relied_on_when_its_owner_is_a_built_in() {
        for row in ROWS {
            let (root, prototype) = match row.owner {
                Owner::Global => continue,
                Owner::Static(root) => (root, false),
                Owner::Proto(Kind::String) => ("String", true),
                Owner::Proto(Kind::Number) => ("Number", true),
                Owner::Proto(Kind::Boolean) => ("Boolean", true),
                Owner::Proto(_) => continue,
            };
            if !row.call || matches!(row.owner, Owner::Proto(_)) && row.key == "length" {
                continue;
            }
            assert!(
                RELIANCE.roots.contains(&root),
                "{root} is not a relied root"
            );
            let listed = RELIANCE.members.iter().any(|(owner, proto, keys)| {
                *owner == root && *proto == prototype && keys.contains(&row.key)
            });
            assert!(
                listed,
                "{root}{} {} is not relied on",
                if prototype { ".prototype" } else { "" },
                row.key
            );
        }
    }

    #[test]
    fn every_global_row_is_a_relied_root() {
        for row in ROWS.iter().filter(|row| row.owner == Owner::Global) {
            assert!(RELIANCE.roots.contains(&row.key), "{}", row.key);
        }
    }
}
