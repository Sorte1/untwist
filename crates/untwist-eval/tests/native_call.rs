use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{record_units_charged, UnitRecord};
use untwist_testing::node::printed_by_node;

fn unit(body: &str, decoding: bool) -> UnitRecord {
    let source = format!("String((function () {{ {body} }})());");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let mut units = record_units_charged(|| match decoding {
        true => eval_script_decoding(&parsed.program, Budget::default(), |_| ()),
        false => eval_script_in(&parsed.program, Budget::default(), |_, _| ()),
    });
    units.remove(0)
}

fn node_prints(body: &str) -> Option<String> {
    let program = format!("console.log(String((function () {{ {body} }})()));");
    printed_by_node(&program).map(|printed| printed.trim_end().to_string())
}

const DECODED: &[(&str, &str)] = &[
    ("var c = [104, 105]; return String.fromCharCode.apply(null, c)", "hi"),
    ("var s = 'abc', i = 1; return ''.charCodeAt.call(s, i)", "98"),
    ("return (function () { return [].slice.call(arguments).join('-') })(1, 2, 3)", "1-2-3"),
    ("return (function (a) { a = 9; return [].slice.call(arguments, 1).join('-') + a })(1, 2, 3)", "2-39"),
    ("return (function (a) { a = 9; return [].slice.call(arguments).join('-') })(1, 2, 3)", "9-2-3"),
    ("return (function () { 'use strict'; return typeof [].slice.call(arguments, 0, 1) })(1, 2)", "object"),
    ("return parseInt.bind(null)('7')", "7"),
    ("return String.fromCharCode.bind(null, 104)(105)", "hi"),
    ("return parseInt.call.call(parseInt, null, '7')", "7"),
];

#[test]
fn a_native_called_through_call_apply_or_bind_answers_as_node_in_both_modes() {
    for (body, expected) in DECODED {
        if let Some(printed) = node_prints(body) {
            assert_eq!(printed, *expected, "node on {body}");
        }
        for decoding in [false, true] {
            let got = unit(body, decoding);
            assert_eq!(
                got.values,
                [format!("string:{expected}")],
                "decoding={decoding} {body}: {got:?}"
            );
        }
    }
}

const BOUND_PAST_THE_CAP: &str =
    "var f = parseInt; for (var k = 0; k < 65; k++) f = f.bind(null); return f('7')";

const REFUSED: &[(&str, &str)] = &[
    (
        "return eval.call(null, '1+1')",
        "hostile global eval reached",
    ),
    (
        "return typeof Function.call(null, 'return this')",
        "hostile global Function reached",
    ),
    (
        "return setTimeout.apply(null, [function () {}, 0])",
        "hostile global setTimeout reached",
    ),
    (
        "return Object.getPrototypeOf(parseInt).call.call(eval, null, '1+1')",
        "hostile global eval reached",
    ),
    (
        "return parseInt.call.call(eval, null, '1+1')",
        "hostile global eval reached",
    ),
    (BOUND_PAST_THE_CAP, "bound function chain too deep"),
];

#[test]
fn a_hostile_native_or_a_bound_chain_past_the_cap_keeps_its_note_in_both_modes() {
    for (body, note) in REFUSED {
        for decoding in [false, true] {
            let got = unit(body, decoding);
            assert_eq!(
                got.bail.as_deref(),
                Some(*note),
                "decoding={decoding} {body}: {got:?}"
            );
        }
    }
}

const PATCHED_CALL: &str = "var fp = Object.getPrototypeOf(parseInt); fp.call = function () { return 'C' }; return String.prototype.slice.call('hello', 0, 2)";

#[test]
fn a_patched_function_prototype_call_stays_refused_while_decoding_and_is_honoured_outside() {
    let decoding = unit(PATCHED_CALL, true);
    assert_eq!(
        decoding.bail.as_deref(),
        Some("unsupported construct: a write to a property whose attributes or accessors the evaluator does not model"),
        "{decoding:?}"
    );
    assert_eq!(unit(PATCHED_CALL, false).values, ["string:C"]);
}

const UNMODELLED_ARGUMENTS: &[(&str, &str)] = &[
    ("return (function () { arguments[5] = 7; return [].slice.call(arguments).length })(1, 2)", "2"),
    ("return (function () { arguments.length = 1; return [].slice.call(arguments).join('-') })(1, 2)", "1"),
    ("return (function () { delete arguments[0]; return 0 in [].slice.call(arguments) })(1, 2)", "false"),
    ("return (function () { arguments.x = 1; return [].slice.call(arguments).length })(1, 2)", "2"),
];

#[test]
fn a_slice_of_a_rewritten_arguments_object_answers_as_node_or_bails_in_both_modes() {
    for (body, expected) in UNMODELLED_ARGUMENTS {
        if let Some(printed) = node_prints(body) {
            assert_eq!(printed, *expected, "node on {body}");
        }
        for decoding in [false, true] {
            let got = unit(body, decoding);
            let answered = got.values == [format!("string:{expected}")];
            assert!(
                answered || got.bail.is_some(),
                "decoding={decoding} {body}: {got:?}"
            );
        }
    }
}
