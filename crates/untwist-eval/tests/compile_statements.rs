mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::heap::HeapLedger;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units, record_units_charged, render_value, CompileCensus,
    UnitRecord,
};

struct Pin {
    values: &'static [&'static str],
    bail: Option<&'static str>,
    steps: u64,
    heap: (u64, u64),
}

const NO_HEAP: (u64, u64) = (0, 0);
const NESTING: Option<&str> = Some("nesting depth cap exceeded");

fn assert_pinned(label: &str, records: &[UnitRecord], pin: &Pin) {
    assert_eq!(records.len(), 1, "{label}: {records:?}");
    let record = &records[0];
    let values: Vec<&str> = record.values.iter().map(String::as_str).collect();
    assert_eq!(values, pin.values, "{label}: {record:?}");
    assert_eq!(record.bail.as_deref(), pin.bail, "{label}: {record:?}");
    assert_eq!(record.steps_used, pin.steps, "{label}: {record:?}");
    assert_eq!(
        (record.heap_peak, record.heap_final),
        pin.heap,
        "{label}: {record:?}"
    );
}

fn both(label: &str, key: &str, run: impl Fn()) -> (Vec<UnitRecord>, CompileCensus) {
    let mut compiled = Vec::new();
    let census = compile_census(|| compiled = record_units(&run));
    assert!(!compiled.is_empty(), "{label}: no unit");
    frozen("compile_statements", key, &compiled);
    (compiled, census)
}

fn script(label: &str, source: &str, decoding: bool) -> (Vec<UnitRecord>, CompileCensus) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{label}: {:?}",
        parsed.diagnostics
    );
    let program = &parsed.program;
    let run = || match decoding {
        true => eval_script_decoding(program, Budget::default(), |_| ()),
        false => eval_script_in(program, Budget::default(), |_, _| ()),
    };
    let compiled = record_units_charged(run);
    let key = format!("{label}\n{decoding}\n{source}");
    frozen("compile_statements charged", &key, &compiled);
    both(label, &key, run)
}

fn decode_unit(label: &str, source: &str) -> (Vec<UnitRecord>, CompileCensus) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{label}: {:?}",
        parsed.diagnostics
    );
    let program = common::decoder_program(&parsed.program, "arr", &["dec"], None);
    let calls = common::decoder_calls_anywhere(&parsed.program, &["dec"]);
    both(label, &format!("{label}\ndecode\n{source}"), || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
        let _ = evaluator.decode(&program, &calls);
    })
}

fn compiled_at(census: &CompileCensus, source: &str, marker: &str, skip: usize) -> bool {
    let Some(at) = source.find(marker) else {
        panic!("the marker {marker:?} is not in the source");
    };
    let start = (at + skip) as u32;
    census.functions.iter().any(|(from, _)| *from == start)
}

fn frames(count: usize, parens: usize, leaf: usize) -> String {
    format!(
        "function f(n) {{ if (n === 0) return {}0; return {}f(n - 1); }}\nf({});\n",
        "~".repeat(leaf),
        "~".repeat(parens),
        count - 1
    )
}

#[test]
fn and_n2_compiled_bail_and_finish_where_the_walker_pins_them() {
    let (records, census) = script("compiled", &frames(999, 98, 197), false);
    let pin = Pin {
        values: &[],
        bail: NESTING,
        steps: 110_986,
        heap: NO_HEAP,
    };
    assert_pinned("compiled", &records, &pin);
    assert_eq!(census.calls, 999, "every frame compiled");
    let (records, census) = script("at 998 frames", &frames(998, 98, 197), false);
    let pin = Pin {
        values: &["number:bff0000000000000"],
        bail: None,
        steps: 208_779,
        heap: NO_HEAP,
    };
    assert_pinned("at 998 frames, node -1", &records, &pin);
    assert_eq!(census.calls, 998);
    let (records, census) = script("compiled", &frames(999, 98, 196), false);
    let pin = Pin {
        values: &["number:0000000000000000"],
        bail: None,
        steps: 208_986,
        heap: NO_HEAP,
    };
    assert_pinned("compiled, node 0", &records, &pin);
    assert_eq!(census.calls, 999);
}

fn capture_unit(body: &str) -> String {
    format!(
        "function arr() {{ return ['a', 'b']; }}\nfunction dec(i) {{ var keep; {body} return arr()[i] + typeof keep; }}\ndec(0);\n"
    )
}

fn captured(label: &str, body: &str, steps: u64, heap: (u64, u64)) {
    let source = capture_unit(body);
    let (records, census) = decode_unit(label, &source);
    let pin = Pin {
        values: &["string:afunction"],
        bail: None,
        steps,
        heap,
    };
    assert_pinned(label, &records, &pin);
    assert!(
        compiled_at(&census, &source, "function dec", 0),
        "{label}: dec ran in the walker: {census:?}"
    );
    assert!(
        compiled_at(&census, &source, "function arr", 0),
        "{label}: arr ran in the walker"
    );
}

#[test]
fn to_h5_compiled_keep_the_ledger_after_a_closure_made_inside_each_scope_kind() {
    captured(
        "for (var ...) compiled, node afunction",
        "for (var k = 0; k < 1; k++) { keep = function () { return k; }; }",
        40,
        (4_256, 114),
    );
    captured(
        "switch compiled, node afunction",
        "switch (i) { case 0: keep = function () { return i; }; }",
        31,
        (4_208, 114),
    );
    captured(
        "catch compiled, node afunction",
        "try { throw 0; } catch (x) { keep = function () { return x; }; }",
        31,
        (4_256, 114),
    );
    captured(
        "a named function expression compiled, node afunction",
        "keep = (function named() { return function () { return named; }; })();",
        32,
        (4_832, 114),
    );
}

fn wide_function(vars: usize) -> String {
    let names: Vec<String> = (0..vars).map(|n| format!("v{n}")).collect();
    format!(
        "function f() {{ var {}; v{} = 'last'; return v{}; }}\nf();\n",
        names.join(", "),
        vars - 1,
        vars - 1
    )
}

#[test]
fn compiled_a_function_with_70000_vars_reads_its_last() {
    let (records, census) = script("compiled", &wide_function(70_000), false);
    let pin = Pin {
        values: &["string:last"],
        bail: None,
        steps: 10,
        heap: NO_HEAP,
    };
    assert_pinned("compiled, node last", &records, &pin);
    assert_eq!(census.calls, 1, "f compiled");
}

#[test]
fn and_s19_compiled_equal_the_walkers_pins() {
    let (records, census) = script(
        "compiled",
        "var x = 'outer';\nfunction f(a = x) { var x = 1; return a; }\nf();\n",
        false,
    );
    let pin = Pin {
        values: &["string:outer"],
        bail: None,
        steps: 11,
        heap: NO_HEAP,
    };
    assert_pinned("compiled, node outer", &records, &pin);
    assert_eq!(census.calls, 1);
    let (records, census) = script(
        "compiled",
        "function f() { var inner; try { throw 'thrown'; } catch (e) { var e = 'assigned'; inner = e; } return inner + ',' + e; }\nf();\n",
        false,
    );
    let pin = Pin {
        values: &["string:assigned,undefined"],
        bail: None,
        steps: 23,
        heap: NO_HEAP,
    };
    assert_pinned("compiled, node assigned,undefined", &records, &pin);
    assert_eq!(census.calls, 1);
}

fn compiled_in_both_modes(rows: &[(&str, &str)]) {
    for (label, source) in rows {
        for decoding in [false, true] {
            let (_, census) = script(label, source, decoding);
            assert!(census.calls > 0, "{label}: no call entered compiled");
        }
    }
}

#[test]
fn the_ported_statements_run_compiled_as_the_walker_runs_them() {
    compiled_in_both_modes(&[
        ("return, var and if/else", "function f(a) { var b = a + 1, c; if (b > 2) { c = 'big'; } else c = 'small'; return c + b; } f(1) + f(5);"),
        ("for with break and continue", "function f() { var t = 0; for (var i = 0; i < 10; i++) { if (i === 2) continue; if (i > 5) break; t = t + i; } return t + ',' + i; } f();"),
        ("for with an expression init and no test", "function f() { var i, t = ''; for (i = 3; ; i--) { if (!i) break; t = t + i; } return t; } f();"),
        ("while with a closure per iteration", "function f() { var k = 0, g = function () {}; while (k < 3) { g[k] = function () { return k; }; k++; } return g[0]() + g[2](); } f();"),
        ("try/catch of a native throw", "function f() { try { null.x; return 'no'; } catch (e) { return e instanceof TypeError; } } f();"),
        ("a catch without a parameter", "function f() { try { throw 1; } catch { return 'caught'; } } f();"),
        ("a rethrow from catch", "function f() { try { throw 1; } catch (e) { throw e + 1; } } function g() { try { f(); } catch (e) { return e; } } g();"),
        ("hoisted function declarations", "function f() { return g() + h(); function g() { return 1; } function h() { return 2; } } f();"),
        ("switch with fallthrough and default", "function f(x) { var r = ''; switch (x) { case 1: r = r + 'a'; case 2: r = r + 'b'; break; default: r = r + 'd'; case 3: r = r + 'c'; } return r; } f(1) + f(2) + f(3) + f(9);"),
        ("a return from inside nested loops", "function f() { for (var i = 0; i < 3; i++) { while (true) { return i + 7; } } } f();"),
        ("an empty statement and a block", "function f() { ; { var a = 1; { a = a + 1; } } return a; } f();"),
        ("arguments and a var that shadows it", "function f(a) { var arguments; return typeof arguments; } f(1);"),
        ("a var that shadows a parameter", "function f(a) { var a; return a; } f('kept');"),
        ("a function declaration that replaces a parameter", "function f(a) { return typeof a; function a() {} } f(1);"),
        ("a function declaration as an if arm", "function f(a) { if (a) function g() {} return typeof g; } f(1);"),
        ("a loop that reaches the loop cap", "function f() { var i = 0; while (true) { i = i + 1; } } f();"),
    ]);
}

#[test]
fn the_decoders_expression_forms_run_compiled_as_the_walker_runs_them() {
    compiled_in_both_modes(&[
        ("compound assignment, this and new", "function F(n) { this.n = n; this.n *= 2; } function f() { var o = new F(3), s = 'x'; s += o.n; return s; } f();"),
        ("array literals with holes", "function f() { var a = [1, , 3], b = []; b[2] = a.length; return a[1] + ',' + b.length; } f();"),
        ("a spread element", "function f(a) { return [0, ...a]; } f([1]);"),
        ("the high preset's !![] and += on a binding", "function f() { return !![]; } function g(s) { var t = ''; for (var i = 0; i < s.length; i++) { t += s.charAt(i); } return t; } f() + g('ab');"),
        ("a sloppy this while decoding", "function f() { return this; } f();"),
        ("new with a native constructor", "function f() { var r = new RegExp('a+'); return r.test('caa'); } f();"),
    ]);
}

#[test]
fn the_laziness_row_compiles_no_node_of_code_that_never_runs() {
    let source = "var r = 0; function f(c) { if (c) { r = 1 + 2; } else { r = 3 + 4; } while (0) { r = 5 + 6; } for (var i = 0; i < 0; i++) { r = 7 + 8; } try { r = r + 9; } catch (e) { r = 10 + 11; } switch (c) { case 0: r = 12 + 13; } } f(true); r;";
    let (records, census) = script("laziness, statements", source, false);
    assert_eq!(records[0].values, ["number:4028000000000000"]);
    assert_eq!(census.calls, 1);
    for never in ["3 + 4", "5 + 6", "7 + 8", "10 + 11", "12 + 13"] {
        let at = source.find(never).expect("a marker") as u32;
        let end = at + never.len() as u32;
        assert!(
            census
                .node_spans
                .iter()
                .all(|(from, to)| *from < at || *to > end),
            "{never}: a node of code that never ran compiled: {census:?}"
        );
    }
}

#[test]
fn a_function_whose_body_held_a_statement_unported_before_task_8_now_runs_compiled() {
    for (label, source) in [
        ("let", "function f() { let a = 1; return a; } f();"),
        (
            "try/finally",
            "function f() { try { return 1; } finally { } } f();",
        ),
        (
            "a labelled loop",
            "function f() { a: while (1) { break a; } return 2; } f();",
        ),
        (
            "do-while",
            "function f() { var i = 0; do { i++; } while (i < 3); return i; } f();",
        ),
        (
            "for-in",
            "function f() { var k; for (k in { a: 1 }) {} return k; } f();",
        ),
        (
            "a block function",
            "function f() { { function g() {} } return 1; } f();",
        ),
    ] {
        let (_, census) = script(label, source, false);
        assert_eq!(census.calls, 1, "{label}: did not enter compiled");
    }
}

fn high() -> String {
    common::read_fixture("obfuscator.io-high.js")
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset's rotation search is slow in debug"
)]
fn the_high_presets_decoder_and_string_array_function_enter_compiled() {
    let source = high();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "_0x4d6b", &["_0x4c3e"], Some(0));
    let calls = common::decoder_calls_anywhere(&parsed.program, &["_0x4c3e"]);
    let (records, census) = both("the high preset", "the high preset decodes", || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let _ = evaluator.decode(&program, &calls);
    });
    assert!(
        records.iter().all(|unit| unit.bail.is_none()),
        "{records:?}"
    );
    let expected = [
        ("the string-array function", "function _0x4d6b(){", 0),
        ("its replacement", "_0x4d6b=function(){", 8),
        ("the decoder", "function _0x4c3e(", 0),
        ("the decoder's replacement", "return _0x4c3e=function(", 15),
        ("the base64 helper", "var _0x1608fd=function(", 14),
        ("the rc4 helper", "var _0x2bb018=function(", 14),
    ];
    for (what, marker, skip) in expected {
        assert!(
            compiled_at(&census, &source, marker, skip),
            "{what} ran in the walker: {:?}",
            census.functions
        );
    }
    println!(
        "the high preset: {} compiled functions, {} compiled calls",
        census.functions.len(),
        census.calls
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset's rotation search is slow in debug"
)]
fn the_high_presets_rotator_enters_compiled_when_it_is_interpreted_directly() {
    let source = high();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "_0x4d6b", &["_0x4c3e"], Some(0));
    let iife = program.rotator.expect("the high preset's rotator");
    let rotate = || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator.instantiate(&program).expect("instantiates");
        let outcome = session.run_rotator(iife).map_err(|bail| bail.to_string());
        let first = session.array_first().map(|value| render_value(&value));
        (outcome, session.budget().steps_used(), first)
    };
    let mut compiled = None;
    let census = compile_census(|| {
        record_units(|| compiled = Some(rotate()));
    });
    frozen(
        "compile_statements",
        "the high preset's rotator",
        std::slice::from_ref(&compiled),
    );
    for (what, marker, skip) in [
        ("the rotator", "(function(_0x1de6be,_0x296e89){", 1),
        ("a rotator wrapper", "function _0x3896fc(", 0),
    ] {
        assert!(
            compiled_at(&census, &source, marker, skip),
            "{what} ran in the walker: {:?}",
            census.functions
        );
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: the high preset's rotation search is release-only"
)]
fn compiled_the_high_presets_session_keeps_its_ledger_peak_and_final() {
    let source = high();
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "_0x4d6b", &["_0x4c3e"], Some(0));
    let (records, census) = both("compiled", "compiled", || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
        let _ = evaluator.decode(&program, &[]);
    });
    let pin = Pin {
        values: &[],
        bail: None,
        steps: 21_383_392,
        heap: (198_036, 0),
    };
    assert_pinned("compiled", &records, &pin);
    assert_eq!(records[0].memo_inserts, 2_024);
    assert!(
        compiled_at(&census, &source, "function _0x4c3e(", 0),
        "the decoder ran in the walker"
    );
}

fn on_a_2_mib_thread(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the row's thread finished");
}

const PARSE_STACK: usize = 1 << 30;
const STATEMENT_NEST: usize = 40_000;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: a 40,000-deep statement nest, three frames deep, on a 2 MiB thread"
)]
fn a_deep_statement_nest_bails_compiled_where_the_walker_bails_on_a_2_mib_thread() {
    on_a_2_mib_thread(|| {
        let source = format!(
            "function f(n) {{ {}{{ if (n) return f(n - 1); return 0; }} }} f(2);",
            "if (1) ".repeat(STATEMENT_NEST)
        );
        let alloc = Allocator::default();
        let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, &source));
        assert!(parsed.diagnostics.is_empty());
        let program = &parsed.program;
        let (records, census) = both("a deep statement nest", &source, || {
            eval_script_in(program, Budget::default(), |_, _| ());
        });
        assert_eq!(records[0].bail.as_deref(), NESTING, "{records:?}");
        assert_eq!(records[0].steps_used, 200_008, "{records:?}");
        assert_eq!(census.calls, 3, "three frames, the third bails");
    });
}
