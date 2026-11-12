use super::object::{Builtin, Each, Method};

use Builtin::*;
use Method::*;

pub(super) enum Read {
    Values(&'static [Builtin]),
    World,
}

const NOTHING: Read = Read::Values(&[]);
const NUMBER: Read = Read::Values(&[PrimNumber]);
const STRING: Read = Read::Values(&[PrimString]);
const SYMBOL: Read = Read::Values(&[PrimSymbol]);

macro_rules! one {
    ($value:expr) => {
        Read::Values(&[$value])
    };
}

const PURE: Read = one!(Method(Pure));

const GLOBAL_PURE: &[&str] = &[
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURIComponent",
    "decodeURIComponent",
    "encodeURI",
    "decodeURI",
    "escape",
    "unescape",
    "atob",
    "btoa",
];

const GLOBAL_OTHER: &[&str] = &[
    "Promise",
    "Date",
    "RegExp",
    "Error",
    "TypeError",
    "RangeError",
    "SyntaxError",
    "ReferenceError",
    "URIError",
    "EvalError",
    "AggregateError",
    "Proxy",
    "Intl",
    "BigInt",
    "ArrayBuffer",
    "SharedArrayBuffer",
    "DataView",
    "Int8Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "Int16Array",
    "Uint16Array",
    "Int32Array",
    "Uint32Array",
    "Float32Array",
    "Float64Array",
    "BigInt64Array",
    "BigUint64Array",
    "WeakRef",
    "FinalizationRegistry",
    "TextDecoder",
    "TextEncoder",
    "Atomics",
    "WebAssembly",
];

pub(super) const GLOBAL_OBJECT_NAMES: &[&str] = &[
    "globalThis",
    "window",
    "self",
    "global",
    "frames",
    "top",
    "parent",
];

fn global(name: &str) -> Read {
    match name {
        "Object" => one!(ObjectCtor),
        "Function" | "eval" => one!(FunctionCtor),
        "Array" => one!(ArrayCtor),
        "String" => one!(StringCtor),
        "Number" => one!(NumberCtor),
        "Boolean" => one!(BooleanCtor),
        "Symbol" => one!(SymbolCtor),
        "Reflect" => one!(Reflect),
        "console" => one!(Console),
        "Math" => one!(MathNs),
        "JSON" => one!(Json),
        "Map" | "Set" | "WeakMap" | "WeakSet" => one!(CollectionCtor),
        "undefined" => one!(PrimUndefined),
        "NaN" | "Infinity" => NUMBER,
        _ if GLOBAL_OBJECT_NAMES.contains(&name) => one!(Global),
        _ if GLOBAL_PURE.contains(&name) => PURE,
        _ if GLOBAL_OTHER.contains(&name) => one!(Other),
        _ => Read::World,
    }
}

fn object_ctor(name: &str) -> Read {
    match name {
        "defineProperty" => one!(Method(DefineProperty)),
        "defineProperties" => one!(Method(DefineProperties)),
        "assign" => one!(Method(Assign)),
        "setPrototypeOf" => one!(Method(SetPrototypeOf)),
        "create" => one!(Method(Create)),
        "getPrototypeOf" => one!(Method(GetPrototypeOf)),
        "getOwnPropertyDescriptor" => one!(Method(GetOwnPropertyDescriptor)),
        "getOwnPropertyDescriptors" => one!(Method(GetOwnPropertyDescriptors)),
        "keys" | "getOwnPropertyNames" | "getOwnPropertySymbols" => one!(Method(Keys)),
        "values" => one!(Method(Values)),
        "entries" => one!(Method(Entries)),
        "fromEntries" => one!(Method(FromEntries)),
        "freeze" | "seal" | "preventExtensions" => one!(Method(ReturnsFirst)),
        "is" | "isFrozen" | "isSealed" | "isExtensible" | "hasOwn" => PURE,
        "groupBy" => one!(Other),
        "prototype" => one!(ObjectProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn object_proto(name: &str) -> Read {
    match name {
        "constructor" => one!(ObjectCtor),
        "__defineGetter__" => one!(Method(DefineGetter)),
        "__defineSetter__" => one!(Method(DefineSetter)),
        "__lookupGetter__" | "__lookupSetter__" => one!(Method(LookupAccessor)),
        "hasOwnProperty"
        | "isPrototypeOf"
        | "propertyIsEnumerable"
        | "toString"
        | "toLocaleString" => PURE,
        "valueOf" => one!(Method(ReturnsThis)),
        _ => NOTHING,
    }
}

fn function_ctor(name: &str) -> Read {
    match name {
        "prototype" => one!(FunctionProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn function_proto(name: &str) -> Read {
    match name {
        "call" => one!(Method(FnCall)),
        "apply" => one!(Method(FnApply)),
        "bind" => one!(Method(FnBind)),
        "toString" => one!(Method(FnToString)),
        "constructor" => one!(FunctionCtor),
        "@@hasInstance" => PURE,
        "length" => NUMBER,
        "name" => STRING,
        "arguments" | "caller" => one!(Other),
        _ => NOTHING,
    }
}

fn array_ctor(name: &str) -> Read {
    match name {
        "isArray" => PURE,
        "from" => one!(Method(ArrayFrom)),
        "of" => one!(Method(ArrayOf)),
        "fromAsync" => one!(Other),
        "prototype" => one!(ArrayProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn array_proto(name: &str) -> Read {
    match name {
        "push" | "unshift" => one!(Method(ArrayPush)),
        "pop" | "shift" | "at" => one!(Method(ArrayTake)),
        "splice" => one!(Method(ArraySplice)),
        "concat" | "toReversed" => one!(Method(ArrayCopy)),
        "slice" | "flat" => one!(Method(ArraySlice)),
        "with" | "toSpliced" => one!(Method(ArrayWith)),
        "toSorted" => one!(Method(ArrayResort)),
        "reverse" | "copyWithin" => one!(Method(ArraySelf)),
        "fill" => one!(Method(ArrayFill)),
        "sort" => one!(Method(ArraySort)),
        "forEach" | "some" | "every" | "findIndex" | "findLastIndex" => {
            one!(Method(ArrayEach(Each::Visit)))
        }
        "map" => one!(Method(ArrayEach(Each::Map))),
        "filter" => one!(Method(ArrayEach(Each::Filter))),
        "find" | "findLast" => one!(Method(ArrayEach(Each::Find))),
        "flatMap" => one!(Method(ArrayEach(Each::FlatMap))),
        "reduce" | "reduceRight" => one!(Method(ArrayReduce)),
        "join" | "toString" | "toLocaleString" => one!(Method(ArrayJoin)),
        "indexOf" | "lastIndexOf" | "includes" => PURE,
        "keys" | "values" | "entries" | "@@iterator" => one!(Method(ArrayIter)),
        "constructor" => one!(ArrayCtor),
        "length" => NUMBER,
        _ => NOTHING,
    }
}

const STRING_PURE: &[&str] = &[
    "split",
    "match",
    "matchAll",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "concat",
    "endsWith",
    "includes",
    "indexOf",
    "lastIndexOf",
    "localeCompare",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "search",
    "slice",
    "startsWith",
    "substring",
    "substr",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "toString",
    "trim",
    "trimStart",
    "trimEnd",
    "trimLeft",
    "trimRight",
    "valueOf",
    "at",
    "isWellFormed",
    "toWellFormed",
    "anchor",
    "big",
    "blink",
    "bold",
    "fixed",
    "fontcolor",
    "fontsize",
    "italics",
    "link",
    "small",
    "strike",
    "sub",
    "sup",
];

fn string_ctor(name: &str) -> Read {
    match name {
        "fromCharCode" | "fromCodePoint" | "raw" => PURE,
        "prototype" => one!(StringProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn string_proto(name: &str) -> Read {
    match name {
        "replace" | "replaceAll" => one!(Method(StringReplace)),
        "split" | "match" | "matchAll" | "search" => one!(Method(StringPattern)),
        "constructor" => one!(StringCtor),
        "length" => NUMBER,
        "@@iterator" => one!(Method(ArrayIter)),
        _ if STRING_PURE.contains(&name) => PURE,
        _ => NOTHING,
    }
}

fn number_ctor(name: &str) -> Read {
    match name {
        "isInteger" | "isFinite" | "isNaN" | "isSafeInteger" | "parseFloat" | "parseInt" => PURE,
        "EPSILON" | "MAX_SAFE_INTEGER" | "MIN_SAFE_INTEGER" | "MAX_VALUE" | "MIN_VALUE" | "NaN"
        | "NEGATIVE_INFINITY" | "POSITIVE_INFINITY" | "length" => NUMBER,
        "prototype" => one!(NumberProto),
        "name" => STRING,
        _ => NOTHING,
    }
}

fn number_proto(name: &str) -> Read {
    match name {
        "toFixed" | "toPrecision" | "toString" | "toLocaleString" | "valueOf" | "toExponential" => {
            PURE
        }
        "constructor" => one!(NumberCtor),
        _ => NOTHING,
    }
}

fn boolean_ctor(name: &str) -> Read {
    match name {
        "prototype" => one!(BooleanProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn boolean_proto(name: &str) -> Read {
    match name {
        "toString" | "valueOf" => PURE,
        "constructor" => one!(BooleanCtor),
        _ => NOTHING,
    }
}

fn symbol_ctor(name: &str) -> Read {
    match name {
        "for" | "keyFor" => PURE,
        "iterator" | "asyncIterator" | "hasInstance" | "isConcatSpreadable" | "match"
        | "matchAll" | "replace" | "search" | "species" | "split" | "toPrimitive"
        | "toStringTag" | "unscopables" | "asyncDispose" | "dispose" => SYMBOL,
        "prototype" => one!(SymbolProto),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn symbol_proto(name: &str) -> Read {
    match name {
        "toString" | "valueOf" => PURE,
        "description" => STRING,
        "constructor" => one!(SymbolCtor),
        _ => NOTHING,
    }
}

fn reflect(name: &str) -> Read {
    match name {
        "defineProperty" => one!(Method(DefineProperty)),
        "set" => one!(Method(ReflectSet)),
        "get" => one!(Method(ReflectGet)),
        "deleteProperty" => one!(Method(ReflectDeleteProperty)),
        "setPrototypeOf" => one!(Method(SetPrototypeOf)),
        "getPrototypeOf" => one!(Method(GetPrototypeOf)),
        "apply" => one!(Method(ReflectApply)),
        "construct" => one!(Method(ReflectConstruct)),
        "getOwnPropertyDescriptor" => one!(Method(GetOwnPropertyDescriptor)),
        "ownKeys" => one!(Method(Keys)),
        "has" | "isExtensible" | "preventExtensions" => PURE,
        _ => NOTHING,
    }
}

const CONSOLE_METHODS: &[&str] = &[
    "log",
    "warn",
    "error",
    "info",
    "debug",
    "trace",
    "dir",
    "dirxml",
    "table",
    "group",
    "groupEnd",
    "groupCollapsed",
    "time",
    "timeEnd",
    "timeLog",
    "timeStamp",
    "count",
    "countReset",
    "assert",
    "clear",
    "profile",
    "profileEnd",
];

fn console(name: &str) -> Read {
    if CONSOLE_METHODS.contains(&name) {
        PURE
    } else {
        Read::World
    }
}

const MATH_FUNCTIONS: &[&str] = &[
    "abs", "acos", "acosh", "asin", "asinh", "atan", "atanh", "atan2", "cbrt", "ceil", "clz32",
    "cos", "cosh", "exp", "expm1", "floor", "fround", "hypot", "imul", "log", "log1p", "log10",
    "log2", "max", "min", "pow", "random", "round", "sign", "sin", "sinh", "sqrt", "tan", "tanh",
    "trunc", "f16round", "sumPrecise",
];

const MATH_CONSTANTS: &[&str] = &[
    "E", "LN10", "LN2", "LOG10E", "LOG2E", "PI", "SQRT1_2", "SQRT2",
];

fn math(name: &str) -> Read {
    if MATH_FUNCTIONS.contains(&name) {
        PURE
    } else if MATH_CONSTANTS.contains(&name) {
        NUMBER
    } else {
        NOTHING
    }
}

fn json(name: &str) -> Read {
    match name {
        "parse" => one!(Method(JsonParse)),
        "stringify" | "rawJSON" | "isRawJSON" => one!(Other),
        _ => NOTHING,
    }
}

fn collection_ctor(name: &str) -> Read {
    match name {
        "prototype" => one!(CollectionProto),
        "groupBy" => one!(Other),
        "length" => NUMBER,
        "name" => STRING,
        _ => NOTHING,
    }
}

fn collection_proto(name: &str) -> Read {
    match name {
        "set" => one!(Method(CollectionSet)),
        "add" => one!(Method(CollectionAdd)),
        "get" => one!(Method(CollectionGet)),
        "has" | "delete" | "clear" => PURE,
        "forEach" => one!(Method(CollectionEach)),
        "keys" | "values" | "entries" | "@@iterator" => one!(Method(CollectionIter)),
        "size" => NUMBER,
        "constructor" => one!(CollectionCtor),
        "union"
        | "intersection"
        | "difference"
        | "symmetricDifference"
        | "isSubsetOf"
        | "isSupersetOf"
        | "isDisjointFrom"
        | "getOrInsert"
        | "getOrInsertComputed" => one!(Other),
        _ => NOTHING,
    }
}

fn iterator_proto(name: &str) -> Read {
    match name {
        "next" => one!(Method(IterNext)),
        "@@iterator" => one!(Method(ReturnsThis)),
        "map" | "filter" | "take" | "drop" | "flatMap" | "reduce" | "toArray" | "forEach"
        | "some" | "every" | "find" | "constructor" => one!(Other),
        _ => NOTHING,
    }
}

const OTHER_READ: &[Builtin] = &[Other, PrimString, PrimNumber, PrimBoolean];

pub(super) fn read(builtin: Builtin, name: &str) -> Read {
    match builtin {
        Global => global(name),
        ObjectCtor => object_ctor(name),
        ObjectProto => object_proto(name),
        FunctionCtor => function_ctor(name),
        FunctionProto => function_proto(name),
        ArrayCtor => array_ctor(name),
        ArrayProto => array_proto(name),
        StringCtor => string_ctor(name),
        StringProto => string_proto(name),
        NumberCtor => number_ctor(name),
        NumberProto => number_proto(name),
        BooleanCtor => boolean_ctor(name),
        BooleanProto => boolean_proto(name),
        SymbolCtor => symbol_ctor(name),
        SymbolProto => symbol_proto(name),
        Reflect => reflect(name),
        Console => console(name),
        MathNs => math(name),
        Json => json(name),
        CollectionCtor => collection_ctor(name),
        CollectionProto => collection_proto(name),
        IteratorProto => iterator_proto(name),
        Other => Read::Values(OTHER_READ),
        Machinery => Read::Values(&[Machinery, PrimString, PrimNumber]),
        PrimString | PrimNumber | PrimBoolean | PrimSymbol | PrimUndefined | PrimNull
        | Method(_) => NOTHING,
    }
}

const ALL_BUILTIN_VALUES: &[Builtin] = &[
    Global,
    ObjectCtor,
    ObjectProto,
    FunctionCtor,
    FunctionProto,
    ArrayCtor,
    ArrayProto,
    StringCtor,
    StringProto,
    NumberCtor,
    NumberProto,
    BooleanCtor,
    BooleanProto,
    SymbolCtor,
    SymbolProto,
    Reflect,
    Console,
    MathNs,
    Json,
    CollectionCtor,
    CollectionProto,
    IteratorProto,
    Other,
    PrimString,
    PrimNumber,
    PrimBoolean,
    PrimSymbol,
];

pub(super) fn read_any(builtin: Builtin) -> (Vec<Builtin>, bool) {
    match builtin {
        Global => (ALL_BUILTIN_VALUES.to_vec(), true),
        Machinery | Other => match read(builtin, "") {
            Read::Values(values) => (values.to_vec(), false),
            Read::World => (Vec::new(), true),
        },
        PrimString | PrimNumber | PrimBoolean | PrimSymbol | PrimUndefined | PrimNull
        | Method(_) => (Vec::new(), false),
        _ => (methods_of(builtin), false),
    }
}

const PROBE_NAMES: &[&str] = &[
    "defineProperty",
    "defineProperties",
    "assign",
    "setPrototypeOf",
    "create",
    "getPrototypeOf",
    "getOwnPropertyDescriptor",
    "getOwnPropertyDescriptors",
    "keys",
    "values",
    "entries",
    "fromEntries",
    "freeze",
    "is",
    "groupBy",
    "prototype",
    "constructor",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "hasOwnProperty",
    "valueOf",
    "call",
    "apply",
    "bind",
    "toString",
    "arguments",
    "isArray",
    "from",
    "of",
    "push",
    "pop",
    "splice",
    "slice",
    "reverse",
    "fill",
    "sort",
    "forEach",
    "map",
    "filter",
    "find",
    "flatMap",
    "reduce",
    "join",
    "indexOf",
    "@@iterator",
    "replace",
    "split",
    "fromCharCode",
    "isInteger",
    "toFixed",
    "for",
    "iterator",
    "description",
    "set",
    "get",
    "deleteProperty",
    "construct",
    "ownKeys",
    "has",
    "log",
    "abs",
    "PI",
    "parse",
    "stringify",
    "add",
    "next",
    "size",
    "length",
    "name",
];

fn methods_of(builtin: Builtin) -> Vec<Builtin> {
    let mut found = vec![Other];
    for name in PROBE_NAMES {
        if let Read::Values(values) = read(builtin, name) {
            for value in values {
                if !found.contains(value) {
                    found.push(*value);
                }
            }
        }
    }
    found
}

pub(super) fn protos(builtin: Builtin) -> &'static [Builtin] {
    match builtin {
        Global => &[ObjectProto],
        ObjectCtor | FunctionCtor | ArrayCtor | StringCtor | NumberCtor | BooleanCtor
        | SymbolCtor | CollectionCtor | Method(_) => &[FunctionProto],
        FunctionProto | ArrayProto | StringProto | NumberProto | BooleanProto | SymbolProto
        | CollectionProto | IteratorProto | Reflect | Console | MathNs | Json => &[ObjectProto],
        Other => &[ObjectProto, FunctionProto],
        PrimString => &[StringProto],
        PrimNumber => &[NumberProto],
        PrimBoolean => &[BooleanProto],
        PrimSymbol => &[SymbolProto],
        ObjectProto | Machinery | PrimUndefined | PrimNull => &[],
    }
}

pub(super) const PRIMITIVES: &[Builtin] = &[PrimString, PrimNumber, PrimBoolean, PrimSymbol];

const BACK_ROUTE_NAMES: &[&str] = &[
    "window",
    "self",
    "globalThis",
    "frames",
    "parent",
    "top",
    "opener",
    "defaultView",
    "contentWindow",
    "view",
    "source",
    "target",
    "currentTarget",
    "srcElement",
    "relatedTarget",
    "originalTarget",
    "explicitOriginalTarget",
    "ownerGlobal",
];

pub(super) fn is_back_route(name: &str) -> bool {
    BACK_ROUTE_NAMES.contains(&name)
}

pub(super) const WORLD_CONSTRUCTOR_NAMES: &[&str] = &["constructor", "__proto__", "prototype"];

pub(super) const WORLD_CONSTRUCTORS: &[Builtin] = &[
    ObjectCtor,
    ObjectProto,
    FunctionCtor,
    FunctionProto,
    ArrayCtor,
    ArrayProto,
    StringCtor,
    StringProto,
    NumberCtor,
    NumberProto,
    BooleanCtor,
    BooleanProto,
    SymbolCtor,
    SymbolProto,
    CollectionCtor,
    CollectionProto,
    IteratorProto,
    Other,
];
