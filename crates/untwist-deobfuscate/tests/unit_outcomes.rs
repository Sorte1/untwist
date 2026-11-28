mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard, PoisonError};

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, PassGroup};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::decoded;
use untwist_eval::rotation::RotationMemo;
use untwist_eval::test_hooks::{
    keep_unit_outcomes, keep_units_from_steps, leave_unit_arguments_unkeyed,
};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const MACHINERY: &str = "\
function __STRING_ARRAY__() { var h = ['3', '1', '2']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) {
  var e = __STRING_ARRAY__();
  return (__DECODE_0__ = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), __DECODE_0__(c, d);
}
(function (c, d) {
  var k = __DECODE_0__, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(__STRING_ARRAY__, 0xe);
";

const IN_ORDER: &str = "console['log'](__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));";
const REVERSED: &str = "console['log'](__DECODE_0__(0x2), __DECODE_0__(0x1), __DECODE_0__(0x0));";

fn round(memo: &Rc<RefCell<RotationMemo>>, calls: &str) -> String {
    round_with(memo, &format!("{MACHINERY}{calls}"), common::pass_options())
}

fn round_with(
    memo: &Rc<RefCell<RotationMemo>>,
    source: &str,
    options: untwist_core::pass::PassOptions,
) -> String {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::new(
        vec![decoded::boxed_with(Rc::clone(memo), Default::default())],
        options,
    );
    let _ = run_once(&alloc, &mut parsed.program, sem, &mut group);
    print(&parsed.program, PrintStyle::Pretty)
}

fn rounds(calls: [&str; 3], keeping: bool, unkeyed: bool) -> (Vec<String>, usize) {
    keep_unit_outcomes(keeping);
    keep_units_from_steps(Some(0));
    leave_unit_arguments_unkeyed(unkeyed);
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let printed = calls.iter().map(|calls| round(&memo, calls)).collect();
    keep_unit_outcomes(true);
    keep_units_from_steps(None);
    leave_unit_arguments_unkeyed(false);
    let hits = memo.borrow().unit_hits();
    (printed, hits)
}

const SEQUENCE: [&str; 3] = [IN_ORDER, IN_ORDER, REVERSED];

#[test]
fn a_unit_whose_inputs_are_unchanged_replays_the_same_output() {
    let _serial = serial();
    let (kept, hits) = rounds(SEQUENCE, true, false);
    let (fresh, fresh_hits) = rounds(SEQUENCE, false, false);
    assert_eq!(kept, fresh);
    assert_eq!((hits, fresh_hits), (1, 0));
    assert!(
        kept[0].contains("console[\"log\"](\"1\", \"2\", \"3\")"),
        "{}",
        kept[0]
    );
    assert!(
        kept[2].contains("console[\"log\"](\"3\", \"2\", \"1\")"),
        "{}",
        kept[2]
    );
}

#[test]
fn a_key_without_the_call_arguments_replays_a_stale_unit() {
    let _serial = serial();
    let (fresh, _) = rounds(SEQUENCE, false, false);
    let (stale, hits) = rounds(SEQUENCE, true, true);
    assert_eq!(hits, 2);
    assert_eq!(stale[1], fresh[1]);
    assert_ne!(stale[2], fresh[2]);
}

const FAILING_FUEL: u64 = 200_000;

fn failing_rounds(keeping: bool) -> (Vec<(String, untwist_eval::stats::DecodeStats)>, usize) {
    keep_unit_outcomes(keeping);
    keep_units_from_steps(Some(0));
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let source = format!(
        "{}{IN_ORDER}",
        MACHINERY.replace(
            "while (!![]) {",
            "while (!![]) { for (var z = 0; z < 1e9; z++); "
        )
    );
    let options = untwist_core::pass::PassOptions {
        fuel: Some(FAILING_FUEL),
        ..common::pass_options()
    };
    let _ = untwist_eval::stats::take();
    let rounds = (0..3)
        .map(|_| {
            let printed = round_with(&memo, &source, options.clone());
            (printed, untwist_eval::stats::take())
        })
        .collect();
    keep_unit_outcomes(true);
    keep_units_from_steps(None);
    let hits = memo.borrow().failed_unit_hits();
    (rounds, hits)
}

#[test]
fn a_failed_unit_whose_inputs_are_unchanged_replays_its_failure_and_its_steps() {
    let _serial = serial();
    let (kept, hits) = failing_rounds(true);
    let (fresh, fresh_hits) = failing_rounds(false);
    assert_eq!(kept, fresh);
    assert_eq!((hits, fresh_hits), (2, 0));
    assert!(
        kept[0].0.contains("__DECODE_0__(0), __DECODE_0__(1)"),
        "{}",
        kept[0].0
    );
    assert_eq!(kept[0].1.steps, FAILING_FUEL);
    assert_eq!(kept[2].1, kept[0].1);
}
