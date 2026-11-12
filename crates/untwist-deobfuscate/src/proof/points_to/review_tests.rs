use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use std::time::Duration;

use super::model::{self, Read};
use super::object::Builtin;
use super::points_to_sites;

const HEAD: &str = "var j = ['fromCharCode'][0], k = ['String'][0], E = function () { return 'X'; }, F = {}; F[j] = E; var t = this; var Z = t[k];";

fn site_count(source: &str, source_type: SourceType) -> usize {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, source_type).parse();
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let scoping = semantic.into_scoping();
    points_to_sites(&parsed.program, &scoping, &FxHashMap::default(), &[]).len()
}

fn framed(patch: &str) -> String {
    format!("(function () {{ {HEAD} {patch} }})();")
}

fn unseen(rows: &[(&str, &str)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, patch)| site_count(&framed(patch), SourceType::cjs()) == 0)
        .map(|(label, _)| label.to_string())
        .collect()
}

#[test]
fn a_program_object_that_never_meets_a_built_in_is_not_a_site() {
    assert_eq!(
        site_count(&framed("var o = {}; o[j] = E;"), SourceType::cjs()),
        0
    );
}

#[test]
fn a_root_function_of_a_script_runs_with_the_global_object_as_this() {
    let source = format!("{HEAD} function init() {{ this[k][j] = E; }} setTimeout('init()', 0);");
    assert!(site_count(&source, SourceType::script()) > 0);
    let alone = "function init() { this[k][j] = E; }";
    assert!(site_count(alone, SourceType::script()) > 0);
}

const SPREAD_AND_APPLY: &[(&str, &str)] = &[
    ("seal-spread", "var s = Object.seal; s(...[Z])[j] = E;"),
    ("object-spread", "Object(...[Z])[j] = E;"),
    (
        "create-spread",
        "var c = Object.create; Object.getPrototypeOf(c(...[Z]))[j] = E;",
    ),
    (
        "definer-apply",
        "var d = Object.defineProperty; var a = [Z, j, { value: E }]; d.apply(null, a);",
    ),
    (
        "definer-spread",
        "var d = Object.defineProperty; var a = [Z, j, { value: E }]; d(...a);",
    ),
    (
        "definer-reflect-apply",
        "var d = Object.defineProperty; var a = [Z, j, { value: E }]; Reflect.apply(d, null, a);",
    ),
    (
        "assign-apply",
        "var d = Object.assign; d.apply(null, [Z, F]);",
    ),
    (
        "apply-of-a-spread-list",
        "var f = function (q) { q[j] = E; }; f.apply(...[null, [Z]]);",
    ),
];

#[test]
fn a_built_in_called_with_spread_or_apply_arguments_keeps_them() {
    assert_eq!(unseen(SPREAD_AND_APPLY), Vec::<String>::new());
}

#[test]
fn a_partly_spread_definer_has_an_unknown_key() {
    let source = framed("var d = Object.defineProperty; var a = [j, { value: E }]; d(Z, ...a);");
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, &source, SourceType::cjs()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let sites = points_to_sites(&parsed.program, &scoping, &FxHashMap::default(), &[]);
    assert_eq!(sites.len(), 1);
    let used = crate::proof::UsedNames::default();
    assert!(crate::proof::alias_refusal(&sites, &used, &source).is_some());
}

const UNDEFINED_KEYS: &[(&str, &str)] = &[
    (
        "undefined-key",
        "var o = { undefined: Z }; var u; o[u][j] = E;",
    ),
    (
        "undefined-in-a-concat",
        "var u; var o = {}; o['aundefined'] = Z; o['a' + u][j] = E;",
    ),
    (
        "missing-argument-key",
        "var o = { undefined: Z }; function g(n) { return o[n]; } g()[j] = E;",
    ),
    ("null-key", "var o = { null: Z }; o[null][j] = E;"),
    (
        "no-return-key",
        "var o = { undefined: Z }; function h() {} o[h()][j] = E;",
    ),
];

#[test]
fn an_undefined_or_null_key_reads_and_writes_a_member() {
    assert_eq!(unseen(UNDEFINED_KEYS), Vec::<String>::new());
}

const ACCESSOR_RECEIVERS: &[(&str, &str)] = &[
    ("getter-on-a-host-array", "var s; Object.defineProperty(Array.prototype, 'zz', { get: function () { s = this; return 1; } }); host.list().zz; s[0].String[j] = E;"),
    ("setter-on-a-host-array", "Object.defineProperty(Array.prototype, 'zz', { set: function (v) { v[j] = E; } }); host.list().zz = Z;"),
    ("setter-on-a-primitive", "Object.defineProperty(Z.prototype, 'zz', { set: function (v) { v[j] = E; } }); 'abc'.zz = Z;"),
    ("reflect-get-receiver", "var o = { get zz() { return this; } }; Reflect.get(o, 'zz', Z)[j] = E;"),
    ("reflect-set-receiver", "Reflect.set({}, j, E, Z);"),
    ("reflect-set-receiver-alias", "var R = Reflect; R.set({}, j, E, Z);"),
];

#[test]
fn accessors_run_with_host_primitive_and_reflect_receivers() {
    assert_eq!(unseen(ACCESSOR_RECEIVERS), Vec::<String>::new());
}

#[test]
fn reflect_construct_takes_the_prototype_of_new_target() {
    let rows = [(
        "construct-new-target",
        "function N() {} N.prototype = Z; var o = Reflect.construct(function () {}, [], N); Object.getPrototypeOf(o)[j] = E;",
    )];
    assert_eq!(unseen(&rows), Vec::<String>::new());
}

#[test]
fn copying_a_built_in_copies_every_member_it_may_have() {
    let rows = [
        (
            "assign-window",
            "var o = Object.assign({}, window); o.document.defaultView.String[j] = E;",
        ),
        (
            "spread-window",
            "var o = { ...window }; o.document.defaultView.String[j] = E;",
        ),
    ];
    assert_eq!(unseen(&rows), Vec::<String>::new());
}

const UNLISTED_METHODS: &[(&str, &str)] = &[
    (
        "set-union",
        "var st = new Set([{ S: Z }]); var un = st.union(new Set()); for (var v of un) v.S[j] = E;",
    ),
    (
        "iterator-for-each",
        "[{ S: Z }].values().forEach(function (v) { v.S[j] = E; });",
    ),
    (
        "iterator-map",
        "[{ S: Z }].values().map(function (v) { v.S[j] = E; }).next();",
    ),
    (
        "array-from-async",
        "Array.fromAsync([{ S: Z }]).then(function (a) { a[0].S[j] = E; });",
    ),
    (
        "map-get-or-insert",
        "var mp = new Map(); mp.getOrInsert(1, { S: Z }).S[j] = E;",
    ),
];

#[test]
fn a_standard_method_the_tables_do_not_list_keeps_its_flows() {
    assert_eq!(unseen(UNLISTED_METHODS), Vec::<String>::new());
}

const NATIVE_FLOWS: &[(&str, &str)] = &[
    (
        "symbol-replace-hook",
        "var pt = {}; pt[Symbol.replace] = function (s, r) { r[j] = E; }; 'a'.replace(pt, Z);",
    ),
    (
        "symbol-split-hook",
        "var pt = {}; pt[Symbol.split] = function (s, r) { r[j] = E; }; 'a'.split(pt, Z);",
    ),
    (
        "concat-on-an-object",
        "[].concat.call({ S: Z })[0].S[j] = E;",
    ),
    (
        "map-entry-pairs",
        "var mp = new Map(); mp.set(1, { S: Z }); for (var [a, v] of mp) v.S[j] = E;",
    ),
    (
        "array-entry-pairs",
        "for (var [a, v] of [{ S: Z }].entries()) v.S[j] = E;",
    ),
];

#[test]
fn native_calls_hand_on_every_flow() {
    assert_eq!(unseen(NATIVE_FLOWS), Vec::<String>::new());
}

#[test]
fn two_bind_applications_at_one_call_keep_both() {
    let rows = [(
        "bind-twice-at-one-span",
        "var g = function () { this[j] = E; }; var f = host.see() ? function () {} : Function.prototype.bind; var kk = host.see() ? 'bind' : 'call'; var r = f[kk](g, Z); r();",
    )];
    assert_eq!(unseen(&rows), Vec::<String>::new());
}

#[test]
fn a_block_function_called_from_outside_its_block_is_the_same_function() {
    let rows = [("annex-b-call", "{ function fb(o) { o[j] = E; } } fb(Z);")];
    assert_eq!(unseen(&rows), Vec::<String>::new());
}

const BUILT_IN_ACCESSORS: &[(&str, &str)] = &[
    ("lookup-proto-getter", "var g = Object.prototype.__lookupGetter__('__proto__'); g.call('abc').constructor[j] = E;"),
    ("descriptor-proto-getter", "var g = Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').get; g.call('abc').constructor[j] = E;"),
    ("function-arguments-definer", "function f() { f.arguments[0](Z, j, { value: E }); } f(Object.defineProperty);"),
    ("function-arguments-value", "function f() { f.arguments[0][j] = E; } f(Z);"),
];

#[test]
fn built_in_accessors_and_function_arguments_return_what_they_hold() {
    assert_eq!(unseen(BUILT_IN_ACCESSORS), Vec::<String>::new());
}

const ELEMENT_READ_STEPS: u64 = 20_000;

#[test]
fn applying_push_to_an_array_reads_only_its_elements() {
    for source in [
        "var a = []; a.push.apply(a, [1]);",
        "var a = []; a.push.apply(a, [{}]);",
        "function f() { var a = []; a.push.apply(a, arguments); } f(1);",
    ] {
        let alloc = Allocator::default();
        let parsed = Parser::new(&alloc, source, SourceType::cjs()).parse();
        let scoping = SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping();
        let decoded = FxHashMap::default();
        let mut generator = super::generate::Gen::new(&parsed.program, &scoping, &decoded, &[]);
        generator.run(&parsed.program);
        generator.g.solve(None);
        assert!(!generator.g.exhausted(), "{source}");
        let steps = generator.g.steps();
        assert!(steps < ELEMENT_READ_STEPS, "{source}: {steps} steps");
    }
}

const STANDARD_MEMBERS: &[(Builtin, &[&str])] = &[
    (
        Builtin::ObjectCtor,
        &[
            "assign",
            "create",
            "defineProperties",
            "defineProperty",
            "entries",
            "freeze",
            "fromEntries",
            "getOwnPropertyDescriptor",
            "getOwnPropertyDescriptors",
            "getOwnPropertyNames",
            "getOwnPropertySymbols",
            "getPrototypeOf",
            "groupBy",
            "hasOwn",
            "is",
            "isExtensible",
            "isFrozen",
            "isSealed",
            "keys",
            "preventExtensions",
            "prototype",
            "seal",
            "setPrototypeOf",
            "values",
            "length",
            "name",
        ],
    ),
    (
        Builtin::ObjectProto,
        &[
            "constructor",
            "hasOwnProperty",
            "isPrototypeOf",
            "propertyIsEnumerable",
            "toLocaleString",
            "toString",
            "valueOf",
            "__defineGetter__",
            "__defineSetter__",
            "__lookupGetter__",
            "__lookupSetter__",
        ],
    ),
    (
        Builtin::FunctionProto,
        &[
            "apply",
            "bind",
            "call",
            "constructor",
            "toString",
            "@@hasInstance",
            "length",
            "name",
            "arguments",
            "caller",
        ],
    ),
    (
        Builtin::ArrayCtor,
        &[
            "from",
            "fromAsync",
            "isArray",
            "of",
            "prototype",
            "length",
            "name",
        ],
    ),
    (
        Builtin::ArrayProto,
        &[
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
            "toLocaleString",
            "toReversed",
            "toSorted",
            "toSpliced",
            "toString",
            "unshift",
            "values",
            "with",
            "@@iterator",
            "constructor",
            "length",
        ],
    ),
    (
        Builtin::StringCtor,
        &[
            "fromCharCode",
            "fromCodePoint",
            "raw",
            "prototype",
            "length",
            "name",
        ],
    ),
    (
        Builtin::StringProto,
        &[
            "at",
            "charAt",
            "charCodeAt",
            "codePointAt",
            "concat",
            "endsWith",
            "includes",
            "indexOf",
            "isWellFormed",
            "lastIndexOf",
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
            "slice",
            "split",
            "startsWith",
            "substring",
            "substr",
            "toLocaleLowerCase",
            "toLocaleUpperCase",
            "toLowerCase",
            "toString",
            "toUpperCase",
            "toWellFormed",
            "trim",
            "trimEnd",
            "trimStart",
            "trimLeft",
            "trimRight",
            "valueOf",
            "@@iterator",
            "constructor",
            "length",
        ],
    ),
    (
        Builtin::SymbolCtor,
        &[
            "asyncDispose",
            "asyncIterator",
            "dispose",
            "for",
            "hasInstance",
            "isConcatSpreadable",
            "iterator",
            "keyFor",
            "match",
            "matchAll",
            "replace",
            "search",
            "species",
            "split",
            "toPrimitive",
            "toStringTag",
            "unscopables",
            "prototype",
            "length",
            "name",
        ],
    ),
    (
        Builtin::Reflect,
        &[
            "apply",
            "construct",
            "defineProperty",
            "deleteProperty",
            "get",
            "getOwnPropertyDescriptor",
            "getPrototypeOf",
            "has",
            "isExtensible",
            "ownKeys",
            "preventExtensions",
            "set",
            "setPrototypeOf",
        ],
    ),
    (
        Builtin::MathNs,
        &[
            "abs",
            "acos",
            "acosh",
            "asin",
            "asinh",
            "atan",
            "atanh",
            "atan2",
            "cbrt",
            "ceil",
            "clz32",
            "cos",
            "cosh",
            "exp",
            "expm1",
            "f16round",
            "floor",
            "fround",
            "hypot",
            "imul",
            "log",
            "log1p",
            "log10",
            "log2",
            "max",
            "min",
            "pow",
            "random",
            "round",
            "sign",
            "sin",
            "sinh",
            "sqrt",
            "sumPrecise",
            "tan",
            "tanh",
            "trunc",
            "E",
            "LN10",
            "LN2",
            "LOG10E",
            "LOG2E",
            "PI",
            "SQRT1_2",
            "SQRT2",
        ],
    ),
    (
        Builtin::Json,
        &["parse", "stringify", "rawJSON", "isRawJSON"],
    ),
    (
        Builtin::CollectionCtor,
        &["groupBy", "prototype", "length", "name"],
    ),
    (
        Builtin::CollectionProto,
        &[
            "add",
            "clear",
            "delete",
            "difference",
            "entries",
            "forEach",
            "get",
            "getOrInsert",
            "getOrInsertComputed",
            "has",
            "intersection",
            "isDisjointFrom",
            "isSubsetOf",
            "isSupersetOf",
            "keys",
            "set",
            "size",
            "symmetricDifference",
            "union",
            "values",
            "@@iterator",
            "constructor",
        ],
    ),
    (
        Builtin::IteratorProto,
        &[
            "next",
            "drop",
            "every",
            "filter",
            "find",
            "flatMap",
            "forEach",
            "map",
            "reduce",
            "some",
            "take",
            "toArray",
            "@@iterator",
            "constructor",
        ],
    ),
];

#[test]
fn every_standard_member_of_a_modelled_built_in_reads_as_something() {
    let mut absent = Vec::new();
    for (builtin, names) in STANDARD_MEMBERS {
        for name in *names {
            if let Read::Values([]) = model::read(*builtin, name) {
                absent.push(format!("{builtin:?}.{name}"));
            }
        }
    }
    assert!(absent.is_empty(), "{absent:?}");
}

#[test]
fn a_solve_past_the_deadline_gives_up_through_one_fallback_site() {
    let mut source = String::from("var x;");
    for _ in 0..300 {
        source.push_str("var oI = {}; x = oI; oI.f = {};");
    }
    for _ in 0..300 {
        source.push_str("x.f = x.f;");
    }
    let limits = untwist_core::budget::Limits {
        memory: None,
        deadline: Some(untwist_core::budget::Deadline::after(Duration::ZERO)),
    };
    let scope = untwist_core::budget::enter(limits);
    let count = site_count(&source, SourceType::cjs());
    drop(scope);
    assert_eq!(count, 1);
}

const CALLER_REACH: &[(&str, &str)] = &[
    (
        "host-reads-a-caller-argument",
        "function f(a) { host.peek(); } f(Z); var w = host.back(); w[j] = E;",
    ),
    (
        "host-writes-a-caller-argument",
        "host.keep(Z); function f(a) { host.swap(); a[j] = E; } f({});",
    ),
    (
        "host-reads-a-caller-argument-without-parameters",
        "function f() { host.peek(); } f(Z); host.back()[j] = E;",
    ),
];

#[test]
fn a_foreign_callee_reaches_the_arguments_of_a_sloppy_caller() {
    assert_eq!(unseen(CALLER_REACH), Vec::<String>::new());
}

#[test]
fn an_index_read_off_a_foreign_value_may_be_the_global_object() {
    let rows = [("index-of-a-host-list", "host.list()[0].String[j] = E;")];
    assert_eq!(unseen(&rows), Vec::<String>::new());
}

#[test]
fn a_strict_caller_keeps_its_arguments() {
    let source = "(function () { 'use strict'; var o = {}; function f(a) { host.peek(); a[j] = E; } f(o); })();";
    assert_eq!(site_count(source, SourceType::cjs()), 0);
}

const CALLER_READS: &[(&str, &str)] = &[
    ("caller-called-back", "function p(o) { if (o) { o[j] = E; return; } g()(Z); } function g() { return g.caller; } p();"),
    ("caller-arguments", "function f() { return f.caller; } function h(q) { var c = f(); c.arguments[0][j] = E; } h(Z);"),
    ("caller-under-an-unknown-key", "function p(o) { if (o) { o[j] = E; return; } g()(Z); } function g() { return g[host.key()]; } p();"),
];

#[test]
fn a_caller_read_on_a_program_function_may_be_any_program_function() {
    assert_eq!(unseen(CALLER_READS), Vec::<String>::new());
}
