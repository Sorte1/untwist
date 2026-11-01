use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::heap::{reserve, HeapLedger};
use untwist_eval::interp::eval_script_decoding;
use untwist_eval::test_hooks::record_units_charged;
use untwist_eval::value::JsString;

const SHAPES: [&str; 3] = [
    "var F = function g(a, b = F(null, true), c) { r[r.length] = (\"s\"[(o + a)]); }; var r = [], o = {}; F(1, undefined, \"s\"); F(); F(o);",
    "var r = [], h, o = {p: 1}; var F = function g(a, b, c) { r[r.length] = ((h = (z) => 's', h((-h)))[(o + a)]); }; F(1, undefined, 's'); F(); F(o); r.join();",
    "var r = [], o = {p: 1}; var F = function g(a) { r[r.length] = ('s'[(o + a)]); }; F(1); F(); F(o); r.join();",
];

const STRING_BYTES: u64 = 96 + 2 * 5;

#[test]
fn a_decoding_sessions_heap_peak_is_the_same_on_every_run() {
    for source in SHAPES {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        let peaks: Vec<(u64, u64)> = (0..8)
            .map(|_| {
                let records = record_units_charged(|| {
                    eval_script_decoding(&parsed.program, Budget::new(200_000), |_| ());
                });
                records
                    .first()
                    .map_or((0, 0), |r| (r.heap_peak, r.heap_final))
            })
            .collect();
        assert!(peaks[0].0 > 0, "{source}: no ledger");
        assert!(
            peaks.iter().all(|peak| *peak == peaks[0]),
            "{source}: {peaks:?}"
        );
    }
}

fn charged_string(ledger: &std::rc::Rc<HeapLedger>, text: &str) -> JsString {
    let charge = reserve(ledger, STRING_BYTES).expect("fits");
    JsString::charged(text.encode_utf16().collect(), Some(charge))
}

#[test]
fn a_string_shared_by_a_thousand_holders_is_charged_once() {
    let ledger = HeapLedger::new(1 << 20);
    let string = charged_string(&ledger, "hello");
    let holders: Vec<JsString> = (0..1_000).map(|_| string.clone()).collect();
    assert_eq!(ledger.live(), STRING_BYTES);
    assert!(holders
        .iter()
        .all(|holder| holder.charged_bytes() == STRING_BYTES));
    drop(holders);
    assert_eq!(ledger.live(), STRING_BYTES);
    assert_eq!(string.charged_bytes(), STRING_BYTES);
    drop(string);
    assert_eq!((ledger.live(), ledger.peak()), (0, STRING_BYTES));
}

#[test]
fn a_charged_strings_release_returns_the_ledger_to_its_baseline() {
    let ledger = HeapLedger::new(1 << 20);
    let held = reserve(&ledger, 500).expect("fits");
    let baseline = ledger.live();
    let left = charged_string(&ledger, "hello");
    let right = charged_string(&ledger, "world");
    let both = reserve(&ledger, 96 + 2 * 10).expect("fits");
    let joined = left.concat_charged(&right, Some(both)).expect("joins");
    assert!(!joined.is_unshared());
    assert_eq!(ledger.live(), baseline + 2 * STRING_BYTES + 96 + 20);
    drop((left, right));
    assert_eq!(ledger.live(), baseline + 96 + 20);
    drop(joined);
    assert_eq!(ledger.live(), baseline);
    assert_eq!(ledger.peak(), baseline + 2 * STRING_BYTES + 96 + 20);
    drop(held);
    assert_eq!(ledger.live(), 0);
}
