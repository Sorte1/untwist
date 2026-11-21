use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use super::generate::Gen;
use super::points_to_sites;

const HEAD: &str = "var j = ['fromCharCode'][0], k = ['String'][0], E = function () { return 'X'; }; var t = this; var Z = t[k];";

fn site_count(source: &str) -> usize {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::cjs()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    points_to_sites(&parsed.program, &scoping, &FxHashMap::default(), &[]).len()
}

fn framed(patch: &str) -> String {
    format!("(function () {{ {HEAD} {patch} }})();")
}

fn unseen(rows: &[(&str, &str)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, patch)| site_count(&framed(patch)) == 0)
        .map(|(label, _)| label.to_string())
        .collect()
}

fn seen(rows: &[(&str, &str)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, patch)| site_count(&framed(patch)) > 0)
        .map(|(label, _)| label.to_string())
        .collect()
}

fn held(source: &str) -> (bool, u64) {
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::cjs()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let decoded = FxHashMap::default();
    let mut generator = Gen::new(&parsed.program, &scoping, &decoded, &[]);
    generator.run(&parsed.program);
    generator.g.solve(None);
    (generator.g.exhausted(), generator.g.held_and_cap().0)
}

const NUMBER_KEYS: &[(&str, &str)] = &[
    (
        "number-key-reads-a-number-name",
        "var o = {}; o['1'] = Z; var i = 0; i++; o[i][j] = E;",
    ),
    (
        "number-key-reads-nan",
        "var o = {}; o.NaN = Z; o[0 / 0][j] = E;",
    ),
    (
        "number-key-reads-a-fraction-name",
        "var o = {}; o['0.5'] = Z; o[1 / 2][j] = E;",
    ),
    (
        "number-key-reads-an-unknown-write",
        "var o = {}; o[host.key()] = Z; o[2 * 1][j] = E;",
    ),
    (
        "number-key-write-reaches-a-number-name",
        "var o = {}; o[1 * 1] = Z; o['1'][j] = E;",
    ),
    (
        "number-key-write-reaches-a-literal-index",
        "var o = {}; o[1 * 1] = Z; o[1][j] = E;",
    ),
    (
        "number-key-write-reaches-for-in",
        "var o = {}; o[1 * 1] = Z; for (var p in o) o[p][j] = E;",
    ),
    (
        "number-key-write-reaches-an-unknown-read",
        "var o = {}; o[1 * 1] = Z; o[host.key()][j] = E;",
    ),
    (
        "number-key-write-reaches-the-elements",
        "var o = []; o[1 * 1] = Z; o.forEach(function (v) { v[j] = E; });",
    ),
    (
        "number-key-write-reaches-a-copy",
        "var o = {}; o[1 * 1] = Z; var c = Object.assign({}, o); c[1][j] = E;",
    ),
    (
        "number-key-reads-a-prototype-member",
        "var p = { 3: Z }; var o = Object.create(p); o[1 + 2][j] = E;",
    ),
    ("array-literal-index", "var a = [Z]; a[0][j] = E;"),
    ("array-literal-number-key", "var a = [Z]; a[0 * 1][j] = E;"),
    (
        "array-literal-for-of",
        "var a = [Z]; for (var v of a) v[j] = E;",
    ),
    (
        "array-literal-each",
        "var a = [Z]; a.forEach(function (v) { v[j] = E; });",
    ),
    (
        "array-literal-unknown-key",
        "var a = [Z]; a[host.key()][j] = E;",
    ),
    ("pushed-element", "var a = []; a.push(Z); a[0][j] = E;"),
    (
        "pushed-element-spread",
        "var a = []; a.push(Z); var b = [...a]; b[0][j] = E;",
    ),
    (
        "arguments-literal-index",
        "function f() { arguments[0][j] = E; } f(Z);",
    ),
    (
        "arguments-number-key",
        "function f() { arguments[0 * 1][j] = E; } f(Z);",
    ),
    (
        "mapped-arguments-write",
        "function f(a) { arguments[0 * 1] = Z; a[j] = E; } f({});",
    ),
    ("rest-parameter", "function f(...r) { r[0][j] = E; } f(Z);"),
    ("array-method-result", "var a = [Z].slice(); a[0][j] = E;"),
];

#[test]
fn a_number_key_reaches_every_member_a_number_can_name() {
    assert_eq!(unseen(NUMBER_KEYS), Vec::<String>::new());
}

const NUMBER_KEY_MISSES: &[(&str, &str)] = &[
    (
        "number-key-write-misses-a-name",
        "var o = {}; o[1 * 1] = Z; o.name[j] = E;",
    ),
    (
        "array-element-misses-length",
        "var a = [Z]; a.length[j] = E;",
    ),
    (
        "pushed-element-misses-a-method",
        "var a = []; a.push(Z); a.foo[j] = E;",
    ),
];

#[test]
fn a_number_key_misses_members_no_number_can_name() {
    assert_eq!(seen(NUMBER_KEY_MISSES), Vec::<String>::new());
}

const EVERY_FUNCTION: &[(&str, &str)] = &[
    (
        "caller-gets-a-field-store",
        "function g() { if (g.box) { g.box[j] = E; } } function f() { f.caller.box = Z; } function h() { f(); } h(); g();",
    ),
    (
        "caller-gets-an-unknown-key-store",
        "function g() { if (g.box) { g.box[j] = E; } } function f() { f.caller[host.key()] = Z; } function h() { f(); } h(); g();",
    ),
    (
        "caller-is-constructed",
        "function C() { Object.getPrototypeOf(this)[j] = E; } function f() { return f.caller; } function h() { var c = f(); var o = new c(); } C.prototype = Z; h();",
    ),
    (
        "caller-is-called-with-a-receiver",
        "function g() { this[j] = E; } function f() { return f.caller; } function h() { var c = f(); c.call(Z); } h();",
    ),
    (
        "caller-is-called-with-spread",
        "function p(o) { if (o) { o[j] = E; return; } g()(...[Z]); } function g() { return g.caller; } p();",
    ),
    (
        "caller-prototype-is-read",
        "function C() {} C.prototype.box = Z; function f() { return f.caller; } function h() { f().prototype.box[j] = E; } h(); new C();",
    ),
    (
        "caller-escapes",
        "function g(o) { if (o) o[j] = E; } function f() { host.keep(f.caller); } f(); host.back()(Z);",
    ),
    (
        "caller-keys",
        "function g() {} g.box = Z; function f() { var c = f.caller; for (var p in c) c[p][j] = E; } f();",
    ),
    (
        "caller-assigned",
        "function g() {} g.box = Z; function f() { var c = Object.assign({}, f.caller); c.box[j] = E; } f();",
    ),
    (
        "caller-proto-set",
        "function f() { Object.setPrototypeOf(f.caller, { box: Z }); } function g() { g.box[j] = E; } function h() { f(); } h(); g();",
    ),
];

#[test]
fn one_object_for_every_program_function_keeps_their_flows() {
    assert_eq!(unseen(EVERY_FUNCTION), Vec::<String>::new());
}

const COVERED_BY_WORLD: &[(&str, &str)] = &[
    (
        "host-or-string-constructor",
        "var s = host.pick() ? host.get() : 'abc'; s.constructor[j] = E;",
    ),
    (
        "host-or-string-prototype-getter",
        "Object.defineProperty(String.prototype, 'zz', { get: function () { return Z; } }); var s = host.pick() ? host.get() : 'abc'; s.zz[j] = E;",
    ),
    (
        "host-or-string-prototype-member",
        "String.prototype.zz = Z; var s = host.pick() ? host.get() : 'abc'; s.zz[j] = E;",
    ),
    (
        "host-or-number-prototype-member",
        "Number.prototype.zz = Z; var s = host.pick() ? host.get() : 1 * 2; s.zz[j] = E;",
    ),
    (
        "host-or-string-setter",
        "Object.defineProperty(String.prototype, 'zz', { set: function (v) { v[j] = E; } }); var s = host.pick() ? host.get() : 'abc'; s.zz = Z;",
    ),
    (
        "host-or-string-key",
        "var o = { abc: Z }; var s = host.pick() ? host.get() : 'abc'; o[s][j] = E;",
    ),
    (
        "escaped-built-in-member",
        "host.keep(Function.prototype.bind); var b = host.back(); b.zz = Z; Function.prototype.bind.zz[j] = E;",
    ),
    (
        "escaped-built-in-unknown-member",
        "host.keep(Function.prototype.bind); var b = host.back(); b[host.key()] = Z; Function.prototype.bind.zz[j] = E;",
    ),
    (
        "built-in-member-written-twice",
        "var b = Function.prototype.bind; b.zz = {}; host.keep(b); host.back().zz = Z; b.zz[j] = E;",
    ),
];

#[test]
fn values_a_foreign_value_stands_for_keep_their_flows() {
    assert_eq!(unseen(COVERED_BY_WORLD), Vec::<String>::new());
}

#[test]
fn unknown_keys_on_many_functions_stay_small() {
    let mut source = String::new();
    for index in 0..2000 {
        source.push_str(&format!("function f{index}(a) {{ return a + {index}; }}"));
    }
    source.push_str("function g() { return g[host.key()]; }");
    for index in 0..300 {
        source.push_str(&format!(
            "var r{index} = g(); r{index}.p{index} = r{index}.q; r{index}({index});"
        ));
    }
    let (exhausted, held) = held(&source);
    assert!(!exhausted, "{held}");
    assert!(held < 800_000, "{held}");
}
