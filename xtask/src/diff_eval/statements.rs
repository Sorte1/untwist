use super::pool::{Rng, POOL};

const STRICT_PERCENT: u64 = 30;
const LOGGED_PERCENT: u64 = 35;
const ALIASING_PERCENT: u64 = 10;

const LOGGED_VALUES: &[&str] = &[
    "{ valueOf: function () { log += 'a'; return 2; } }",
    "{ toString: function () { log += 'b'; return 'x'; } }",
    "{ valueOf: function () { log += 'c'; return {}; }, toString: function () { log += 'd'; return '3'; } }",
    "{ valueOf: function () { log += 'e'; throw 7; } }",
    "(log += 'v', 5)",
    "(log += 'w', 'p')",
    "(log += 'z', { q: 1 })",
];

const KEYS: &[&str] = &[
    "'a'",
    "'p'",
    "'q'",
    "'x'",
    "'length'",
    "'name'",
    "'prototype'",
    "'constructor'",
    "'__proto__'",
    "'toString'",
    "'valueOf'",
    "'caller'",
    "'arguments'",
    "'lastIndex'",
    "'index'",
    "'0'",
    "'1'",
    "'-0'",
    "'1e3'",
    "'01'",
    "0",
    "1",
    "2",
    "-0",
    "-1",
    "1.5",
    "4294967295",
    "4294967294",
    "NaN",
    "Infinity",
    "undefined",
    "null",
    "true",
    "'source'",
    "'global'",
    "'flags'",
    "'callee'",
];

const LOGGED_KEYS: &[&str] = &[
    "{ toString: function () { log += 'k'; return 'p'; } }",
    "{ toString: function () { log += 'k'; return 'length'; } }",
    "{ valueOf: function () { log += 'k'; return 1; } }",
    "{ toString: function () { log += 'k'; return {}; } }",
    "{ toString: function () { log += 'k'; throw 3; } }",
    "[1]",
    "['a', 'b']",
    "{}",
    "function () {}",
    "/x/",
];

const TARGETS: &[&str] = &[
    "{}",
    "{ a: 1, p: 2 }",
    "{ length: 2, 0: 'x', 1: 'y' }",
    "[1, 2, 3]",
    "[]",
    "['a', 'b']",
    "function f(a, b) { return a; }",
    "function () {}",
    "() => 1",
    "function* g() { yield 1; }",
    "async function h() {}",
    "({ m() { return 1; } }).m",
    "(function (a, b) { return a; }).bind(null, 1)",
    "(function* () {}).bind(null)",
    "/a/g",
    "(function () { return arguments; })(1, 2)",
    "Object.create({ k: 1 })",
    "new (function F() { this.a = 1; })()",
    "'str'",
    "5",
    "true",
    "null",
    "undefined",
];

const BINARY: &[&str] = &[
    "+",
    "-",
    "*",
    "/",
    "%",
    "**",
    "==",
    "!=",
    "===",
    "!==",
    "<",
    ">",
    "<=",
    ">=",
    "&",
    "|",
    "^",
    "<<",
    ">>",
    ">>>",
    "in",
    "instanceof",
    "&&",
    "||",
    "??",
    ",",
];

const UNARY: &[&str] = &["typeof ", "void ", "!", "-", "+", "~"];

const COMPOUND: &[&str] = &[
    "=", "+=", "-=", "*=", "/=", "%=", "**=", "&=", "|=", "^=", "<<=", ">>=", ">>>=", "&&=", "||=",
    "??=",
];

const UPDATE: &[(&str, &str)] = &[("", "++"), ("", "--"), ("++", ""), ("--", "")];

const HOST_NAMES: &[&str] = &[
    "global",
    "globalThis",
    "process",
    "Buffer",
    "setTimeout",
    "setInterval",
    "clearTimeout",
    "URL",
    "URLSearchParams",
    "TextEncoder",
    "TextDecoder",
    "fetch",
    "structuredClone",
    "queueMicrotask",
    "require",
    "module",
    "exports",
    "__dirname",
    "__filename",
    "console",
    "window",
    "self",
    "document",
    "navigator",
    "frames",
    "location",
    "atob",
    "btoa",
    "performance",
    "crypto",
    "WebAssembly",
    "Intl",
    "Iterator",
    "SharedArrayBuffer",
    "Atomics",
    "escape",
    "unescape",
    "undefinedName",
    "Symbol",
    "BigInt",
    "Proxy",
    "Reflect",
    "Math",
    "JSON",
];

const THROWERS: &[&str] = &[
    "null.x",
    "undefined[k]",
    "o.nope()",
    "(void 0)()",
    "new 1",
    "k in 'str'",
    "throw v",
    "nope",
    "(function () { 'use strict'; nope = 1; })()",
    "JSON.parse('{')",
    "decodeURIComponent('%')",
    "RegExp('[')",
    "'a'.repeat(-1)",
    "(1).toFixed(200)",
    "o[k] = v",
    "o[k]()",
    "[].reduce(function () {})",
    "Object.defineProperty(1, 'a', {})",
    "x.y.z",
    "(function f() { 'use strict'; f = 1; })()",
];

const OBSERVERS: &[&str] = &[
    "e instanceof TypeError",
    "e instanceof Error",
    "e.name",
    "e.message",
    "String(e)",
    "typeof e",
    "e === v",
    "e.constructor === TypeError",
    "Object.prototype.toString.call(e)",
    "e",
    "'' + e",
];

fn one<'s>(rng: &mut Rng, list: &[&'s str]) -> &'s str {
    list[rng.below(list.len())]
}

fn value(rng: &mut Rng) -> &'static str {
    if rng.chance(LOGGED_PERCENT) {
        return one(rng, LOGGED_VALUES);
    }
    POOL[rng.below(POOL.len())].source
}

fn key(rng: &mut Rng) -> &'static str {
    if rng.chance(LOGGED_PERCENT) {
        return one(rng, LOGGED_KEYS);
    }
    one(rng, KEYS)
}

fn wrap(rng: &mut Rng, body: &str) -> String {
    let directive = if rng.chance(STRICT_PERCENT) {
        "'use strict'; "
    } else {
        ""
    };
    format!("(function () {{ {directive}var log = ''; {body} }})()")
}

fn binary(rng: &mut Rng) -> (String, String) {
    let op = one(rng, BINARY);
    let body = format!(
        "var a = {}; var b = {}; var r = (a {op} b); return [r, log];",
        value(rng),
        value(rng)
    );
    (format!("stmt a {op} b"), body)
}

fn unary(rng: &mut Rng) -> (String, String) {
    let op = one(rng, UNARY);
    let body = format!("var a = {}; var r = {op}a; return [r, log];", value(rng));
    (format!("stmt {}a", op.trim()), body)
}

fn member_read(rng: &mut Rng) -> (String, String) {
    let body = format!(
        "var o = {}; var k = {}; var r = o[k]; return [r, log];",
        one(rng, TARGETS),
        key(rng)
    );
    ("stmt o[k]".to_string(), body)
}

fn write_read(rng: &mut Rng) -> (String, String) {
    let op = one(rng, COMPOUND);
    let body = format!(
        "var o = {}; var k = {}; var r = (o[k] {op} {}); return [r, o[k], o, log];",
        one(rng, TARGETS),
        key(rng),
        value(rng)
    );
    (format!("stmt o[k] {op} v"), body)
}

fn static_write(rng: &mut Rng) -> (String, String) {
    let name = one(rng, KEYS).trim_matches('\'');
    let usable = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic() || c == '_');
    let name = if usable { name } else { "name" };
    let body = format!(
        "var o = {}; var r = (o.{name} = {}); return [r, o.{name}, o, log];",
        one(rng, TARGETS),
        value(rng)
    );
    (format!("stmt o.{name} = v"), body)
}

fn update(rng: &mut Rng) -> (String, String) {
    let (before, after) = UPDATE[rng.below(UPDATE.len())];
    let body = format!(
        "var o = {}; var k = {}; var r = {before}o[k]{after}; return [r, o[k], o, log];",
        one(rng, TARGETS),
        key(rng)
    );
    (format!("stmt {before}o[k]{after}"), body)
}

fn in_delete(rng: &mut Rng) -> (String, String) {
    let body = format!(
        "var o = {}; var k = {}; var a = k in o; var d = delete o[k]; return [a, d, k in o, o, log];",
        one(rng, TARGETS),
        key(rng)
    );
    ("stmt in/delete".to_string(), body)
}

fn typeof_case(rng: &mut Rng) -> (String, String) {
    let name = one(rng, HOST_NAMES);
    let body = format!("return [typeof {name}, typeof {name} === 'undefined'];");
    (format!("stmt typeof {name}"), body)
}

pub fn statement_case(rng: &mut Rng) -> (String, String) {
    if rng.chance(ALIASING_PERCENT) {
        return super::aliasing::aliasing_case(rng);
    }
    let (label, body) = match rng.below(14) {
        0 | 1 => binary(rng),
        2 => unary(rng),
        3 => member_read(rng),
        4 | 5 => write_read(rng),
        6 => static_write(rng),
        7 => update(rng),
        8 => in_delete(rng),
        9 => typeof_case(rng),
        10 => super::flow::loop_case(rng),
        11 => super::flow::block_function_case(rng),
        12 => try_case(rng),
        _ => super::flow::sample_case(rng),
    };
    (label, wrap(rng, &body))
}

fn try_case(rng: &mut Rng) -> (String, String) {
    let thrower = one(rng, THROWERS);
    let observer = one(rng, OBSERVERS);
    let body = format!(
        "var o = {}; var k = {}; var v = {}; try {{ log += 's'; {thrower}; log += 'n'; }} catch (e) {{ log += 'c'; return ['c', {observer}, log]; }} finally {{ log += 'f'; }} return ['n', log];",
        one(rng, TARGETS),
        key(rng),
        value(rng)
    );
    (format!("stmt try {thrower} / {observer}"), body)
}

pub fn pick_value(rng: &mut Rng) -> &'static str {
    value(rng)
}

pub fn pick_key(rng: &mut Rng) -> &'static str {
    key(rng)
}

pub fn pick_target(rng: &mut Rng) -> &'static str {
    one(rng, TARGETS)
}
