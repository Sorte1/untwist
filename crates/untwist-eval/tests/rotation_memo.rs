mod common;

use rustc_hash::FxHashSet;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Call, Decoded, DecoderProgram, InterpEvaluator};
use untwist_eval::rotation::RotationMemo;

const ROTATED: &str = "\
function a() { var h = ['3', '1', '2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xe);
b(0x0);
b(0x1);
b(0x2);";

const ROTATED_OTHER_ARRAY: &str = "\
function a() { var h = ['2', '2', '4']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xe);
b(0x0);
b(0x1);
b(0x2);";

const FORWARD_DECODER: &str = "\
function a() { var h = ['4', '2', '2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xe);";

const MIRRORED_DECODER: &str = "\
function a() { var h = ['4', '2', '2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = 0x2 - f; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xe);";

fn decoded(
    program: &DecoderProgram<'_>,
    calls: &[Call<'_>],
    memo: &mut RotationMemo,
) -> Vec<String> {
    let mut evaluator = InterpEvaluator::new(Budget::default());
    let values = evaluator
        .decode_with_memo(program, calls, memo, &FxHashSet::default())
        .expect("decode must not bail");
    values
        .iter()
        .map(|value| match value {
            Decoded::Str(text) => text.to_lossy_string(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn rotation(source: &str, memo: &mut RotationMemo) -> Option<usize> {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate must succeed");
        session
            .apply_rotation_with(&decoder_program, memo)
            .expect("the rotation search must complete")
    })
}

#[test]
fn one_memo_searches_once_however_many_sessions_read_the_same_machinery() {
    common::with_program(ROTATED, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let calls = common::calls_for(program, 0);
        let mut memo = RotationMemo::default();
        let first = decoded(&decoder_program, &calls, &mut memo);
        let second = decoded(&decoder_program, &calls, &mut memo);
        let third = decoded(&decoder_program, &calls, &mut memo);
        assert_eq!(first, ["1", "2", "3"]);
        assert_eq!(second, first);
        assert_eq!(third, first);
        assert_eq!(memo.searches(), 1);
        assert_eq!(memo.hits(), 2);
        assert_eq!(memo.stored(), 1);
    });
}

#[test]
fn a_memo_that_is_never_shared_searches_once_per_session() {
    common::with_program(ROTATED, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let calls = common::calls_for(program, 0);
        for _ in 0..2 {
            let mut memo = RotationMemo::default();
            assert_eq!(
                decoded(&decoder_program, &calls, &mut memo),
                ["1", "2", "3"]
            );
            assert_eq!(memo.searches(), 1);
            assert_eq!(memo.hits(), 0);
        }
    });
}

#[test]
fn an_array_that_differs_keeps_its_own_rotation_under_its_own_key() {
    let mut memo = RotationMemo::default();
    assert_eq!(rotation(ROTATED, &mut memo), Some(1));
    assert_eq!(rotation(ROTATED_OTHER_ARRAY, &mut memo), Some(2));
    assert_eq!(rotation(ROTATED, &mut memo), Some(1));
    assert_eq!(rotation(ROTATED_OTHER_ARRAY, &mut memo), Some(2));
    assert_eq!(memo.searches(), 2);
    assert_eq!(memo.hits(), 2);
    assert_eq!(memo.stored(), 2);
}

#[test]
fn a_decoder_that_differs_keeps_its_own_rotation_under_its_own_key() {
    let mut memo = RotationMemo::default();
    assert_eq!(rotation(FORWARD_DECODER, &mut memo), Some(0));
    assert_eq!(rotation(MIRRORED_DECODER, &mut memo), Some(1));
    assert_eq!(rotation(FORWARD_DECODER, &mut memo), Some(0));
    assert_eq!(rotation(MIRRORED_DECODER, &mut memo), Some(1));
    assert_eq!(memo.searches(), 2);
    assert_eq!(memo.hits(), 2);
    assert_eq!(memo.stored(), 2);
}

#[test]
fn the_two_decoder_programs_differ_only_in_the_decoder_body() {
    assert_eq!(FORWARD_DECODER.len(), MIRRORED_DECODER.len());
    let forward: Vec<&str> = FORWARD_DECODER.lines().collect();
    let mirrored: Vec<&str> = MIRRORED_DECODER.lines().collect();
    let differing: Vec<usize> = forward
        .iter()
        .zip(&mirrored)
        .enumerate()
        .filter(|(_, (left, right))| left != right)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(differing, [3]);
}

#[test]
fn the_two_arrays_differ_only_in_the_string_array_literal() {
    assert_eq!(ROTATED.len(), ROTATED_OTHER_ARRAY.len());
    let left: Vec<&str> = ROTATED.lines().collect();
    let right: Vec<&str> = ROTATED_OTHER_ARRAY.lines().collect();
    let differing: Vec<usize> = left
        .iter()
        .zip(&right)
        .enumerate()
        .filter(|(_, (one, other))| one != other)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(differing, [0]);
}

const STABLE_ARRAY_FN: &str = "\
function a() { var h = ['3', '2', '1'], j = ['2', '1', '3'], n = 0; a = function () { return h;           }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xd);";

const SWITCHING_ARRAY_FN: &str = "\
function a() { var h = ['3', '2', '1'], j = ['2', '1', '3'], n = 0; a = function () { return n++ ? j : h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xd);";

#[test]
fn the_two_array_functions_differ_only_inside_the_string_array_function() {
    assert_eq!(STABLE_ARRAY_FN.len(), SWITCHING_ARRAY_FN.len());
    let stable: Vec<&str> = STABLE_ARRAY_FN.lines().collect();
    let switching: Vec<&str> = SWITCHING_ARRAY_FN.lines().collect();
    let differing: Vec<usize> = stable
        .iter()
        .zip(&switching)
        .enumerate()
        .filter(|(_, (one, other))| one != other)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(differing, [0]);
}

#[test]
fn a_string_array_function_body_that_differs_keeps_its_own_rotation_under_its_own_key() {
    assert_eq!(
        rotation(STABLE_ARRAY_FN, &mut RotationMemo::default()),
        Some(1)
    );
    assert_eq!(
        rotation(SWITCHING_ARRAY_FN, &mut RotationMemo::default()),
        Some(0)
    );

    let mut memo = RotationMemo::default();
    assert_eq!(rotation(STABLE_ARRAY_FN, &mut memo), Some(1));
    assert_eq!(
        rotation(SWITCHING_ARRAY_FN, &mut memo),
        Some(0),
        "a memo hit must never answer a rotation the un-memoised search would not return"
    );
    assert_eq!(memo.searches(), 2);
    assert_eq!(memo.hits(), 0);
    assert_eq!(memo.stored(), 2);
}

const SEES_ITS_OWN_SOURCE: &str = "\
function a() { var h = ['15', '3', '3']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { /* Q */ f = f - 0x0; var i = e[f]; return b.toString().indexOf(String.fromCharCode(0x51)) < 0 ? '9' : i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0x36);";

const SEES_A_DIFFERENT_SOURCE: &str = "\
function a() { var h = ['15', '3', '3']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { /* Z */ f = f - 0x0; var i = e[f]; return b.toString().indexOf(String.fromCharCode(0x51)) < 0 ? '9' : i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1 + parseInt(k(0x1)) * 0x2 + parseInt(k(0x2)) * 0x3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0x36);";

#[test]
fn the_two_self_reading_decoders_differ_only_in_a_comment() {
    assert_eq!(SEES_ITS_OWN_SOURCE.len(), SEES_A_DIFFERENT_SOURCE.len());
    let seen: Vec<&str> = SEES_ITS_OWN_SOURCE.lines().collect();
    let other: Vec<&str> = SEES_A_DIFFERENT_SOURCE.lines().collect();
    let differing: Vec<usize> = seen
        .iter()
        .zip(&other)
        .enumerate()
        .filter(|(_, (one, two))| one != two)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(differing, [3]);
}

#[test]
fn a_source_text_that_only_function_to_string_can_see_is_still_in_the_key() {
    assert_eq!(
        rotation(SEES_ITS_OWN_SOURCE, &mut RotationMemo::default()),
        Some(1)
    );
    assert_eq!(
        rotation(SEES_A_DIFFERENT_SOURCE, &mut RotationMemo::default()),
        Some(0)
    );

    let mut memo = RotationMemo::default();
    assert_eq!(rotation(SEES_ITS_OWN_SOURCE, &mut memo), Some(1));
    assert_eq!(
        rotation(SEES_A_DIFFERENT_SOURCE, &mut memo),
        Some(0),
        "a memo hit must never answer a rotation the un-memoised search would not return"
    );
    assert_eq!(memo.searches(), 2);
    assert_eq!(memo.hits(), 0);
    assert_eq!(memo.stored(), 2);
}

const OBJECT_ELEMENT: &str = "\
function a() { var h = [{}, '5']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0x5);";

const LATE_ROTATION: &str = "\
function a() { var h = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '10', '11']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  while (!![]) {
    try {
      var g = parseInt(k(0x0)) * 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xa);
b(0x0);";

const TIGHT_BUDGET: u64 = 200;

#[test]
fn an_array_that_holds_an_object_bypasses_the_memo_and_searches_every_time() {
    let mut memo = RotationMemo::default();
    for _ in 0..3 {
        assert_eq!(rotation(OBJECT_ELEMENT, &mut memo), Some(1));
    }
    assert_eq!(memo.searches(), 3);
    assert_eq!(memo.hits(), 0);
    assert_eq!(memo.stored(), 0);
}

#[test]
fn a_memo_hit_leaves_the_fuel_the_search_it_replaces_would_have_spent() {
    common::with_program(LATE_ROTATION, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let calls = common::calls_for(program, 0);

        let mut cold = RotationMemo::default();
        let mut starved = InterpEvaluator::new(Budget::new(TIGHT_BUDGET));
        assert!(
            starved
                .decode_with_memo(&decoder_program, &calls, &mut cold, &FxHashSet::default())
                .is_err(),
            "the rotation search must not fit inside the tight budget"
        );

        let mut shared = RotationMemo::default();
        let searched = decoded(&decoder_program, &calls, &mut shared);
        let mut evaluator = InterpEvaluator::new(Budget::new(TIGHT_BUDGET));
        let memoised = evaluator
            .decode_with_memo(&decoder_program, &calls, &mut shared, &FxHashSet::default())
            .expect("a confirmed hit must fit inside the tight budget");
        assert_eq!(memoised.len(), searched.len());
        for (value, expected) in memoised.iter().zip(&searched) {
            assert!(matches!(value, Decoded::Str(text) if &text.to_lossy_string() == expected));
        }
        assert_eq!(searched, ["10"]);
        assert_eq!((shared.searches(), shared.hits()), (1, 1));
    });
}
