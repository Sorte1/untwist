use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units_charged, CompileCensus, UnitRecord,
};

struct Refusal {
    row: &'static str,
    taken: &'static str,
    untaken: &'static str,
    text: &'static str,
}

const fn refusal(
    row: &'static str,
    taken: &'static str,
    untaken: &'static str,
    text: &'static str,
) -> Refusal {
    Refusal {
        row,
        taken,
        untaken,
        text,
    }
}

const REFUSALS: &[Refusal] = &[
    refusal(
        "with",
        "function f(o) { var a = 1; with (o) { a = p; } return a; } f({p: 2});",
        "function f(o, c) { var a = 1; if (c) { with (o) { a = p; } } return a; } f({p: 2}, false);",
        "statement kind",
    ),
    refusal(
        "a direct eval",
        "function f() { var a = 1; return eval('a + 1'); } f();",
        "function f(c) { var a = 1; return c ? eval('a + 1') : a; } f(false);",
        "eval",
    ),
    refusal(
        "Function(...)",
        "function f() { var g = Function('return 1'); return g(); } f();",
        "function f(c) { if (c) { return Function('return 1')(); } return 0; } f(0);",
        "Function",
    ),
    refusal(
        "a class declaration",
        "function f() { var a = 1; class C {} return a; } f();",
        "function f(c) { var a = 1; if (c) { class C {} } return a; } f(false);",
        "statement kind",
    ),
    refusal(
        "a destructuring parameter",
        "function f(a, {b}) { return a + b; } f(1, {b: 2});",
        "function f(a, {b}) { return a + b; } function g(c) { return c ? f(1, {b: 2}) : 3; } g(false);",
        "destructuring parameter",
    ),
    refusal(
        "a rest parameter",
        "function f(a, ...b) { return a; } f(1, 2);",
        "function f(a, ...b) { return a; } function g(c) { return c ? f(1, 2) : 3; } g(false);",
        "rest parameter",
    ),
    refusal(
        "a destructuring var declaration",
        "function f(o) { var a = 1, {p} = o; return a + p; } f({p: 2});",
        "function f(o, c) { var a = 1; if (c) { var {p} = o; } return a; } f({p: 2}, false);",
        "destructuring declaration",
    ),
    refusal(
        "a destructuring let declaration",
        "function f(o) { let a = 1, [b] = o; return a + b; } f([2]);",
        "function f(o, c) { let a = 1; if (c) { let [b] = o; } return a; } f([2], false);",
        "destructuring declaration",
    ),
    refusal(
        "a destructuring catch parameter",
        "function f() { try { throw {m: 1}; } catch ({m}) { return m; } } f();",
        "function f(c) { try { if (c) throw {m: 1}; } catch ({m}) { return m; } return 0; } f(false);",
        "destructuring catch parameter",
    ),
    refusal(
        "a destructuring for-in binding",
        "function f() { var r = 0; for (var [k] in {ab: 1}) { r = k; } return r; } f();",
        "function f() { var r = 0; for (var [k] in {}) { r = k; } return r; } f();",
        "destructuring for-in binding",
    ),
    refusal(
        "using",
        "function f() { var a = 1; { using u = null; } return a; } f();",
        "function f(c) { var a = 1; if (c) { using u = null; } return a; } f(false);",
        "using declaration",
    ),
];

fn parse_and<R>(source: &str, body: impl FnOnce(&Program<'_>) -> R) -> R {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    body(&parsed.program)
}

fn run_in(program: &Program<'_>, fuel: Option<u64>, decoding: bool) -> Vec<UnitRecord> {
    let budget = fuel.map_or_else(Budget::default, Budget::new);
    record_units_charged(|| match decoding {
        true => eval_script_decoding(program, budget, |_| ()),
        false => eval_script_in(program, budget, |_, _| ()),
    })
}

fn compared(label: &str, source: &str, decoding: bool) -> (UnitRecord, CompileCensus) {
    parse_and(source, |program| {
        let mut compiled = Vec::new();
        let census = compile_census(|| compiled = run_in(program, None, decoding));
        let steps: u64 = compiled.iter().map(|unit| unit.steps_used).sum();
        let mut capped = vec![compiled.clone()];
        for cap in 0..=steps + 1 {
            capped.push(run_in(program, Some(cap), decoding));
        }
        frozen(
            "compile_refusals",
            &format!("{label}\n{decoding}\n{source}"),
            std::slice::from_ref(&capped),
        );
        (compiled.remove(0), census)
    })
}

#[test]
fn to_r7_refuse_compiled_at_the_walkers_node_text_and_step() {
    for refusal in REFUSALS {
        for decoding in [false, true] {
            let label = format!("{} taken", refusal.row);
            let (unit, census) = compared(&label, refusal.taken, decoding);
            let bail = unit.bail.as_deref().unwrap_or_default();
            assert!(
                bail.contains(refusal.text),
                "{label} (decoding {decoding}): {unit:?}"
            );
            assert!(census.calls > 0, "{label}: no call entered compiled");
        }
    }
}

#[test]
fn to_r7_in_an_untaken_branch_do_not_refuse() {
    for refusal in REFUSALS {
        for decoding in [false, true] {
            let label = format!("{} untaken", refusal.row);
            let (unit, census) = compared(&label, refusal.untaken, decoding);
            assert_eq!(unit.bail, None, "{label} (decoding {decoding}): {unit:?}");
            assert!(census.calls > 0, "{label}: no call entered compiled");
        }
    }
}

const DEAD_ZONES: &[(&str, &str)] = &[
    (
        "an early let while decoding",
        "function f() { var r = x; let x = 1; return r; } f();",
    ),
    (
        "an early let in an untaken branch while decoding",
        "function f(c) { if (c) { var r = x; let x = 1; } return 0; } f(false);",
    ),
    (
        "an early class while decoding",
        "function f() { var r = C; class C {} return r; } f();",
    ),
];

#[test]
fn an_early_let_or_class_while_decoding_refuses_before_any_step_as_the_walker() {
    for (label, source) in DEAD_ZONES {
        let (unit, _) = compared(label, source, true);
        assert!(
            unit.bail
                .as_deref()
                .is_some_and(|bail| bail.contains("temporal dead zone")),
            "{label}: {unit:?}"
        );
        assert_eq!(unit.steps_used, 0, "{label}: {unit:?}");
        let (_, census) = compared(label, source, false);
        assert!(
            census.calls > 0,
            "{label}: no call entered compiled outside decoding"
        );
    }
}
