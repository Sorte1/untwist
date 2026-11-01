mod common;

use oxc_allocator::Allocator;
use rustc_hash::FxHashSet;
use untwist_core::parse::parse_program;
use untwist_eval::budget::{heap_budget, Budget};
use untwist_eval::decode::{Call, InterpEvaluator};
use untwist_eval::heap::HeapLedger;
use untwist_eval::rotation::RotationMemo;
use untwist_eval::test_hooks::{
    compile_census, frozen, record_units, render_value, CompileCensus, UnitRecord,
};

const OUT_OF_RANGE_READ: &str = "\
function a() { var h = ['x', '1a', '2a', '3a', '4a', '5a']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0;
    var r = e[g];
    if (b.k === undefined) { c = arguments; b.k = true; }
    var n = f + e[0], hit = c[n];
    var v;
    if (!hit) { v = e[f] + '!'; c[n] = v; } else v = hit;
    return v;
  }), b(c, d);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(0, 0)) + parseInt(b(1, 99)) + parseInt(b(2, 1));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 6);
b(0, 0);
b(1, 99);
b(2, 1);";

const UNFIRED_GATE: &str = "\
function a() { var h = ['x', '1a', '2a', '3a']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0;
    if (b.k === undefined) { c = arguments; b.k = true; }
    var n = f + e[0], hit = c[n];
    var v;
    if (!hit) { v = e[f] + '!'; c[n] = v; } else v = hit;
    return v;
  }), b(c, d);
}
function p(c, d) {
  var e = a();
  return (p = function (f, g) {
    f = f - 0;
    if (p.q === undefined) { c = arguments; p.q = true; }
    var n = f + e[0], hit = c[n];
    var v;
    if (!hit) { v = e[f] + '?'; c[n] = v; } else v = hit;
    return v;
  }), p(c, d);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(0)) + parseInt(p(1));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 3);
b(0);
b(1);";

const WRAPPED_STATELESS: &str = "\
function a() { var h = ['4', 'x', '1', '2', '3']; a = function () { return h; }; return a(); }
function b(i) { i = i - 0x10; var e = a(); return e[i]; }
(function (c, d) {
  function w(x, y) { return b(y - -0x10, x); }
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(w(0, 0)) / 0x1 + -parseInt(b(0x11)) * (parseInt(w(0, 2)) / 0x2) + parseInt(b(0x13));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) { e['push'](e['shift']()); }
  }
})(a, 2);
b(0x10);
b(0x12);";

const ARGUMENTS: &str = "\
function arr() { return ['a', 'b', 'c', 'd']; }
function dec(i, k) { return arr()[i] + (k === undefined ? '' : k); }
dec(0);
dec(-0x1 + 2);
dec(1, 'x');
dec(2, true);
dec(3, null);
dec(~-1 >>> 0, 'y' + 'z');
dec((0, 3));
dec([1][0]);
dec(0 ? 1 : 2, typeof dec);
dec(+'2');";

const LITERAL_ARGUMENTS: u64 = 11;
const COMPILED_ARGUMENTS: u64 = 4;

fn both(label: &str, key: &str, run: impl Fn()) -> (Vec<UnitRecord>, CompileCensus) {
    let mut compiled = Vec::new();
    let census = compile_census(|| compiled = record_units(&run));
    assert!(!compiled.is_empty(), "{label}: no unit");
    frozen("compile_entries", key, &compiled);
    (compiled, census)
}

fn decode(source: &str, array: &str, decoders: &[&str]) -> Vec<CompileCensus> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let program = common::decoder_program(&parsed.program, array, decoders, Some(0));
    let found = common::calls_for(&parsed.program, 0);
    let mut censuses = Vec::new();
    for heap in [false, true] {
        let label = format!("{array} with the heap ledger {heap}");
        let (records, census) = both(&label, &format!("{label}\n{source}"), || {
            let mut evaluator = InterpEvaluator::new(Budget::default());
            if heap {
                evaluator.heap = Some(HeapLedger::new(heap_budget(source.len())));
            }
            let _ = evaluator.decode_partial_with_memo(
                &program,
                &found,
                &mut RotationMemo::default(),
                &FxHashSet::default(),
            );
        });
        assert!(!records.is_empty(), "{label}: no unit");
        assert!(
            records.iter().all(|unit| unit.bail.is_none()),
            "{label}: {records:?}"
        );
        censuses.push(census);
    }
    censuses
}

fn twice<'a>(calls: Vec<Call<'a>>) -> Vec<Call<'a>> {
    let again = calls.iter().map(|call| Call {
        decoder: call.decoder,
        span: call.span,
        arguments: call.arguments.clone(),
    });
    let mut doubled: Vec<Call<'a>> = again.collect();
    doubled.extend(calls);
    doubled
}

fn attempts_of(records: &[UnitRecord], kind: &str) -> usize {
    records
        .iter()
        .flat_map(|unit| unit.attempts.iter())
        .filter(|line| line.starts_with(kind))
        .count()
}

fn recorded(source: &str, array: &str, decoders: &[&str]) -> Vec<UnitRecord> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let program = common::decoder_program(&parsed.program, array, decoders, Some(0));
    let calls = common::calls_for(&parsed.program, 0);
    record_units(|| {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let _ = evaluator.decode_partial_with_memo(
            &program,
            &calls,
            &mut RotationMemo::default(),
            &FxHashSet::default(),
        );
    })
}

#[test]
fn the_rotation_search_records_each_attempts_terms_compiled_as_in_the_walker() {
    let sample = common::read_fixture("obfuscator.io.js");
    let cases: [(&str, &str, &str, &[&str]); 4] = [
        (
            "the obfuscator.io sample, dead terms skipped",
            &sample,
            "c",
            &["d"],
        ),
        (
            "a wrapper in the prelude, dead terms skipped",
            WRAPPED_STATELESS,
            "a",
            &["b"],
        ),
        (
            "a stateful decoder, dead terms probed",
            OUT_OF_RANGE_READ,
            "a",
            &["b"],
        ),
        (
            "two stateful decoders, dead terms probed",
            UNFIRED_GATE,
            "a",
            &["b", "p"],
        ),
    ];
    for (label, source, array, decoders) in cases {
        let records = recorded(source, array, decoders);
        assert!(attempts_of(&records, "attempt") > 0, "{label}: {records:?}");
        assert!(attempts_of(&records, "term") > 0, "{label}: {records:?}");
        for census in decode(source, array, decoders) {
            assert!(census.checksum_scopes > 0, "{label}: {census:?}");
            assert!(census.checksum_terms > 0, "{label}: {census:?}");
        }
    }
}

#[test]
fn dead_terms_read_the_same_values_through_the_compiled_checksum_frame() {
    for (label, source, decoders) in [
        ("an out-of-range read", OUT_OF_RANGE_READ, &["b"][..]),
        ("an unfired gate", UNFIRED_GATE, &["b", "p"][..]),
    ] {
        let records = recorded(source, "a", decoders);
        let dead = attempts_of(&records, "dead term");
        assert!(dead > 0, "{label}: no dead term was read: {records:?}");
        for census in decode(source, "a", decoders) {
            assert!(census.checksum_terms > 0, "{label}: {census:?}");
        }
    }
}

#[test]
fn a_prelude_wrapper_enters_compiled_inside_the_checksum_frame() {
    let censuses = decode(WRAPPED_STATELESS, "a", &["b"]);
    let marker = WRAPPED_STATELESS.find("function w(").expect("the wrapper") as u32;
    for census in censuses {
        assert!(
            census.functions.iter().any(|(start, _)| *start == marker),
            "the wrapper ran in the walker: {census:?}"
        );
    }
}

#[test]
fn a_literal_call_argument_takes_the_direct_path_and_another_compiles_once() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, ARGUMENTS);
    let program = common::decoder_program(&parsed.program, "arr", &["dec"], None);
    let calls = twice(common::calls_for(&parsed.program, 0));
    let (records, census) = both("the call arguments", ARGUMENTS, || {
        let mut evaluator = InterpEvaluator::new(Budget::default());
        evaluator.heap = Some(HeapLedger::new(heap_budget(ARGUMENTS.len())));
        let _ = evaluator.decode_partial_with_memo(
            &program,
            &calls,
            &mut RotationMemo::default(),
            &FxHashSet::default(),
        );
    });
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0].site_steps.len(), calls.len(), "{records:?}");
    assert_eq!(census.direct_arguments, 2 * LITERAL_ARGUMENTS, "{census:?}");
    assert_eq!(
        census.compiled_arguments,
        2 * COMPILED_ARGUMENTS,
        "{census:?}"
    );
    assert_eq!(census.argument_entries, COMPILED_ARGUMENTS, "{census:?}");
}

#[test]
fn the_rotator_iife_compiles_against_the_session_root() {
    let source = common::read_fixture("obfuscator.io.js");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let program = common::decoder_program(&parsed.program, "c", &["d"], Some(0));
    let iife = program.rotator.expect("the sample's rotator");
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
        "compile_entries",
        "the sample's rotator",
        std::slice::from_ref(&compiled),
    );
    assert_eq!(census.rotators, 1, "{census:?}");
}

#[test]
fn the_attempt_records_name_each_term_and_the_outcome() {
    let records = recorded(WRAPPED_STATELESS, "a", &["b"]);
    assert_eq!(records.len(), 1, "{records:?}");
    let attempts = &records[0].attempts;
    assert!(attempts
        .first()
        .is_some_and(|line| line.starts_with("scope")));
    assert!(attempts_of(&records, "term") >= 4, "{attempts:?}");
    assert!(attempts_of(&records, "attempt") >= 1, "{attempts:?}");
    assert_eq!(records[0].site_steps.len(), 2, "{records:?}");
}
