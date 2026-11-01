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
        "s10_closures_per_for_let_iteration",
        "function f() { var fs = []; for (let i = 0, g = () => i; i < 3; i++) { fs.push(() => i); if (i === 1) fs.push(g); } return fs.map(function (h) { return h(); }).join(); }",
        "f()",
        "0,1,0,2",
    ),
    row(
        "s10_a_let_head_updated_in_its_body",
        "function f() { var fs = []; for (let i = 0; i < 6; i++) { let j = i * 2; fs.push(() => i + ':' + j); i += 1; } return fs.map(function (h) { return h(); }).join(); }",
        "f()",
        "1:0,3:4,5:8",
    ),
    row(
        "s11_for_in_and_for_of_with_closures",
        "function f() { var fs = []; for (let k in {a: 1, b: 2}) fs.push(() => k); for (const v of [3, 4]) { fs.push(() => v); } for (var w of [5, 6]) fs.push(() => w); var o = {}; for (o.p in {x: 1, y: 2}) {} var t; for (t of ['s', 't']) {} return fs.map(function (h) { return h(); }).concat([o.p, t]).join(); }",
        "f()",
        "a,b,3,4,6,6,y,t",
    ),
    row(
        "s12_switch_with_a_let_read_across_cases",
        "function f(x) { var g, r; switch (x) { case 0: let a = 'zero'; const b = 'b'; g = () => a + b; case 1: a = a + '!'; r = [a, g(), typeof b].join(); break; default: return 'd'; } return r; }",
        "[f(0), f(7)].join('|')",
        "zero!,zero!b,string|d",
    ),
    row(
        "s13_try_return_finally",
        "function f() { var log = ''; function g() { try { log += 't'; return 'r'; } finally { log += 'f'; } } function h() { try { return 'x'; } finally { return 'y'; } } function k() { try { throw 1; } catch (e) { return 'c' + e; } finally { log += 'k'; } } function m() { for (var i = 0; i < 3; i++) { try { if (i === 1) break; } finally { log += i; } } return i; } function n() { try { throw 'lost'; } finally { return 'kept'; } } return [g(), log, h(), k(), m(), n(), log].join(); }",
        "f()",
        "r,tf,y,c1,1,kept,tfk01",
    ),
    row(
        "s14_a_labelled_continue",
        "function f() { var r = []; outer: for (var i = 0; i < 3; i++) { for (var j = 0; j < 3; j++) { if (j === 1) continue outer; r.push(i + '' + j); } } inner: while (true) { do { break inner; } while (true); } block: { r.push('b'); if (r) break block; r.push('never'); } return r.join(); }",
        "f()",
        "00,10,20,b",
    ),
    row(
        "do_while_with_continue",
        "function f() { var i = 0, s = ''; do { i++; if (i === 2) continue; s += i; } while (i < 4); return s; }",
        "f()",
        "134",
    ),
    row(
        "blocks_with_lexicals_and_closures",
        "function f() { 'use strict'; var fs = []; { let a = 1; const b = 2; fs.push(() => a + b); { let a = 10; fs.push(() => a); } a = 5; } let c = 'c'; { function g() { return c; } fs.push(g); } return fs.map(function (h) { return h(); }).join(); }",
        "f()",
        "7,10,c",
    ),
    row(
        "a_catch_body_with_lexicals",
        "function f() { try { null.x; } catch (e) { let m = typeof e; const n = () => m; return [m, n()].join(); } }",
        "f()",
        "object,object",
    ),
    row(
        "nested_try_with_completions_overriding",
        "function f() { var log = []; for (var i = 0; i < 2; i++) { try { try { throw i; } finally { log.push('in' + i); } } catch (e) { log.push('c' + e); continue; } finally { log.push('out' + i); } } return log.join(); }",
        "f()",
        "in0,c0,out0,in1,c1,out1",
    ),
    row(
        "throw_through_loops_and_labels",
        "function f() { try { a: for (var k in {p: 1}) { while (true) { throw k + '!'; } } } catch (e) { return e; } }",
        "f()",
        "p!",
    ),
    row(
        "a_top_level_let_and_const_read_by_a_function",
        "let top = 't'; const two = 2; function f() { return top + two; }",
        "f()",
        "t2",
    ),
    row(
        "a_debugger_statement",
        "function f() { debugger; return 'd'; }",
        "f()",
        "d",
    ),
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

fn compared(label: &str, source: &str, decoding: bool) -> (Outcome, CompileCensus) {
    parse_and(source, |program| {
        let at = Run {
            fuel: None,
            decoding,
        };
        let compared = compared_in(label, program, at);
        frozen(
            "compile_flow",
            &format!("{label}\n{decoding}\n{source}"),
            std::slice::from_ref(&compared.0),
        );
        compared
    })
}

const SWEPT_STEPS: u64 = 3_000;
const SAMPLED_CAPS: u64 = 300;

fn swept(label: &str, source: &str, decoding: bool) {
    parse_and(source, |program| {
        let full = Run {
            fuel: None,
            decoding,
        };
        let total: u64 = run_in(program, full)
            .0
            .iter()
            .map(|unit| unit.steps_used)
            .sum();
        let caps: Vec<u64> = match total <= SWEPT_STEPS {
            true => (0..=total + 1).collect(),
            false => (0..SAMPLED_CAPS)
                .map(|index| index * (total + 2) / SAMPLED_CAPS)
                .collect(),
        };
        let mut outcomes = Vec::with_capacity(caps.len());
        for cap in caps {
            let at = Run {
                fuel: Some(cap),
                decoding,
            };
            outcomes.push(compared_in(label, program, at).0);
        }
        frozen(
            "compile_flow swept",
            &format!("{label}\n{decoding}\n{source}"),
            std::slice::from_ref(&outcomes),
        );
    });
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
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("compile_flow");
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
fn to_s14_and_every_statement_form_run_compiled_and_print_what_node_prints() {
    for row in ROWS {
        if let Some(printed) = printed_by_node(row) {
            assert_eq!(
                printed, row.node,
                "{}: node prints something else",
                row.name
            );
        }
        let ((units, _), census) = compared(row.name, &script(row), false);
        assert!(census.calls > 0, "{}: no call entered compiled", row.name);
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
fn every_statement_form_matches_the_walker_while_decoding() {
    for row in ROWS {
        compared(row.name, &script(row), true);
    }
}

#[test]
fn every_statement_form_matches_the_walker_at_every_fuel_cap() {
    for row in ROWS {
        swept(row.name, &script(row), false);
        swept(row.name, &script(row), true);
    }
}

const WALKER_ONLY_SHAPES: &[(&str, &str)] = &[
    (
        "a var for-in binding declared in the loop's own scope",
        "function f() { var k = 'kept'; for (var k in {a: 1}) {} return k; } f();",
    ),
    (
        "a nested label continuing the outer loop",
        "function f() { var n = 0; a: b: while (n < 3) { n++; continue a; } return n; } f();",
    ),
    (
        "an early let read outside decoding",
        "var x = 'outer'; function f() { var r = x; let x = 'inner'; return r + x; } f();",
    ),
    (
        "an early let read through a hoisted function",
        "var x = 'outer'; function f() { function g() { return x; } var r = g(); let x = 'inner'; return r + g(); } f();",
    ),
    (
        "a let read across switch cases it skipped",
        "var a = 'outer'; function f(x) { switch (x) { case 0: let a = 'zero'; case 1: return typeof a + a; } } f(1);",
    ),
    (
        "a const reassigned",
        "function f() { const c = 1; c = 2; return c; } f();",
    ),
    (
        "a for-let head read in its own initialiser",
        "var i = 'outer'; function f() { for (let i = i + '!'; ; ) { return i; } } f();",
    ),
    (
        "a class declaration",
        "function f() { class C {} return 1; } f();",
    ),
    (
        "a refusal inside try/finally skips the finally",
        "var g = 0; function f() { try { with ({}) { g = 2; } } finally { g = 1; g = g + 1; } return g; } f();",
    ),
    (
        "a labelled block at program level",
        "a: { break a; } 'after';",
    ),
    (
        "a function body's const read in its own declaration",
        "var x = 'outer'; function f() { const x = typeof x + x; return x; } f();",
    ),
    (
        "a block's let read in its own declaration",
        "var x = 'outer'; function f() { { let x = x + '!'; return x; } } f();",
    ),
    (
        "a parameter default read in its own and a later parameter's default",
        "var b = 'outer', c = 'c'; function f(a = b, b = a + b, c = b + c) { return [a, b, c].join(); } f();",
    ),
    (
        "a duplicate parameter read in a default before its last binding",
        "var a = 'oa'; function f(a, b = a, a) { return b; } f(1, undefined, 3);",
    ),
    (
        "a catch body's let read in its own declaration",
        "var x = 'outer'; function f() { try { throw 1; } catch (e) { let x = x + e; return x; } } f();",
    ),
];

#[test]
fn the_walkers_own_semantics_hold_compiled_where_node_differs() {
    for (label, source) in WALKER_ONLY_SHAPES {
        for decoding in [false, true] {
            compared(label, source, decoding);
        }
        swept(label, source, false);
    }
}
