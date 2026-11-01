use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{compile_census, frozen, record_units_charged, UnitRecord};

fn charged(source: &str, decoding: bool) -> (Vec<UnitRecord>, u64) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = &parsed.program;
    let run = || match decoding {
        true => eval_script_decoding(program, Budget::default(), |_| ()),
        false => eval_script_in(program, Budget::default(), |_, _| ()),
    };
    let mut records = Vec::new();
    let census = compile_census(|| records = record_units_charged(run));
    (records, census.calls)
}

fn equal_charged(label: &str, source: &str) -> Vec<UnitRecord> {
    let mut last = Vec::new();
    for decoding in [false, true] {
        let (compiled, calls) = charged(source, decoding);
        assert!(calls > 0, "{label}: no call entered compiled");
        assert!(compiled[0].heap_peak > 0, "{label}: no ledger");
        frozen(
            "compile_held_values",
            &format!("{label}\n{decoding}\n{source}"),
            &compiled,
        );
        last = compiled;
    }
    last
}

#[test]
fn a_compiled_if_and_while_hold_their_test_value_across_the_arm_as_the_walker_does() {
    for (label, source) in [
        (
            "if, an array test",
            "function f() { if (['a', 'b']) { var x = [1]; } return 1; } f();",
        ),
        (
            "if, a built string test",
            "function f(s) { if (s + s) { var x = s + 'y'; } return 1; } f('abcdefgh');",
        ),
        (
            "while, an array test",
            "function f() { var n = 2; while (n-- ? [n] : 0) { var x = [1, 2]; } return 1; } f();",
        ),
    ] {
        equal_charged(label, source);
    }
}

#[test]
fn the_other_compiled_statements_drop_their_operands_where_the_walker_does() {
    for (label, source) in [
        ("for, the test", "function f() { for (var n = 2; n-- ? [n] : 0; ) { var x = [1, 2]; } return 1; } f();"),
        ("for, the update", "function f() { for (var n = 0; n < 2; [n++]) { var x = [1, 2]; } return 1; } f();"),
        ("switch, the discriminant", "function f() { switch ([1]) { default: var x = [1, 2]; } return 1; } f();"),
        ("switch, a case test", "function f() { switch (1) { case [2]: break; default: var x = [1, 2]; } return 1; } f();"),
        ("return", "function f() { return [1]; } f();"),
        ("throw", "function f() { try { throw [1]; } catch (e) { var x = [2]; } return 1; } f();"),
        ("an expression statement", "function f() { [1, 2]; var x = [3]; return 1; } f();"),
        ("a var initialiser", "function f() { var a = [1], b = [a, 2]; return b.length; } f();"),
    ] {
        equal_charged(label, source);
    }
}

#[test]
fn a_charged_if_test_at_the_heap_budget_bails_compiled_where_the_walker_bails() {
    let source = "function f() { var a = 'aaaaaaaa'; for (var i = 0; i < 19; i++) a += a; var b = a + a; var s = b + a; a = null; b = null; if (s + 'x') { var t = s + 'y'; return 'done'; } return 'no'; } f();";
    let records = equal_charged("the budget flip", source);
    assert_eq!(
        records[0].bail.as_deref(),
        Some("the input's heap budget is spent"),
        "{records:?}"
    );
}
