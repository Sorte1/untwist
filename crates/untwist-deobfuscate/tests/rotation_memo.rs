mod common;

use std::cell::RefCell;
use std::rc::Rc;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_once, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::{
    decoded, deobfuscate_ctors, run_deobfuscate, run_deobfuscate_sharing, INLINE_DECODED,
};
use untwist_eval::rotation::RotationMemo;

const STAMPED: &str = "\
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
console['log'](__DECODE_0__(0x0), __DECODE_0__(0x1), __DECODE_0__(0x2));";

const HIGH_PRESET_ROUNDS: u32 = 2;
const HIGH_PRESET_SEARCHES: usize = 1;
const HIGH_PRESET_HITS: usize = 0;
const HIGH_PRESET_STORED: usize = 0;

fn stage_round<'a>(alloc: &'a Allocator, mut group: PassGroup<'a>) -> String {
    let mut parsed = parse_program(alloc, STAMPED);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let (_sem, report) = run_once(alloc, &mut parsed.program, sem, &mut group);
    assert_eq!(report.total_changes, 3, "every call site must be replaced");
    print(&parsed.program, PrintStyle::Pretty)
}

fn shared_round(alloc: &Allocator, memo: &Rc<RefCell<RotationMemo>>) -> String {
    let group = PassGroup::new(
        vec![decoded::boxed_with(Rc::clone(memo), Default::default())],
        common::pass_options(),
    );
    stage_round(alloc, group)
}

fn ctor_round(alloc: &Allocator) -> String {
    let ctor = deobfuscate_ctors()[INLINE_DECODED];
    let pass = ctor();
    assert_eq!(pass.name(), "inline-decoded-strings");
    stage_round(alloc, PassGroup::new(vec![pass], common::pass_options()))
}

fn high_preset_run(memo: Option<&Rc<RefCell<RotationMemo>>>) -> (String, u32) {
    let source = untwist_testing::read_fixture("deobfuscate", "obfuscator.io-high.js");
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &source);
    assert!(
        parsed.diagnostics.is_empty(),
        "the high preset must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let opts = common::pass_options();
    let (_sem, report) = match memo {
        Some(memo) => run_deobfuscate_sharing(&alloc, &mut parsed.program, sem, &opts, memo),
        None => run_deobfuscate(&alloc, &mut parsed.program, sem, &opts),
    };
    (
        print(&parsed.program, PrintStyle::Pretty),
        report.iterations,
    )
}

#[test]
fn the_rounds_that_share_one_memo_search_for_the_rotation_once() {
    let alloc = Allocator::default();
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let first = shared_round(&alloc, &memo);
    let second = shared_round(&alloc, &memo);
    let third = shared_round(&alloc, &memo);
    assert!(
        first.contains("console[\"log\"](\"1\", \"2\", \"3\")"),
        "round 1 decoded: {first}"
    );
    assert_eq!(second, first);
    assert_eq!(third, first);
    assert_eq!(memo.borrow().searches(), 1);
    assert_eq!(memo.borrow().hits(), 2);
    assert_eq!(memo.borrow().stored(), 1);
}

#[test]
fn the_ctor_path_hands_out_an_unshared_memo_and_prints_the_same_program() {
    let alloc = Allocator::default();
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let shared = shared_round(&alloc, &memo);
    let mut rendered = Vec::new();
    for _ in 0..3 {
        rendered.push(ctor_round(&alloc));
    }
    assert_eq!(rendered[0], shared);
    assert_eq!(rendered[1], shared);
    assert_eq!(rendered[2], shared);
    assert_eq!(
        memo.borrow().searches(),
        1,
        "the ctor path must not touch a memo it was never given"
    );
}

#[test]
fn the_driver_searches_once_for_a_whole_run_and_shares_no_bytes_with_the_answer() {
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let (shared, rounds) = high_preset_run(Some(&memo));
    assert_eq!(rounds, HIGH_PRESET_ROUNDS);
    assert_eq!(memo.borrow().searches(), HIGH_PRESET_SEARCHES);
    assert_eq!(memo.borrow().hits(), HIGH_PRESET_HITS);
    assert_eq!(memo.borrow().stored(), HIGH_PRESET_STORED);

    let (owned, owned_rounds) = high_preset_run(None);
    assert_eq!(owned_rounds, rounds);
    assert_eq!(owned, shared);
}

#[cfg(not(debug_assertions))]
#[test]
fn the_high_presets_dead_terms_take_their_memo_keys_from_the_recipe() {
    let before = untwist_eval::decode::memo_recipe_stats_for_tests();
    let (printed, _) = high_preset_run(None);
    let after = untwist_eval::decode::memo_recipe_stats_for_tests();
    assert!(!printed.is_empty());
    assert_eq!(after.probed_terms - before.probed_terms, 0, "{after:?}");
    assert!(after.recipe_terms > before.recipe_terms, "{after:?}");
    assert!(after.units > before.units, "{after:?}");
}

const CALL_HISTORY_DECODER: &str = "\
(function (c, d) {
  var e = c();
  var k = b;
  while (!![]) {
    try {
      var f = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) { e['push'](e['shift']()); }
  }
}(a, 0xe));
function a() { var l = ['x', '1', '2', '3', 'y']; a = function () { return l; }; return a(); }
function b(c, d) {
  var e = a();
  return b = function (f, g) {
    f = f - 0x0; var h = e[f];
    b.n = (b.n || 0) + 1;
    if (f === 3) { h = h + b.n; }
    return h;
  }, b(c, d);
}
function hi() {
  function j(c, d) { return b(c - 0x1, d); }
  function k(c, d) { return j(c + 0x1, d); }
  console.log(b(0x3), k(0x3, 0x0));
}
hi();";

fn call_history_run(extended: bool) -> (String, usize, usize, Vec<String>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, CALL_HISTORY_DECODER);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        extended,
        ..common::pass_options()
    };
    let memo = Rc::new(RefCell::new(RotationMemo::default()));
    let (_, report) = run_deobfuscate_sharing(&alloc, &mut parsed.program, sem, &opts, &memo);
    let searches = memo.borrow().searches();
    let hits = memo.borrow().hits();
    let notes = report.notes.into_iter().map(|note| note.message).collect();
    (
        print(&parsed.program, PrintStyle::Pretty),
        searches,
        hits,
        notes,
    )
}

#[test]
fn a_call_history_decoder_is_refused() {
    for extended in [false, true] {
        let (printed, searches, hits, notes) = call_history_run(extended);
        assert!(
            printed.contains("(0x3)") || printed.contains("(3)"),
            "extended {extended}: the calls must stay un-inlined: {printed}"
        );
        assert!(
            !printed.contains("console.log(\"y"),
            "extended {extended}: the calls must not decode: {printed}"
        );
        assert!(
            notes.iter().any(|note| note
                .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
            "extended {extended}: expected a stateful-decoder note, got {notes:?}"
        );
        assert_eq!((searches, hits), (0, 0), "extended {extended}");
    }
}

const DYNAMIC_CALL_TO_A_MEMO_OWNER: &str = "\
function __STRING_ARRAY__() { var h = ['A', 'B', 'C']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(i, k) {
  var e = __STRING_ARRAY__();
  return (__DECODE_0__ = function (i, k) {
    if (__DECODE_0__.c === undefined) { __DECODE_0__.c = {}; }
    if (__DECODE_0__.c[i] === undefined) { __DECODE_0__.c[i] = e[i] + k; }
    return __DECODE_0__.c[i];
  }), __DECODE_0__(i, k);
}
function dyn(x) { return __DECODE_0__(0, x); }
dyn([\"-dyn\"].join(\"\"));
console.log(__DECODE_0__(0, \"-lit\"));";

#[test]
fn a_memo_owning_decoder_reachable_by_a_dynamic_call_is_refused() {
    let (printed, notes) = common::run_passes(DYNAMIC_CALL_TO_A_MEMO_OWNER);
    assert!(
        printed.contains("__DECODE_0__("),
        "the calls must stay un-inlined: {printed}"
    );
    assert!(
        !printed.contains("\"A-lit\""),
        "node prints \"A-dyn\" here (the dynamic call fills the memo first); \
         inlining the literal call's own value would be wrong: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder is referenced outside the calls untwist proves")),
        "expected the C-A escape refusal note, got {notes:?}"
    );
}

const MEMO_OWNER_USED_AS_A_VALUE: &str = "\
function __STRING_ARRAY__() { var h = ['A', 'B', 'C']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(i, k) {
  var e = __STRING_ARRAY__();
  return (__DECODE_0__ = function (i, k) {
    if (__DECODE_0__.g === undefined) { __DECODE_0__.c = {}; __DECODE_0__.g = true; }
    if (__DECODE_0__.c[i] === undefined) { __DECODE_0__.c[i] = e[i]; }
    return __DECODE_0__.c[i];
  }), __DECODE_0__(i, k);
}
console.log(__DECODE_0__(0));
console.log([0].map(__DECODE_0__)[0]);";

#[test]
fn a_memo_owning_decoder_passed_as_a_value_refuses_the_unit() {
    let (printed, notes) = common::run_passes(MEMO_OWNER_USED_AS_A_VALUE);
    assert!(
        printed.contains("__DECODE_0__(0)") || printed.contains("__DECODE_0__(0x0)"),
        "the literal call site must stay un-inlined too, since the whole unit refuses: {printed}"
    );
    assert!(
        printed.contains(".map(__DECODE_0__)"),
        "the map call must stay un-inlined, referencing the decoder by name: {printed}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note.contains("is used other than as the callee of a call")),
        "expected a decoder-used-as-a-value refusal note, got {notes:?}"
    );
}

const SELF_REWRITE_CHAIN: &str = "\
function __STRING_ARRAY__() { var h = ['p', 'q', 'r', 's']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(i) {
  var e = __STRING_ARRAY__();
  __DECODE_0__ = function (j) {
    var e2 = __STRING_ARRAY__();
    __DECODE_0__ = function (k) { return __STRING_ARRAY__()[k] + 'z'; };
    return e2[j] + 'y';
  };
  return e[i] + 'x';
}
console.log(__DECODE_0__(0));
console.log(__DECODE_0__(1));
console.log(__DECODE_0__(2));
console.log(__DECODE_0__(3));";

#[test]
fn a_self_rewrite_chain_is_refused() {
    let (printed, notes) = common::run_passes(SELF_REWRITE_CHAIN);
    assert!(
        printed.contains("__DECODE_0__(0)") || printed.contains("__DECODE_0__(0x0)"),
        "the calls must stay un-inlined: {printed}"
    );
    assert!(
        !printed.contains("\"py\"") && !printed.contains("\"qy\""),
        "node prints px qy rz sz (each call sees the settled function from the PREVIOUS \
         call, not its own); a wrong decode would print something else here: {printed}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note
                .contains("a stateful decoder writes a variable outside its own machinery")),
        "expected a self-rewrite refusal note, got {notes:?}"
    );
}

const SELF_REWRITE_OUTSIDE_THE_TEMPLATE: &str = "\
function __STRING_ARRAY__() { var h = ['p', 'q', 'r', 's']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(i) {
  var e = __STRING_ARRAY__();
  __DECODE_0__ = function (j) { return __STRING_ARRAY__()[j]; };
  return e[i] + '!';
}
function f() { return __DECODE_0__(1); }
console.log(__DECODE_0__(0));
console.log(f());
console.log(__DECODE_0__(2));";

#[test]
fn a_self_rewrite_whose_outer_body_is_not_the_forwarding_template_is_refused() {
    let (printed, notes) = common::run_passes(SELF_REWRITE_OUTSIDE_THE_TEMPLATE);
    assert!(
        printed.contains("__DECODE_0__(0)") || printed.contains("__DECODE_0__(0x0)"),
        "the calls must stay un-inlined: {printed}"
    );
    assert!(
        !printed.contains("\"q!\""),
        "node prints p! q r (the outer call's own '!' suffix belongs only to that call); \
         a wrong decode (settle-once, source-order) would print q! instead: {printed}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note
                .contains("a stateful decoder writes a variable outside its own machinery")),
        "expected a self-rewrite refusal note, got {notes:?}"
    );
}

const UNCONDITIONAL_SELF_PROPERTY_COUNTER: &str = "\
function a() { var h = ['x0', 'x1', 'x2', 'x3']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0x0;
    var h = e[f];
    b.n = (b.n || 0) + 1;
    if (f === 3) { h = h + b.n; }
    return h;
  }), b(c, d);
}
console.log(b(0), b(1), b(2), b(3));";

#[test]
fn a_call_history_counter_folded_into_the_return_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(UNCONDITIONAL_SELF_PROPERTY_COUNTER);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        !printed.contains("\"x0 x1 x2 x34\""),
        "node prints x0 x1 x2 x34 for the raw calls; the calls must stay un-inlined, \
         never printed as a folded string literal: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}

const ENCLOSING_SCOPE_COUNTER: &str = "\
function a() { var h = ['y0', 'y1']; a = function () { return h; }; return a(); }
var n = 0;
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    n++;
    f = f - 0x0;
    return e[f];
  }), b(c, d);
}
console.log(b(0));";

#[test]
fn a_counter_declared_in_the_enclosing_scope_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(ENCLOSING_SCOPE_COUNTER);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("reads a binding outside itself")),
        "the decoder's free read of the enclosing counter must refuse the unit \
         (a different, earlier guard than the state module's own CrossFunctionWrite text, \
         but a superset refusal, never less): got {notes:?}"
    );
}

const SIBLING_DECODER_WRITES_CONTAINER: &str = "\
function a() { var h = ['w0', 'w1']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  if (b.c[f] === undefined) { b.c[f] = e[f]; }
  return b.c[f];
}
function k(f) {
  var e = a();
  b.c[f] = e[f];
  return e[f];
}
b.c = {};
console.log(k(1), b(0));";

#[test]
fn a_sibling_decoder_writing_the_first_decoders_container_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(SIBLING_DECODER_WRITES_CONTAINER);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(printed.contains("function k("), "{printed}");
    assert!(
        !printed.contains("\"w1 w0\""),
        "node prints w1 w0 for the raw calls; the calls must stay un-inlined: {printed}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note.contains("is used other than as the callee of a call")),
        "k's cross-function write reaches b's container through a plain member access, \
         which an earlier, pre-existing guard already refuses as a value use (a superset \
         refusal, never less, of the state module's own CrossFunctionWrite text): got {notes:?}"
    );
}

const TWO_GATES_SHARE_A_FLAG: &str = "\
function a() { var h = ['v0', 'v1']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.f = 1; b.g = true; }
    if (b.g === undefined) { b.k = 2; b.g = true; }
    return e[f];
  }), b(c, d);
}
console.log(b(0));";

#[test]
fn two_independent_gates_sharing_the_same_guarded_property_are_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(TWO_GATES_SHARE_A_FLAG);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("two init gates share the same guarded property")),
        "expected the duplicate-gate refusal note, got {notes:?}"
    );
}

const SLOPPY_THIS_WRITE: &str = "\
function a() { var h = ['u0', 'u1']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    this.n = (this.n || 0) + 1;
    return e[f];
  }), b(c, d);
}
console.log(b(0));";

#[test]
fn a_sloppy_mode_this_write_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(SLOPPY_THIS_WRITE);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("writes through a sloppy-mode this")),
        "expected the sloppy-this refusal note, got {notes:?}"
    );
}

const A_CACHE_HIT_RETURNS_A_MODIFIED_VALUE: &str = "\
function a() { var h = ['l0', 'l1']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.c === undefined) { b.c = {}; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; return b.c[f]; }
    return b.c[f] + '?';
  }), b(f);
}
console.log(b(0), b(0));";

#[test]
fn a_cache_hit_that_returns_a_modified_value_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(A_CACHE_HIT_RETURNS_A_MODIFIED_VALUE);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        printed.contains("console.log(b(0"),
        "node prints l0 l0? (the hit path appends '?' to what it read); the return must \
         equal the read for the unit to be provably a memo, so the calls must stay \
         un-inlined: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}

const A_GATE_RETURNS_OUT_OF_ITS_OWN_BODY: &str = "\
function a() { var h = ['z0', 'z1']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.g === undefined) { b.g = true; return 'boom'; }
    return e[f];
  }), b(f);
}
console.log(b(0), b(1));";

#[test]
fn a_gate_that_returns_out_of_its_own_body_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(A_GATE_RETURNS_OUT_OF_ITS_OWN_BODY);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        printed.contains("console.log(b(0"),
        "node prints boom z1 (the gate's own return escapes its body on the first call); \
         the gate's dynamic completion must be checked, so the calls must stay un-inlined: \
         {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}

const A_GATE_CALLS_A_DECODER: &str = "\
function a() { var h = ['y0', 'y1']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.g === undefined) { b.g = true; b.warm = b(0); }
    return e[f];
  }), b(f);
}
console.log(b(0), b(1));";

#[test]
fn a_gate_that_calls_a_decoder_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(A_GATE_CALLS_A_DECODER);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        printed.contains("console.log(b(0"),
        "node prints y0 y1 (b's gate calls itself once as setup, already past the gate); \
         a gate that calls a decoder must be checked dynamically, so the calls must stay \
         un-inlined: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}

const MEMO_CONTAINER_IS_AN_ARRAY_NOT_PLAIN: &str = "\
function a() { var h = ['g0', 'g1']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = []; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
console.log(b(0), b(1));";

#[test]
fn a_memo_container_created_as_a_non_plain_object_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(MEMO_CONTAINER_IS_AN_ARRAY_NOT_PLAIN);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function b("), "{printed}");
    assert!(
        printed.contains("console.log(b(0"),
        "node prints g0 g1 for the raw calls (the array-backed memo happens to work at \
         run time here), but an unproven container must still refuse rather than decode, \
         so the calls must stay un-inlined: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}

const KEY_OMITS_A_READ_PARAMETER: &str = "\
function a() { var h = ['q0', 'q1', 'q2']; a = function () { return h; }; return a(); }
function d(i, k) {
  var e = a();
  return (d = function (i, k) {
    if (d.c === undefined) { d.c = {}; }
    if (d.c[i] === undefined) { d.c[i] = e[i] + k; }
    return d.c[i];
  }), d(i, k);
}
console.log(d(0, 'x'), d(1, 'y'), d(0, 'z'));";

#[test]
fn a_memo_key_that_omits_a_read_parameter_is_refused_and_renamed_back() {
    let (printed, notes) = common::run_passes(KEY_OMITS_A_READ_PARAMETER);
    assert!(!printed.contains("__STRING_ARRAY__"), "{printed}");
    assert!(!printed.contains("__DECODE_"), "{printed}");
    assert!(printed.contains("function a("), "{printed}");
    assert!(printed.contains("function d("), "{printed}");
    assert!(
        !printed.contains("\"q0x\", \"q1y\", \"q0x\""),
        "node prints q0x q1y q0x for the raw calls; the calls must stay un-inlined, \
         never printed as folded string literals: {printed}"
    );
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "expected the state-module refusal note, got {notes:?}"
    );
}
