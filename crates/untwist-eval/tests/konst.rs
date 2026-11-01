use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::interp::{eval_script_decoding, eval_script_in};
use untwist_eval::test_hooks::{
    compile_census, frozen, nesting_at_first_bail, record_units, CompileCensus, UnitRecord,
};

struct Seen {
    records: Vec<UnitRecord>,
    census: CompileCensus,
}

fn run(source: &str, decoding: bool) -> (Vec<UnitRecord>, Option<u32>) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = &parsed.program;
    let mut records = Vec::new();
    let nesting = nesting_at_first_bail(|| {
        records = record_units(|| match decoding {
            true => eval_script_decoding(program, Budget::default(), |_| ()),
            false => eval_script_in(program, Budget::default(), |_, _| ()),
        });
    });
    (records, nesting)
}

fn compared(label: &str, source: &str, decoding: bool) -> Seen {
    let mut compiled = (Vec::new(), None);
    let census = compile_census(|| compiled = run(source, decoding));
    frozen(
        "konst",
        &format!("{label}\n{decoding}\n{source}"),
        std::slice::from_ref(&compiled),
    );
    assert!(census.calls > 0, "{label}: no call entered compiled");
    Seen {
        records: compiled.0,
        census,
    }
}

fn returned(constant: &str) -> String {
    format!("(function () {{ return {constant}; }})();")
}

fn folds(label: &str, constant: &str, bits: u64) {
    for decoding in [false, true] {
        let seen = compared(label, &returned(constant), decoding);
        assert_eq!(seen.census.constants, 1, "{label}: {:?}", seen.census);
        assert_eq!(
            seen.records[0].values,
            [format!("number:{bits:016x}")],
            "{label}"
        );
    }
}

fn unfolded(label: &str, source: &str, value: &str) {
    for decoding in [false, true] {
        let seen = compared(label, &returned(source), decoding);
        assert_eq!(seen.census.constants, 0, "{label}: {:?}", seen.census);
        if !decoding {
            assert_eq!(seen.records[0].values, [value], "{label}");
        }
    }
}

const NAN: u64 = 0x7ff8_0000_0000_0000;

#[test]
fn to_k6_numeric_constants_fold_to_the_walkers_bits() {
    folds("-0, node -0", "-0", 0x8000_0000_0000_0000);
    folds("1/0, node Infinity", "1/0", 0x7ff0_0000_0000_0000);
    folds(
        "the high preset's constant, node 256",
        "-0x1da3+0x4*0x56f+0x2b*0x35",
        0x4070_0000_0000_0000,
    );
    folds("~-1 >>> 0, node 0", "~-1 >>> 0", 0);
    folds(
        "2**53 + 1, node 9007199254740992",
        "2**53 + 1",
        0x4340_0000_0000_0000,
    );
    folds(
        "parenthesised, node 256",
        "(-0x1da3+0x4*0x56f+0x2b*0x35)",
        0x4070_0000_0000_0000,
    );
}

#[test]
fn zero_over_zero_folds_to_the_walkers_nan() {
    for decoding in [false, true] {
        let seen = compared("0/0, node NaN", &returned("0/0"), decoding);
        assert_eq!(seen.census.constants, 1, "{:?}", seen.census);
        let rendered = &seen.records[0].values[0];
        let bits = u64::from_str_radix(rendered.trim_start_matches("number:"), 16).unwrap();
        assert!(f64::from_bits(bits).is_nan(), "{rendered}");
        assert_eq!(bits & NAN, NAN, "{rendered}");
    }
}

#[test]
fn to_k6_what_is_not_a_constant_runs_per_node() {
    unfolded(
        "-x, node -2",
        "(function (x) { return -x; })(2)",
        "number:c000000000000000",
    );
    unfolded("+'1', node 1", "+'1'", "number:3ff0000000000000");
    unfolded("0x1 - '1', node 0", "0x1 - '1'", "number:0000000000000000");
    unfolded("~[], node -1", "~[]", "number:bff0000000000000");
    unfolded("(1, 2), node 2", "(1, 2)", "number:4000000000000000");
}

#[test]
fn no_gate_node_sits_inside_a_constant() {
    for (label, source, constants) in [
        ("1 && 2", "1 && 2", 0),
        ("0 || 1", "0 || 1", 0),
        ("null ?? 1", "null ?? 1", 0),
        ("1 ? 2 : 3", "1 ? 2 : 3", 0),
        ("a gate beside a constant", "(0 || 1) + (2 * 3)", 1),
        ("constants in both arms", "1 ? 2 + 3 : -4", 2),
        ("a constant under a logical", "(1 + 1) && -(2)", 2),
    ] {
        for decoding in [false, true] {
            let seen = compared(label, &returned(source), decoding);
            assert_eq!(seen.census.constants, constants, "{label}");
        }
    }
}

fn bails_unfolded(label: &str, constant: &str) {
    let seen = compared(label, &returned(constant), true);
    assert_eq!(seen.census.constants, 0, "{label}: {:?}", seen.census);
    assert_eq!(
        seen.records[0].bail.as_deref(),
        Some("unsupported construct: a power whose result the evaluator cannot prove exact"),
        "{label}: {:?}",
        seen.records[0]
    );
}

#[test]
fn an_inexact_power_is_not_folded_and_bails_at_the_walkers_step() {
    bails_unfolded("2 ** 0.5", "2 ** 0.5");
    bails_unfolded("0x2 ** 0x40, past EXACT_LIMIT", "0x2 ** 0x40");
    bails_unfolded("inside a larger constant", "1 + (0x2 ** 0x40) * 3");
    folds(
        "0x2 ** 0x34, exact, node 4503599627370496",
        "0x2 ** 0x34",
        0x4330_0000_0000_0000,
    );
}

#[test]
fn comparisons_are_not_folded() {
    unfolded("0x1 < 0x2, node true", "0x1 < 0x2", "bool:true");
    unfolded(
        "-(0x1 === 0x1), node -1",
        "-(0x1 === 0x1)",
        "number:bff0000000000000",
    );
    let seen = compared(
        "beside a constant",
        &returned("(0x1 < 0x2) + (1 + 2)"),
        false,
    );
    assert_eq!(seen.census.constants, 1, "{:?}", seen.census);
}

#[test]
fn a_constant_is_folded_once_and_run_on_every_call() {
    let source = "var r = 0; function f() { return 0x10 * -0x2 + 0x21; } for (var i = 0; i < 50; i++) r += f(); r;";
    let seen = compared("a constant in a loop, node 50", source, false);
    assert_eq!(seen.census.constants, 1, "{:?}", seen.census);
    assert_eq!(seen.census.constant_runs.len(), 50, "{:?}", seen.census);
    assert_eq!(seen.records[0].values, ["number:4049000000000000"]);
}

fn deep_constant(n: usize) -> String {
    format!("{}0x7{}", "-(".repeat(n), ")".repeat(n))
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release-only: a constant 100,000 deep on a 2 MiB thread"
)]
fn a_deep_constant_folds_runs_and_bails_where_the_walker_does_on_a_2_mib_thread() {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(|| {
            for n in [99_990usize, 100_010, 150_000] {
                let source = format!("(() => {})();", deep_constant(n));
                let alloc = Allocator::default();
                let parsed = untwist_eval::stack::grow(1 << 30, || parse_program(&alloc, &source));
                assert!(parsed.diagnostics.is_empty());
                let program = &parsed.program;
                let script = || eval_script_in(program, Budget::default(), |_, _| ());
                let mut compiled = Vec::new();
                let census = compile_census(|| compiled = record_units(script));
                frozen("konst deep", &n.to_string(), &compiled);
                assert_eq!(census.constants, 1, "a constant {n} deep");
                assert_eq!(
                    census.constant_fallbacks,
                    u64::from(n > 100_000),
                    "a constant {n} deep"
                );
            }
        })
        .expect("a thread")
        .join()
        .expect("the row's thread finished");
}
