mod common;

use untwist_eval::budget::Budget;
use untwist_eval::decode::{Evaluator, InterpEvaluator};

const UNPROVEN: &str = "a stateful decoder keeps state that is not a proven memo or init gate";

const ARRAY: &str =
    "function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }\n";

const INIT_GATE: &str =
    "if (b.i === undefined) { b.d = function (x) { return x + '!'; }; b.c = {}; b.i = true; }";

const FIRST: &str = "var z = e[0];";

const RESET: &str = "b.s !== z && (b.c = {}, b.s = z);";

const LOOKUP: &str =
    "var m = b.c[f];\n  m === undefined ? (v = b.d(v), b.c[f] = v) : v = m;\n  return v;";

const CALLS: &str = "b(0); b(1); b(2); b(1);";

const DECODED: [&str; 4] = ["s0!", "s1!", "s2!", "s1!"];

struct Decoder<'s> {
    prefix: &'s str,
    shift: &'s str,
    first: &'s str,
    reset: &'s str,
    lookup: &'s str,
    calls: &'s str,
}

const GENERATED: Decoder<'static> = Decoder {
    prefix: "",
    shift: "f = f - 0;",
    first: FIRST,
    reset: RESET,
    lookup: LOOKUP,
    calls: CALLS,
};

fn source(decoder: &Decoder<'_>) -> String {
    format!(
        "{ARRAY}{prefix}function b(f, g) {{\n  {shift}\n  var e = a();\n  var v = e[f];\n  {INIT_GATE}\n  {first}\n  {reset}\n  {lookup}\n}}\n{calls}",
        prefix = decoder.prefix,
        shift = decoder.shift,
        first = decoder.first,
        reset = decoder.reset,
        lookup = decoder.lookup,
        calls = decoder.calls,
    )
}

fn decode(source: &str, decoders: &[&str]) -> Result<Vec<String>, String> {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", decoders, None);
        let calls = common::decoder_calls_anywhere(program, decoders);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        evaluator
            .decode(&decoder_program, &calls)
            .map(|values| values.iter().map(common::render_decoded).collect())
            .map_err(|bail| bail.to_string())
    })
}

fn assert_decodes(source: &str, expected: &[&str]) {
    let expected: Vec<String> = expected.iter().map(|s| format!("string:{s}")).collect();
    assert_eq!(decode(source, &["b"]), Ok(expected), "source:\n{source}");
}

fn assert_refused(source: &str, reason: &str) {
    assert_eq!(
        decode(source, &["b"]),
        Err(format!("unsupported construct: {reason}")),
        "source:\n{source}"
    );
}

#[test]
fn a_memo_reset_gate_on_the_first_array_element_decodes() {
    assert_decodes(&source(&GENERATED), &DECODED);
}

#[test]
fn a_memo_reset_gate_written_as_an_if_statement_decodes() {
    let decoder = Decoder {
        reset: "if (b.s !== z) { b.c = {}; b.s = z; }",
        ..GENERATED
    };
    assert_decodes(&source(&decoder), &DECODED);
}

#[test]
fn a_memo_reset_gate_whose_index_is_a_constant_expression_decodes() {
    let decoder = Decoder {
        first: "var z = e[0x31d + -0x575 + 0x258];",
        ..GENERATED
    };
    assert_decodes(&source(&decoder), &DECODED);
}

#[test]
fn a_memo_reset_gate_with_the_state_compared_on_the_right_decodes() {
    let decoder = Decoder {
        reset: "z !== b.s && (b.c = {}, b.s = z);",
        ..GENERATED
    };
    assert_decodes(&source(&decoder), &DECODED);
}

#[test]
fn a_memo_reset_gate_that_does_anything_else_is_refused() {
    let rows: &[(&str, Decoder<'_>, &str)] = &[
        (
            "a third write in the gate",
            Decoder {
                reset: "b.s !== z && (b.c = {}, b.s = z, b.t = 1);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a third statement in the if form",
            Decoder {
                reset: "if (b.s !== z) { b.c = {}; b.s = z; b.t = 1; }",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "an outer variable written in the gate",
            Decoder {
                prefix: "var n = 0;\n",
                reset: "b.s !== z && (b.c = {}, b.s = z, n = 1);",
                ..GENERATED
            },
            "a stateful decoder writes a variable outside its own machinery",
        ),
        (
            "a memo reset to a non-empty object",
            Decoder {
                reset: "b.s !== z && (b.c = { 0: 'forged!' }, b.s = z);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a memo reset to an array",
            Decoder {
                reset: "b.s !== z && (b.c = [], b.s = z);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a state that stores something else",
            Decoder {
                reset: "b.s !== z && (b.c = {}, b.s = z + 'x');",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a state stored before the reset",
            Decoder {
                reset: "b.s !== z && (b.s = z, b.c = {});",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a loose comparison",
            Decoder {
                reset: "b.s != z && (b.c = {}, b.s = z);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate that resets a key other than the memo",
            Decoder {
                reset: "b.s !== z && (b.k = {}, b.s = z);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate on the second array element",
            Decoder {
                first: "var z = e[1];",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate on another array",
            Decoder {
                first: "var w = ['s0'];\n  var z = w[0];",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate on an array that is reassigned",
            Decoder {
                first: "var z = e[0];\n  e = ['t0', 't1', 't2'];",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate value that is reassigned",
            Decoder {
                first: "var z = e[0];\n  z = 'other';",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a gate value that escapes into the result",
            Decoder {
                lookup: "var m = b.c[f];\n  m === undefined ? (v = b.d(v), b.c[f] = v) : v = m;\n  return v + z.slice(9);",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a state read outside the gate",
            Decoder {
                lookup: "var m = b.c[f];\n  m === undefined ? (v = b.d(v), b.c[f] = v) : v = m;\n  return b.s === 'none' ? m : v;",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a memo read outside its lookup",
            Decoder {
                lookup: "var held = b.c;\n  var m = b.c[f];\n  m === undefined ? (v = b.d(v), b.c[f] = v) : v = m;\n  return held === b.c ? v : v;",
                ..GENERATED
            },
            UNPROVEN,
        ),
        (
            "a second reset gate on the same state",
            Decoder {
                reset: "b.s !== z && (b.c = {}, b.s = z);\n  b.s !== z && (b.c = {}, b.s = z);",
                ..GENERATED
            },
            UNPROVEN,
        ),
    ];
    for (row, decoder, reason) in rows {
        let text = source(decoder);
        let result = decode(&text, &["b"]);
        assert_eq!(
            result,
            Err(format!("unsupported construct: {reason}")),
            "{row}:\n{text}"
        );
    }
}

#[test]
fn a_memo_reset_gate_inside_the_miss_branch_is_refused() {
    let decoder = Decoder {
        reset: "",
        lookup: "var m = b.c[f];\n  if (m === undefined) { b.s !== z && (b.c = {}, b.s = z); v = b.d(v); b.c[f] = v; } else { v = m; }\n  return v;",
        ..GENERATED
    };
    assert_refused(&source(&decoder), UNPROVEN);
}

#[test]
fn a_reset_gate_in_one_decoder_on_another_decoders_memo_is_refused() {
    let text = format!(
        "{ARRAY}function c(f) {{ var e = a(); return e[f]; }}\nfunction b(f, g) {{\n  f = f - 0;\n  var e = a();\n  var v = e[f];\n  {INIT_GATE}\n  {FIRST}\n  c.s !== z && (b.c = {{}}, c.s = z);\n  {LOOKUP}\n}}\n{CALLS} c(0);"
    );
    assert_eq!(
        decode(&text, &["b", "c"]).err(),
        Some(format!("unsupported construct: {UNPROVEN}")),
        "source:\n{text}"
    );
}

#[test]
fn a_memo_reset_gate_whose_memo_another_decoder_reads_is_refused() {
    let text = format!(
        "{ARRAY}function c(f) {{ var e = a(); return b.c === undefined ? e[f] : e[f]; }}\nfunction b(f, g) {{\n  f = f - 0;\n  var e = a();\n  var v = e[f];\n  {INIT_GATE}\n  {FIRST}\n  {RESET}\n  {LOOKUP}\n}}\n{CALLS} c(0);"
    );
    assert_eq!(
        decode(&text, &["b", "c"]).err(),
        Some(format!("unsupported construct: {UNPROVEN}")),
        "source:\n{text}"
    );
}

const MIXED_INDEXES: &str = "b('0x1'); b(1); b(2); b('0x2'); b(' 2 ');";

#[test]
fn a_shifted_index_passed_as_a_hex_string_and_as_a_number_shares_its_memo_entry() {
    let decoder = Decoder {
        calls: MIXED_INDEXES,
        ..GENERATED
    };
    assert_decodes(&source(&decoder), &["s1!", "s1!", "s2!", "s2!", "s2!"]);
}

#[test]
fn a_string_and_a_number_index_stay_distinct_unless_the_decoder_only_shifts_it() {
    let rows: &[(&str, Decoder<'_>)] = &[
        (
            "an index scaled instead of shifted",
            Decoder {
                shift: "f = f * 1;",
                calls: MIXED_INDEXES,
                ..GENERATED
            },
        ),
        (
            "an index shifted by a string",
            Decoder {
                shift: "f = f - '0';",
                calls: MIXED_INDEXES,
                ..GENERATED
            },
        ),
        (
            "an index read before its shift",
            Decoder {
                shift: "var raw = f;\n  f = f - 0;",
                calls: MIXED_INDEXES,
                ..GENERATED
            },
        ),
        (
            "a decoder that reads its arguments object",
            Decoder {
                lookup: "var m = b.c[f];\n  m === undefined ? (v = b.d(v), b.c[f] = v) : v = m;\n  return arguments.length > 9 ? m : v;",
                calls: MIXED_INDEXES,
                ..GENERATED
            },
        ),
    ];
    for (row, decoder) in rows {
        let text = source(decoder);
        assert_eq!(
            decode(&text, &["b"]),
            Err(format!("unsupported construct: {UNPROVEN}")),
            "{row}:\n{text}"
        );
    }
}
