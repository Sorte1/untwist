use std::path::PathBuf;

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{
    compile_census, frozen, nesting_at_first_bail, record_units_charged, CompileCensus, UnitRecord,
};
use untwist_testing::node;

struct Row {
    name: &'static str,
    definitions: &'static str,
    call: &'static str,
    node: &'static str,
}

const fn row(
    name: &'static str,
    definitions: &'static str,
    call: &'static str,
    node: &'static str,
) -> Row {
    Row {
        name,
        definitions,
        call,
        node,
    }
}

const ROWS: &[Row] = &[
    row(
        "delete_members",
        "function f() { var o = {a: 1, b: 2, 3: 'c'}; var d = delete o.a; var e = delete o['zz']; var g = delete o[3]; return [d, e, g, Object.keys(o)].join('|'); }",
        "f()",
        "true|true|true|b",
    ),
    row(
        "in_instanceof",
        "function f() { var o = {k: 1}, F = function () {}; var x = new F(); return ['k' in o, 'z' in o, 0 in [5], o instanceof Object, x instanceof F, o instanceof F].join(); }",
        "f()",
        "true,false,true,true,true,false",
    ),
    row(
        "typeof_void",
        "function f(a) { var o = {t: typeof a, u: typeof nope, v: void a, w: typeof {}}; return [o.t, o.u, o.v, o.w].join(); }",
        "f(1)",
        "number,undefined,,object",
    ),
    row(
        "logical_assignment",
        "function f() { var a = 0, b = 1, c = null, d = 'x', o = {p: 0, q: 2, s: 's'}, k = 'q'; a ||= 5; b &&= 6; c ??= 7; d ??= 8; o.p ||= 8; o[k] &&= 9; o.r ??= 10; o.s ||= 11; return [a, b, c, d, o.p, o.q, o.r, o.s].join(); }",
        "f()",
        "5,6,7,x,8,9,10,s",
    ),
    row(
        "compound_binding",
        "function f() { var x = 7; x += 1; x -= 2; x *= 3; x /= 2; x %= 5; x **= 2; x <<= 3; x >>= 1; x >>>= 1; x &= 255; x |= 256; x ^= 3; return x + {t: 1}.t; }",
        "f()",
        "292",
    ),
    row(
        "compound_members",
        "function f() { var o = {m: 7, n: 'a'}, k = 'm'; o.m += 1; o[k] -= 2; o.m *= 3; o[k] /= 2; o.m %= 5; o[k] **= 2; o.m <<= 3; o[k] >>= 1; o.m >>>= 1; o[k] &= 255; o.m |= 256; o[k] ^= 3; o.n += 'b'; return [o.m, o.n].join(); }",
        "f()",
        "291,ab",
    ),
    row(
        "updates",
        "function f() { var o = {a: 1}, k = 'a', x = 5; var r = [x++, ++x, x--, --x, o.a++, ++o[k], o[k]--, --o.a]; return r.concat([x, o.a]).join(); }",
        "f()",
        "5,7,7,5,1,3,3,1,5,1",
    ),
    row(
        "new_forms",
        "function F(a) { this.a = a; } F.prototype.get = function () { return this.a; }; function f() { var x = new F({v: 3}); var y = new F; return [x.get().v, x instanceof F, Object.getPrototypeOf(x) === F.prototype, y.a].join(); }",
        "f()",
        "3,true,true,",
    ),
    row(
        "template_literals",
        "function f(a) { var o = {}; return `x${a}y${a + 1}z${o}${`n${a}`}`; }",
        "f(1)",
        "x1y2z[object Object]n1",
    ),
    row(
        "template_escapes",
        "function f() { return `a\\tb\\u0041${'c'}\\n`.length + `${{}}`; }",
        "f()",
        "6[object Object]",
    ),
    row(
        "object_literals",
        "function f() { var k = 'c'; var o = {a: 1, 'b': 2, [k]: 3, 4: 'four', m() { return this.a; }, n: function () { return this.b; }, a: 5}; return [o.a, o.b, o.c, o[4], o.m(), o.n(), Object.keys(o)].join('|'); }",
        "f()",
        "5|2|3|four|5|2|4,a,b,c,m,n",
    ),
    row(
        "object_shorthand",
        "function f(x, y) { var o = {x, y, z: {x}}; return [o.x, o.y, o.z.x, Object.keys(o.z)].join(); }",
        "f(1, 2)",
        "1,2,1,x",
    ),
    row(
        "regexp_literals",
        "function f() { var r = /a(b)/; var s = /x/i; return [r.test('ab'), r.exec('xab')[1], s.test('X'), /x/ === /x/, 'ab'.replace(/b/g, 'c')].join(); }",
        "f()",
        "true,b,true,false,ac",
    ),
    row(
        "optional_chains",
        "function f() { var o = {a: {b: function () { return this.c; }, c: 5}}, n = null, u; return [o?.a?.b(), n?.x.y, o.z?.(), o?.['a'].c, n?.(), u?.[0], o.a?.b?.()].join(); }",
        "f()",
        "5,,,5,,,5",
    ),
    row(
        "c1_method_this",
        "function f() { var o = {v: 4, m: function () { return this.v; }, n() { return this.v + 1; }}; return [o.m(), o.n(), o['m']()].join(); }",
        "f()",
        "4,5,4",
    ),
    row(
        "c2_computed_calls",
        "function f() { var o = {g: function (x) { return x * 2; }}, k = 'g', t = [o]; return [o[k](3), t[0]['g'](4), (0, o.g)(5), o['g'].call(null, 6)].join(); }",
        "f()",
        "6,8,10,12",
    ),
    row(
        "c3_new_with_a_prototype_read",
        "function F() { this.a = {v: 1}; } function f() { F.prototype.b = 2; var x = new F(); return [x.a.v, x.b, Object.getPrototypeOf(x) === F.prototype, typeof F.prototype].join(); }",
        "f()",
        "1,2,true,object",
    ),
    row(
        "c4_recursion_999_deep",
        "function f(n, o) { return n === 0 ? o.d : f(n - 1, {d: o.d + 1}); }",
        "f(998, {d: 0})",
        "998",
    ),
    row(
        "c5_a_builtin_calling_back",
        "function f() { return [3, 1, 2].map(function (x) { return {v: x}; }).reduce(function (s, o) { return s + `${o.v}`; }, '') + 'xy'.replace('x', function (m) { return `<${m}>`; }); }",
        "f()",
        "312<x>y",
    ),
    row(
        "c6_to_string_equal",
        "function f() { return {a: `${1}`}; }",
        "[f().a, f.toString()].join('|')",
        "1|function f() { return {a: `${1}`}; }",
    ),
    row(
        "a_chain_holds_its_computed_key_across_the_read",
        "function f(b) { var s = 'tnullu'; return [s?.[b + ''.toString()], s?.[b + 1], s?.['x' + b]].join(); }",
        "f(2)",
        "u,l,",
    ),
    row(
        "literal_arrays",
        "function f() { var a = ['s', 1, true, null, -0x1, 0x2 * 0x3, ~-1 >>> 0]; var b = []; return [a.length, a.join(), b.length, a[4] === -1].join('|'); }",
        "f()",
        "7|s,1,true,,-1,6,0|0|true",
    ),
];

const EDGES: &[(&str, &str)] = &[
    (
        "spread_element",
        "function f() { var a = [1]; return [0, ...a]; } f();",
    ),
    (
        "object_spread",
        "function f() { var a = {}; return {b: 1, ...a}; } f();",
    ),
    (
        "class_expression",
        "function f() { var o = {}; return class {}; } f();",
    ),
    (
        "destructuring_target",
        "function f() { var a, o = {}; [a] = [1]; return a; } f();",
    ),
    (
        "delete_binding",
        "function f() { var x = {}; return delete x; } f();",
    ),
    (
        "accessor_entry",
        "function f() { var o = { get a() { return 1; } }; return typeof o.a; } f();",
    ),
    (
        "proto_entry",
        "function f() { var o = { __proto__: null, k: 1 }; return o.k; } f();",
    ),
    (
        "sloppy_this",
        "function f() { var o = {t: this}; return typeof o.t; } f();",
    ),
    (
        "template_invalid_escape",
        "function f() { return { t: (function (s) { return s; })`\\x` }; } f();",
    ),
    ("bigint", "function f() { var o = {}; return 1n; } f();"),
];

fn parse_and<R>(source: &str, body: impl FnOnce(&Program<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(1 << 30, || parse_program(&alloc, source));
    assert!(
        parsed.diagnostics.is_empty(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    body(&parsed.program)
}

type Outcome = (Vec<UnitRecord>, Option<u32>);

#[derive(Clone, Copy)]
struct Run {
    fuel: Option<u64>,
    decoding: bool,
}

fn run_in(program: &Program<'_>, at: Run) -> Outcome {
    let budget = at.fuel.map_or_else(Budget::default, Budget::new);
    let mut units = Vec::new();
    let nesting = nesting_at_first_bail(|| {
        units = record_units_charged(|| match at.decoding {
            true => eval_script_decoding(program, budget, |_| ()),
            false => eval_script_in(program, budget, |_, _| ()),
        });
    });
    (units, nesting)
}

fn run(source: &str, fuel: Option<u64>, decoding: bool) -> Outcome {
    parse_and(source, |program| run_in(program, Run { fuel, decoding }))
}

fn compared_in(label: &str, program: &Program<'_>, at: Run) -> (Outcome, CompileCensus) {
    let mut compiled = (Vec::new(), None);
    let census = compile_census(|| compiled = run_in(program, at));
    assert!(
        !compiled.0.is_empty(),
        "{label} (fuel {:?}, decoding {}): no unit",
        at.fuel,
        at.decoding
    );
    (compiled, census)
}

fn compared(label: &str, source: &str, fuel: Option<u64>, decoding: bool) -> CompileCensus {
    parse_and(source, |program| {
        let (outcome, census) = compared_in(label, program, Run { fuel, decoding });
        frozen(
            "compile_forms",
            &format!("{label}\n{fuel:?}\n{decoding}\n{source}"),
            std::slice::from_ref(&outcome),
        );
        census
    })
}

fn steps_in(program: &Program<'_>, decoding: bool) -> u64 {
    let at = Run {
        fuel: None,
        decoding,
    };
    run_in(program, at)
        .0
        .iter()
        .map(|unit| unit.steps_used)
        .sum()
}

const SWEPT_STEPS: u64 = 4_000;
const SAMPLED_CAPS: u64 = 400;

fn caps(total: u64) -> Vec<u64> {
    match total <= SWEPT_STEPS {
        true => (0..=total + 1).collect(),
        false => (0..SAMPLED_CAPS)
            .map(|index| index * (total + 2) / SAMPLED_CAPS)
            .collect(),
    }
}

fn swept_with(
    label: &str,
    source: &str,
    decoding: bool,
    caps: fn(u64) -> Vec<u64>,
) -> CompileCensus {
    parse_and(source, |program| {
        let mut total = CompileCensus::default();
        let mut outcomes = Vec::new();
        for cap in caps(steps_in(program, decoding)) {
            let at = Run {
                fuel: Some(cap),
                decoding,
            };
            let (outcome, census) = compared_in(label, program, at);
            outcomes.push(outcome);
            total.template_batches += census.template_batches;
            total.template_fallbacks += census.template_fallbacks;
        }
        frozen(
            "compile_forms swept",
            &format!("{label}\n{decoding}\n{source}"),
            std::slice::from_ref(&outcomes),
        );
        total
    })
}

fn swept(label: &str, source: &str, decoding: bool) -> CompileCensus {
    swept_with(label, source, decoding, caps)
}

fn script(row: &Row) -> String {
    format!("{}\nString({});\n", row.definitions, row.call)
}

fn printed_by_node(row: &Row) -> Option<String> {
    let Some(binary) = node::node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            node::skip_allowed(required.as_deref()),
            "required: {}",
            node::SKIP_MESSAGE
        );
        return None;
    };
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("compile_forms");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join(format!("{}.js", row.name));
    let program = format!("{}\nconsole.log(String({}));\n", row.definitions, row.call);
    std::fs::write(&path, program).expect("write the node program");
    let observed = node::run(&binary, &path)
        .expect("run node")
        .expect("node finished in time");
    assert_eq!(observed.exit, Some(0), "{}: {}", row.name, observed.stderr);
    Some(observed.stdout.trim_end().to_string())
}

#[test]
fn every_form_runs_compiled_and_prints_what_node_prints() {
    for row in ROWS {
        if let Some(printed) = printed_by_node(row) {
            assert_eq!(
                printed, row.node,
                "{}: node prints something else",
                row.name
            );
        }
        let source = script(row);
        let census = compared(row.name, &source, None, false);
        assert!(census.calls > 0, "{}: no call entered compiled", row.name);
        let (units, _) = run(&source, None, false);
        assert_eq!(
            units[0].values,
            [format!("string:{}", row.node)],
            "{}: {:?}",
            row.name,
            units[0]
        );
    }
}

#[test]
fn every_form_matches_the_walker_while_decoding() {
    for row in ROWS {
        compared(row.name, &script(row), None, true);
    }
}

#[test]
fn every_form_matches_the_walker_at_every_fuel_cap() {
    for row in ROWS {
        swept(row.name, &script(row), false);
        swept(row.name, &script(row), true);
    }
}

#[test]
fn every_edge_bails_or_diverges_from_node_exactly_as_the_walker_does() {
    for (name, source) in EDGES {
        for decoding in [false, true] {
            let census = compared(name, source, None, decoding);
            assert!(census.calls > 0, "{name}: no call entered compiled");
        }
        swept(name, source, false);
    }
}

fn literal_array(count: usize, constants: bool) -> String {
    let items: Vec<String> = (0..count)
        .map(|index| match (constants, index % 4) {
            (true, 0) => format!("-0x{index:x}"),
            (true, 1) => format!("0x{index:x} * 0x2 + -0x1"),
            (_, 2) => "true".to_string(),
            (_, 3) => "null".to_string(),
            (_, 1) => format!("'s{index}'"),
            _ => index.to_string(),
        })
        .collect();
    format!("var g = () => [{}]; g().length;", items.join(", "))
}

#[test]
fn a_fuel_sweep_across_a_1000_element_literal_array() {
    for constants in [false, true] {
        let source = literal_array(1_000, constants);
        let full = compared("literal array sweep", &source, None, false);
        assert_eq!(full.templates, 1, "constants {constants}: {full:?}");
        assert_eq!(full.template_batches, 1, "constants {constants}");
        let every = swept_with("literal array sweep", &source, false, |total| {
            (0..=total + 1).collect()
        });
        assert!(
            every.template_fallbacks >= 1_000,
            "constants {constants}: {every:?}"
        );
    }
}

fn nested(levels: usize, array: &str) -> String {
    format!(
        "(() => {}{array}{})();",
        "(0, ".repeat(levels),
        ")".repeat(levels)
    )
}

fn deep_outcome(source: &str) -> (Outcome, CompileCensus) {
    untwist_eval::stack::grow(1 << 30, || {
        let mut outcome = (Vec::new(), None);
        let census = compile_census(|| outcome = run(source, None, false));
        (outcome, census)
    })
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 100,000 nesting levels per run"
)]
fn the_nesting_cap_at_the_arrays_own_level_and_one_below() {
    let cap = untwist_eval::budget::NESTING_DEPTH_CAP as usize;
    for array in ["[1, 'a', null]", "[1, -(0x2 * 0x3), 'b']"] {
        let mut seen = (0, 0);
        for levels in cap - 8..cap + 2 {
            let source = nested(levels, array);
            let (compiled, census) = deep_outcome(&source);
            frozen(
                "compile_forms nesting cap",
                &source,
                std::slice::from_ref(&compiled),
            );
            seen.0 += census.template_batches;
            seen.1 += census.template_fallbacks;
        }
        assert!(seen.0 > 0 && seen.1 > 0, "{array}: {seen:?}");
    }
}

#[test]
fn a_hole_while_decoding_refuses_at_the_walkers_step_and_is_no_template() {
    let source = "function f() { return [1, , 2]; } f();";
    let census = compared("hole while decoding", source, None, true);
    assert_eq!(census.templates, 0, "{census:?}");
    assert!(census.calls > 0);
    let (units, _) = run(source, None, true);
    assert!(
        units[0]
            .bail
            .as_deref()
            .is_some_and(|bail| bail.contains("hole")),
        "{:?}",
        units[0]
    );
    swept("hole while decoding", source, true);
}

#[test]
fn a_template_in_a_loop_keeps_the_walkers_ledger_on_every_execution() {
    let source = "function f() { var keep = [], i; for (i = 0; i < 50; i++) { keep.push(['a', 'b', -0x1, 0x2 * 0x3, null]); } return keep.length; } f();";
    let census = compared("template in a loop", source, None, false);
    assert_eq!(census.templates, 2, "{census:?}");
    assert_eq!(census.template_batches, 51, "{census:?}");
    let (units, _) = run(source, None, false);
    assert!(units[0].heap_peak > 0, "{:?}", units[0]);
    swept("template in a loop", source, false);
}

const HELD_ACROSS_THE_WRITE: &[(&str, &str, u64)] = &[
    (
        "a binding re-bound inside its own &&=",
        "function f() { var x = [1, 2, 3, 4, 5, 6, 7, 8]; x &&= (x = 0, [9, 9, 9, 9, 9, 9, 9, 9]); return x.length; } f();",
        3_152,
    ),
    (
        "a member re-bound inside its own &&=",
        "function f() { var o = {p: [1, 2, 3, 4, 5, 6, 7, 8]}; o.p ||= 0; o.p &&= (o.p = 0, [9, 9, 9, 9, 9, 9, 9, 9, 9]); return o.p.length; } f();",
        3_632,
    ),
];

#[test]
fn a_logical_assignment_holds_its_old_value_until_after_the_write() {
    for (label, source, peak) in HELD_ACROSS_THE_WRITE {
        for decoding in [false, true] {
            let census = compared(label, source, None, decoding);
            assert!(census.calls > 0, "{label}: no call entered compiled");
        }
        let (units, _) = run(source, None, false);
        assert_eq!(units[0].heap_peak, *peak, "{label}: {:?}", units[0]);
    }
}
