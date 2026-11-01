mod common;

use oxc_allocator::Allocator;
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_program;
use untwist_eval::budget::NESTING_DEPTH_CAP;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::decode::InterpEvaluator;
use untwist_eval::heap::HeapLedger;
use untwist_eval::rotation::RotationMemo;
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units, CompileCensus, UnitRecord, UNCHECKED_LEVELS,
};

const ARRAY: &str = "function a() { var h = ['4', 'x', '1', '2', '3', '5a', 'q']; a = function () { return h; }; return a(); }";
const DECODER: &str =
    "function b(i, k) { i = i - 0x10; var e = a(); return e[i] + (k === undefined ? '' : '' + k); }";
const TWO_TERMS: &str = "parseInt(b(0x10)) + parseInt(b(0x12))";
const FOUR_TERMS: &str =
    "parseInt(b(0x10)) / 0x1 + -parseInt(b(0x11)) * (parseInt(b(0x12)) / 0x2) + parseInt(b(0x13))";
const CALLS: &str = "b(0x10);\nb(0x12, 'k');";
const PARSE_STACK: usize = 1 << 30;

fn rotator(params: &str, prelude: &str, terms: &str, args: &str) -> String {
    format!(
        "(function ({params}) {{ {prelude} while (!![]) {{ try {{ var f = {terms}; if (f === d) break; else e['push'](e['shift']()); }} catch (g) {{ e['push'](e['shift']()); }} }} }})({args});"
    )
}

fn program(rotation: &str, calls: &str) -> String {
    format!("{ARRAY}\n{DECODER}\n{rotation}\n{calls}")
}

#[derive(Clone, Copy)]
struct Run {
    fuel: Option<u64>,
    heap: Option<u64>,
    memo_twice: bool,
}

const FULL: Run = Run {
    fuel: None,
    heap: None,
    memo_twice: false,
};

fn decode_rounds(source: &str, run: Run) {
    let alloc = Allocator::default();
    let parsed = untwist_eval::stack::grow(PARSE_STACK, || parse_program(&alloc, source));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = common::decoder_program(&parsed.program, "a", &["b"], Some(0));
    let calls = common::calls_for(&parsed.program, 0);
    let mut memo = RotationMemo::default();
    for _ in 0..if run.memo_twice { 2 } else { 1 } {
        let budget = run.fuel.map_or_else(Budget::default, Budget::new);
        let mut evaluator = InterpEvaluator::new(budget);
        evaluator.heap = run.heap.map(HeapLedger::new);
        let _ =
            evaluator.decode_partial_with_memo(&program, &calls, &mut memo, &FxHashSet::default());
    }
}

fn same(label: &str, source: &str, run: Run) -> (Vec<UnitRecord>, CompileCensus) {
    let mut compiled = Vec::new();
    let census = compile_census(|| {
        compiled = record_units(|| decode_rounds(source, run));
    });
    assert_eq!(
        census.depth_overruns, 0,
        "{label}: an unchecked unit overran"
    );
    (compiled, census)
}

const SESSION_TERM_DEPTH: usize = 6_000;

thread_local! {
    static DIRECT_TERMS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static FOLDED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static FALLBACKS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static SCOPES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

type Counter = std::thread::LocalKey<std::cell::Cell<u64>>;

fn tally(counter: &'static Counter, by: u64) {
    counter.with(|count| count.set(count.get() + by));
}

fn tallied(counter: &'static Counter) -> u64 {
    counter.with(std::cell::Cell::get)
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<'s>(&mut self, items: &[&'s str]) -> &'s str {
        items[(self.next() % items.len() as u64) as usize]
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn constant_caps(census: &CompileCensus, rng: &mut Rng) -> Vec<u64> {
    let runs = &census.constant_runs;
    if runs.is_empty() {
        return Vec::new();
    }
    (0..2)
        .map(|_| {
            let run = runs[rng.below(runs.len() as u64) as usize];
            run.steps + rng.below(run.weight + 1)
        })
        .collect()
}

fn sweep(label: &str, source: &str, rng: &mut Rng, caps: u64) -> u64 {
    let mut swept = Vec::new();
    let runs = sweep_into(label, source, (rng, caps), &mut swept);
    frozen(
        "compile_entry_sessions swept",
        &format!("{label}\n{source}"),
        std::slice::from_ref(&swept),
    );
    runs
}

fn sweep_into(
    label: &str,
    source: &str,
    (rng, caps): (&mut Rng, u64),
    swept: &mut Vec<Vec<UnitRecord>>,
) -> u64 {
    let (full, census) = same(label, source, FULL);
    swept.push(full);
    tally(&DIRECT_TERMS, census.direct_terms);
    tally(&FOLDED, census.constants);
    tally(&SCOPES, census.checksum_scopes);
    let mut runs = 0;
    for memo_twice in [false, true] {
        let ledger = Run {
            heap: Some(heap_budget(source.len())),
            memo_twice,
            ..FULL
        };
        let (records, _) = same(label, source, ledger);
        swept.push(records.clone());
        let steps = records
            .iter()
            .map(|unit| unit.steps_used)
            .max()
            .unwrap_or(0);
        let peak = records.iter().map(|unit| unit.heap_peak).max().unwrap_or(0);
        for _ in 0..caps {
            let fuel = Some(rng.next() % (steps + 2));
            swept.push(same(label, source, Run { fuel, ..ledger }).0);
            let heap = Some(rng.next() % (peak + 1));
            swept.push(same(label, source, Run { heap, ..ledger }).0);
            runs += 2;
        }
        for fuel in constant_caps(&census, rng) {
            let (records, capped) = same(
                label,
                source,
                Run {
                    fuel: Some(fuel),
                    ..ledger
                },
            );
            swept.push(records);
            tally(&FALLBACKS, capped.constant_fallbacks);
            runs += 1;
        }
        runs += 1;
    }
    runs
}

fn rotations() -> Vec<(&'static str, String)> {
    vec![
        ("plain", rotator("c, d", "var e = c();", TWO_TERMS, "a, 5")),
        (
            "four terms",
            rotator("c, d", "var e = c();", FOUR_TERMS, "a, 0x2"),
        ),
        (
            "duplicate parameters",
            rotator("c, c", "var e = c();", TWO_TERMS, "a, 5").replace("f === d", "f === c"),
        ),
        (
            "a prelude function named as the array parameter",
            rotator(
                "c, d",
                "function c() { return a(); } var e = c();",
                TWO_TERMS,
                "0, 5",
            ),
        ),
        (
            "a prelude var redeclares the array parameter",
            rotator("c, d", "var c = c, e = c();", TWO_TERMS, "a, 5"),
        ),
        (
            "the array parameter named as the global array",
            rotator("a, d", "var e = a();", TWO_TERMS, "a, 5"),
        ),
        (
            "an array-argument closure reads the expected value later",
            rotator(
                "c, d",
                "var e = c();",
                TWO_TERMS,
                "function () { return d === 5 ? a() : a(); }, 5",
            ),
        ),
        (
            "an array argument reads its own parameter now",
            rotator(
                "c, d",
                "var e = c();",
                TWO_TERMS,
                "(function () { return c; })(), 5",
            ),
        ),
        (
            "the expected value reads the array parameter",
            rotator("c, d", "var e = c();", TWO_TERMS, "a, c === a ? 5 : 6"),
        ),
        (
            "the expected value builds a charged array",
            rotator("c, d", "var e = c();", TWO_TERMS, "a, [1, 2, 3].length + 2"),
        ),
        (
            "the expected value throws",
            rotator("c, d", "var e = c();", TWO_TERMS, "a, (void 0).x"),
        ),
        (
            "the array argument throws",
            rotator("c, d", "var e = c();", TWO_TERMS, "(void 0).x, 5"),
        ),
        (
            "the prelude throws",
            rotator(
                "c, d",
                "var q = (void 0).x; var e = c();",
                TWO_TERMS,
                "a, 5",
            ),
        ),
        (
            "a prelude closure captures",
            rotator(
                "c, d",
                "var q = function () { return d; }; var r = [q, q]; var e = c();",
                TWO_TERMS,
                "a, 5",
            ),
        ),
        (
            "a prelude wrapper declared after use",
            rotator(
                "c, d",
                "var e = c(); function w(x, y) { return b(y - -0x10, x); }",
                "parseInt(w(0, 0)) + parseInt(w(0, 2))",
                "a, 5",
            ),
        ),
        (
            "never matches",
            rotator("c, d", "var e = c();", TWO_TERMS, "a, 999"),
        ),
        (
            "a strict rotator",
            rotator("c, d", "'use strict'; var e = c();", TWO_TERMS, "a, 5"),
        ),
        (
            "no checksum, the rotator runs",
            rotator("c, d", "var e = c(); c = 0;", TWO_TERMS, "a, 5"),
        ),
    ]
}

fn argument_calls() -> String {
    [
        "0x10, 'a' + 'b'",
        "0x10, 'x' in 'y'",
        "0x10, 1 instanceof 1",
        "0x10, '\\uD800'",
        "0x10, 2 ** 0.5",
        "-0 + 0x10",
        "0x10, (1, 2)",
        "0x10, [1][0]",
        "0x10, -(-(-(1)))",
        "(0x10), ((('k')))",
        "0x10, null + 1",
        "0x10, ~'5'",
        "0x10, b",
        "0x10, typeof b",
        "0x10, a()[0]",
        "0x10 + 0x1 - 0x1",
        "0x10, ({}).x",
    ]
    .iter()
    .map(|argument| format!("b({argument});\n"))
    .collect()
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: each session runs at a sweep of fuel and heap caps"
)]
fn hand_written_sessions_equal_the_walker_under_fuel_and_heap_caps() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut runs = 0;
    for (label, rotation) in rotations() {
        runs += sweep(label, &program(&rotation, CALLS), &mut rng, 40);
    }
    let arguments = program(
        &rotator("c, d", "var e = c();", TWO_TERMS, "a, 5"),
        &argument_calls(),
    );
    runs += sweep("call arguments", &arguments, &mut rng, 40);
    let direct = tallied(&DIRECT_TERMS);
    println!("hand-written sessions: {runs} runs, {direct} direct term runs, 0 differ");
    assert!(direct > 0, "no checksum term ran on the direct path");
}

fn generated(rng: &mut Rng, depth: u32, names: &[&str]) -> String {
    if depth == 0 || rng.next().is_multiple_of(4) {
        return match rng.pick(&[
            "0x1", "2", "'s'", "'7'", "true", "null", "''", "-0", "name", "constant",
        ]) {
            "name" => names[(rng.next() % names.len() as u64) as usize].to_string(),
            "constant" => format!("({})", constant(rng, 3)),
            leaf => leaf.to_string(),
        };
    }
    let form = rng.below(17);
    let mut child = || generated(rng, depth - 1, names);
    match child().len() % 6 {
        0 => format!("({} + {})", child(), child()),
        1 => format!("-({})", child()),
        2 => format!("[{}, {}]", child(), child()),
        3 => format!("({} ? {} : {})", child(), child(), child()),
        4 => format!("(function () {{ return {}; }})()", child()),
        _ => match form {
            0 => format!("({{p: {}, [{}]: {}}}).p", child(), child(), child()),
            1 => format!("`t${{{}}}u${{{}}}`", child(), child()),
            2 => format!("({})?.length", child()),
            3 => format!("delete ({{k: {}}}).k", child()),
            4 => format!("(({{v: {}}}).v ??= {})", child(), child()),
            5 => format!("(({{v: {}}}).v += {})", child(), child()),
            6 => format!("({} in {{p: 1}})", child()),
            7 => format!("({} instanceof Object)", child()),
            8 => format!("new (function (x) {{ this.x = x; }})({}).x", child()),
            9 => format!("/s/.test({})", child()),
            10 => format!("['a', -0x1, 0x2 * 0x3, null, {}]", child()),
            11 => format!(
                "(function () {{ var x = [1, 2, 3, 4, 5, 6, 7, 8]; x &&= (x = 0, [9, 9, 9, 9, 9, 9, 9, 9, {}]); return x; }})()",
                child()
            ),
            12 => format!(
                "((t) => (t.p = [1, 2, 3, 4, 5, 6, 7, 8], t.p &&= (t.p = 0, [9, 9, 9, 9, 9, 9, 9, 9, {}])))({{}})",
                child()
            ),
            14 => {
                let inner = child();
                at_the_unchecked_limit(rng, inner)
            }
            15 => {
                let inner = child();
                to_the_nesting_cap(rng, inner)
            }
            13 => {
                let template = rng.pick(STATEMENT_FORMS);
                format!(
                    "(function () {{ var v = 0, fs = []; {} return [v, fs.map((g) => g())]; }})()",
                    template.replace("@", &generated(rng, depth - 1, names))
                )
            }
            _ => format!("({}, {})", child(), child()),
        },
    }
}

fn at_the_unchecked_limit(rng: &mut Rng, inner: String) -> String {
    let tildes = UNCHECKED_LEVELS as usize - 2 + rng.below(2) as usize;
    format!(
        "(function () {{ var v = {inner}; return {}v; }})()",
        "~".repeat(tildes)
    )
}

fn to_the_nesting_cap(rng: &mut Rng, inner: String) -> String {
    let over = rng.below(2) as usize;
    let blocks = rng.below(97) as usize;
    let per_frame = blocks + UNCHECKED_LEVELS as usize + over - 2;
    let frames = match rng.below(2) {
        0 => (NESTING_DEPTH_CAP as u64 / per_frame as u64 + rng.below(5)).saturating_sub(2),
        _ => rng.below(8),
    }
    .min(990);
    format!(
        "(function f(n) {{ {}return n <= 0 ? {inner} : {}f(n - 1);{} }})({frames})",
        "{ ".repeat(blocks),
        "~".repeat(UNCHECKED_LEVELS as usize - 5 + over),
        " }".repeat(blocks)
    )
}

const STATEMENT_FORMS: &[&str] = &[
    "for (let i = 0; i < 2; i++) { fs.push(() => [i, @]); }",
    "for (const k in {p: 1, q: 2}) { fs.push(() => k); v = @; }",
    "for (var k of [@, 1]) { v = k; }",
    "L: while (true) { do { v = @; break L; } while (false); }",
    "M: for (var j = 0; j < 2; j++) { for (;;) { v = @; continue M; } }",
    "switch (v) { case 0: let s = @; fs.push(() => s); case 1: v = s; break; default: v = 1; }",
    "try { v = @; throw v; } catch (e) { let c = e; v = c; } finally { fs.push(() => v); }",
    "try { v = 1; } finally { v = @; }",
    "{ let a = @; const b = () => a; v = b(); }",
    "do { v = @; } while (v === 0 && fs.length > 9);",
    "if (v === 9) { with ({}) { v = 1; } }",
    "if (v === 9) { var {p} = @; }",
    "{ class C {} v = @; }",
];

fn generated_session(rng: &mut Rng) -> String {
    let array = match rng.next() % 3 {
        0 => "a".to_string(),
        1 => format!("({}, a)", generated(rng, 3, &["a", "c", "d"])),
        _ => format!(
            "function () {{ return ({}, a()); }}",
            generated(rng, 3, &["c", "d", "q", "a"])
        ),
    };
    let expected = format!("(({}), 5)", generated(rng, 3, &["c", "d", "q", "a"]));
    let prelude = format!(
        "var q = {}; var e = c();",
        generated(rng, 3, &["c", "d", "a"])
    );
    let calls: String = (0..3)
        .map(|_| format!("b(0x10, {});\n", generated(rng, 4, &["b", "a"])))
        .collect();
    program(
        &rotator("c, d", &prelude, TWO_TERMS, &format!("{array}, {expected}")),
        &calls,
    )
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 300 generated sessions under fuel and heap caps"
)]
fn generated_sessions_equal_the_walker_under_fuel_and_heap_caps() {
    let mut rng = Rng(0x1234_5678_9abc_def1);
    let mut runs = 0;
    for index in 0..300 {
        let source = generated_session(&mut rng);
        runs += sweep(
            &format!("generated {index}: {source}"),
            &source,
            &mut rng,
            4,
        );
    }
    println!("generated sessions: 300 programs, {runs} runs, 0 differ");
}

fn on_a_2_mib_thread(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the row's thread finished");
}

fn deep(label: &'static str, source: String) -> u64 {
    let (sender, receiver) = std::sync::mpsc::channel();
    on_a_2_mib_thread(move || {
        let run = Run {
            heap: Some(heap_budget(source.len())),
            ..FULL
        };
        let (records, census) = same(label, &source, run);
        frozen("compile_entry_sessions deep", &source, &records);
        let _ = sender.send(census.direct_terms);
    });
    receiver.recv().unwrap_or(0)
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: checksum terms 6,000 deep on a 2 MiB thread, where the state collector still fits"
)]
fn deep_checksum_terms_run_on_the_direct_path_as_the_walker_on_a_2_mib_thread() {
    let mut direct = 0;
    for n in [SESSION_TERM_DEPTH / 2, SESSION_TERM_DEPTH] {
        let negated = format!(
            "{}parseInt(b(0x10)){}",
            "-(-(".repeat(n / 2),
            "))".repeat(n / 2)
        );
        let terms = format!("{negated} + parseInt(b(0x12))");
        direct += deep(
            "a deep negated term",
            program(&rotator("c, d", "var e = c();", &terms, "a, 5"), CALLS),
        );
        let chain = format!("parseInt(b(0x10)){}", " + 0".repeat(n));
        direct += deep(
            "a deep term chain",
            program(&rotator("c, d", "var e = c();", &chain, "a, 5"), CALLS),
        );
    }
    let nested = format!(
        "parseInt(b({}0x10{}))",
        "(".repeat(50_000),
        ")".repeat(50_000)
    );
    direct += deep(
        "a deep term argument",
        program(
            &rotator(
                "c, d",
                "var e = c();",
                &format!("{nested} + parseInt(b(0x12))"),
                "a, 5",
            ),
            CALLS,
        ),
    );
    assert!(direct > 0, "no deep term ran on the direct path");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: entries 100,000 deep on a 2 MiB thread"
)]
fn deep_entries_run_compile_and_drop_as_the_walker_on_a_2_mib_thread() {
    let plain = || rotator("c, d", "var e = c();", TWO_TERMS, "a, 5");
    for n in [99_999usize, 100_001] {
        let argument = format!("{}0x10{}", "+(".repeat(n), ")".repeat(n));
        deep(
            "a deep direct argument",
            program(&plain(), &format!("b({argument});\n")),
        );
        let expected = format!("{}5{}", "-(-(".repeat(n / 2), "))".repeat(n / 2));
        deep(
            "a deep expected value",
            program(
                &rotator("c, d", "var e = c();", TWO_TERMS, &format!("a, {expected}")),
                "b(0x10);\n",
            ),
        );
        let prelude = format!(
            "var q = {}1{}; var e = c();",
            "~(".repeat(n / 2),
            ")".repeat(n / 2)
        );
        deep(
            "a deep prelude",
            program(&rotator("c, d", &prelude, TWO_TERMS, "a, 5"), "b(0x10);\n"),
        );
    }
    let compiled = format!("{}b{}", "[".repeat(25_000), "][0]".repeat(25_000));
    deep(
        "a deep compiled argument",
        program(&plain(), &format!("b(0x10, typeof {compiled});\n")),
    );
}

#[test]
fn an_object_literal_argument_compiles_and_is_remembered_by_address() {
    let source = program(
        &rotator("c, d", "var e = c();", TWO_TERMS, "a, 5"),
        "b(0x10, ({ k: 1 }).k);\nb(0x12, [1][0]);",
    );
    let (records, census) = same("an object literal argument", &source, FULL);
    frozen("compile_entry_sessions", &source, &records);
    assert_eq!(census.argument_entries, 2, "{census:?}");
}

fn constant_leaf(rng: &mut Rng) -> String {
    match rng.below(6) {
        0 => format!("0x{:x}", rng.below(4096)),
        1 => format!("-0x{:x}", rng.below(4096)),
        2 => rng.below(100).to_string(),
        3 => format!("{}.5", rng.below(9)),
        4 => rng.pick(&["-0", "0", "1e3", "0x0"]).to_string(),
        _ => format!("0x{:x}", rng.below(16)),
    }
}

fn constant(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 || rng.below(4) == 0 {
        return constant_leaf(rng);
    }
    let child = |rng: &mut Rng| constant(rng, depth - 1);
    match rng.below(16) {
        0 => format!("-({})", child(rng)),
        1 => format!("+({})", child(rng)),
        2 => format!("~({})", child(rng)),
        3 => format!("({}) ** 0x{:x}", child(rng), rng.below(4)),
        4 => rng
            .pick(&["(2 ** 0.5)", "(0x2 ** 0x40)", "(0x2 ** 0x34)"])
            .to_string(),
        5 => format!("({})", child(rng)),
        _ => {
            let op = rng.pick(&["+", "-", "*", "/", "%", "|", "&", "^", "<<", ">>", ">>>"]);
            format!("{} {op} {}", child(rng), child(rng))
        }
    }
}

fn index_constant(rng: &mut Rng, target: i64) -> String {
    let (a, b, c) = (
        rng.below(0x800) as i64,
        rng.below(0x40) as i64,
        rng.below(0x40) as i64,
    );
    let d = a + b * c - target;
    let tail = match d >= 0 {
        true => format!("- 0x{d:x}"),
        false => format!("+ 0x{:x}", -d),
    };
    match rng.below(3) {
        0 => format!("0x{a:x} + 0x{b:x} * 0x{c:x} {tail}"),
        1 => format!("-(-0x{a:x}) + (0x{b:x} * 0x{c:x}) {tail}"),
        _ => format!("(0x{b:x} * 0x{c:x} + 0x{a:x}) {tail}"),
    }
}

fn term_index(rng: &mut Rng) -> String {
    let target = 0x10 + rng.below(7) as i64;
    match rng.below(5) {
        0 => format!("0x{:x}", 0x10 + rng.below(8)),
        1 => format!("-0x1 + 0x{:x}", 0x11 + rng.below(7)),
        2 => format!("0x{:x} - 0x2", 0x12 + rng.below(6)),
        3 => index_constant(rng, target),
        _ => format!("(0x{:x})", 0x10 + rng.below(9)),
    }
}

fn term_call(rng: &mut Rng, wrapper: bool) -> String {
    let inner = match (wrapper, rng.below(5)) {
        (true, 0) => format!("w(0, {})", rng.below(7)),
        (_, 1) => format!(
            "b({}, {})",
            term_index(rng),
            rng.pick(&["'k'", "0x1", "null", "true", "'a' + 'b'", "void 0", "!0"])
        ),
        (_, 2) => format!("b({}, e)", term_index(rng)),
        (_, 3) => format!("b({}, q)", term_index(rng)),
        _ => format!("b({})", term_index(rng)),
    };
    let parse = rng.pick(&["parseInt", "parseInt", "parseFloat", "Number"]);
    format!("{parse}({inner})")
}

fn term_atom(rng: &mut Rng, wrapper: bool, depth: u32) -> String {
    if depth == 0 {
        return term_call(rng, wrapper);
    }
    let atom = |rng: &mut Rng| term_atom(rng, wrapper, depth - 1);
    match rng.below(11) {
        0 => format!("-({})", atom(rng)),
        1 => format!("{} / 0x{:x}", atom(rng), 1 + rng.below(9)),
        2 => format!("({} * ({}))", atom(rng), atom(rng)),
        3 => format!("(-({}) * -({}))", atom(rng), atom(rng)),
        4 => format!("0x{:x}", rng.below(4096)),
        5 => format!("({} - {})", atom(rng), atom(rng)),
        6 => format!("{} % 0x{:x}", atom(rng), 1 + rng.below(7)),
        7 => format!("+({})", atom(rng)),
        8 => format!("({} - z)", atom(rng)),
        9 => format!("{} / ({})", atom(rng), constant(rng, 3)),
        _ => term_call(rng, wrapper),
    }
}

fn grammar_terms(rng: &mut Rng, wrapper: bool) -> String {
    let count = 1 + rng.below(5);
    (0..count)
        .map(|_| {
            let depth = rng.below(3) as u32;
            term_atom(rng, wrapper, depth)
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn grammar_prelude(rng: &mut Rng, wrapper: bool) -> String {
    let prelude = match rng.below(6) {
        4 => format!(
            "var e = c(), o = {{z: {}, q: `q`}}, z = o?.z, q = o.q; delete o.z; z ??= 1; o.q ||= z;",
            term_atom(rng, false, 1)
        ),
        5 => "var e = c(), t = ['a', -0x1, 0x2 * 0x3], z = t[1] + typeof t, q = `${z}`; z = void 0, z ||= 0x2;".to_string(),
        0 => format!("var e = c(), z = {}, q = 'q';", term_atom(rng, false, 1)),
        1 => "var q = 'str', e = c(); var z = 1; var r;".to_string(),
        2 => format!("var e = c(); var z = {}, q = z;", term_call(rng, false)),
        _ => "var z = 0x2, q, e = c();".to_string(),
    };
    if !wrapper {
        return prelude;
    }
    let shift = match rng.below(2) {
        0 => "-0x10".to_string(),
        _ => format!("({})", index_constant(rng, -0x10)),
    };
    format!("function w(x, y) {{ return b(y - {shift}, x); }} {prelude}")
}

fn grammar_session(rng: &mut Rng) -> String {
    let wrapper = rng.below(3) == 0;
    let prelude = grammar_prelude(rng, wrapper);
    let array = rng.pick(&["a", "a", "a", "(0, a)"]);
    let expected = match rng.below(5) {
        0 => format!("0x{:x}", rng.below(64)),
        1 => format!("-0x{:x} + 0x{:x}", rng.below(9), rng.below(64)),
        2 => "parseInt('5')".to_string(),
        3 => constant(rng, 3),
        _ => rng.below(12).to_string(),
    };
    let terms = grammar_terms(rng, wrapper);
    let rotation = rotator("c, d", &prelude, &terms, &format!("{array}, {expected}"));
    let target = 0x10 + rng.below(7) as i64;
    let index = index_constant(rng, target);
    let calls = format!(
        "b(0x10);\nb(0x12, 'k');\nb(0x11 + 0x1);\nb({index}, {});\n",
        constant(rng, 3)
    );
    program(&rotation, &calls)
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: 150 generated checksum-term sessions under fuel and heap caps"
)]
fn generated_checksum_terms_equal_the_walker_under_fuel_and_heap_caps() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    let (direct, folded, fallbacks, scopes) = (
        tallied(&DIRECT_TERMS),
        tallied(&FOLDED),
        tallied(&FALLBACKS),
        tallied(&SCOPES),
    );
    let mut runs = 0;
    for index in 0..150 {
        let source = grammar_session(&mut rng);
        runs += sweep(&format!("grammar {index}: {source}"), &source, &mut rng, 6);
    }
    let direct = tallied(&DIRECT_TERMS) - direct;
    let folded = tallied(&FOLDED) - folded;
    let fallbacks = tallied(&FALLBACKS) - fallbacks;
    let scopes = tallied(&SCOPES) - scopes;
    println!(
        "checksum-term grammar: 150 programs, {runs} runs, {scopes} compiled scopes, {direct} direct terms, {folded} constants folded, {fallbacks} constant fallbacks, 0 differ"
    );
    assert!(direct > 0, "no generated term ran on the direct path");
    assert!(
        scopes > 0,
        "no generated session built the compiled checksum"
    );
    assert!(folded > 0, "no generated constant folded");
    assert!(fallbacks > 0, "no fuel cap landed inside a folded constant");
}

fn takes_a_checksum(prelude: &str) -> bool {
    let source = format!("{};", rotator("c, d", prelude, TWO_TERMS, "a, 5"));
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let Some(oxc_ast::ast::Statement::ExpressionStatement(statement)) = parsed.program.body.first()
    else {
        panic!("the rotator is an expression statement");
    };
    untwist_eval::decode::checksum_from_rotator(&statement.expression).is_some()
}

#[test]
fn a_checksum_prelude_holds_only_declarations_so_its_hoist_never_nests() {
    assert!(takes_a_checksum("var e = c();"));
    assert!(takes_a_checksum(
        "var e = c(); function w(x) { { return x; } }"
    ));
    assert!(!takes_a_checksum("var e = c(); { var q = 1; }"));
    assert!(!takes_a_checksum("var e = c(); if (e) { var q = 1; }"));
    assert!(!takes_a_checksum("var e = c(); while (0) { var q = 1; }"));
}
