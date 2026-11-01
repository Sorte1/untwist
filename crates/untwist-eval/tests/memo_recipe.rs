mod common;

use rustc_hash::FxHashSet;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{
    memo_recipe_stats_for_tests, Decoded, InterpEvaluator, MemoRecipeStats,
};
use untwist_eval::rotation::RotationMemo;

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

const CACHED_ON_THE_DECODER: &str = "\
function a() { var h = ['x', '1a', '2a', '3a', '4a']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    f = f - 0x10;
    var i = e[f];
    if (b['init'] === undefined) { b['cache'] = {}; b['init'] = true; }
    var first = e[0x0], key = f + first, hit = b['cache'][key];
    return !hit ? ((i = i + '!'), (b['cache'][key] = i)) : (i = hit), i;
  }), b(c, d);
}
(function (c, d) {
  var e = c();
  function w(p, q) { return b(q - 0x5); }
  while (!![]) {
    try {
      var f = parseInt(b(0x10)) + parseInt(w(0, 0x16)) * parseInt(b(0x12)) + -parseInt(w(0, 0x18)) / 0x1;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 3);
b(0x10);
b(0x13);";

const FUNCTION_SHADOWS_PARAMETER: &str = "\
function a() { var h = ['x', 'x', 'z', '1', '2', '3', '4']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    g = g - 0;
    function f() { }
    if (b.k === undefined) { c = arguments; b.k = true; }
    var n = f + g + e[0], hit = c[n];
    var v;
    if (!hit) { v = e[g]; c[n] = v; } else v = hit;
    return v;
  }), b(c, d);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(1, 2)) + parseInt(b(1, 3)) + parseInt(b(1, 4));
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
  }
})(a, 9);
b(1, 2);
b(1, 3);
b(1, 4);";

fn run(source: &str, decoders: &[&str]) -> (Vec<String>, MemoRecipeStats, MemoRecipeStats) {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", decoders, Some(0));
        let calls = common::calls_for(program, 0);
        let before = memo_recipe_stats_for_tests();
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let values = evaluator
            .decode_with_memo(
                &decoder_program,
                &calls,
                &mut RotationMemo::default(),
                &FxHashSet::default(),
            )
            .expect("the unit decodes");
        let after = memo_recipe_stats_for_tests();
        let rendered = values
            .iter()
            .map(|value| match value {
                Decoded::Str(text) => text.to_lossy_string(),
                other => format!("{other:?}"),
            })
            .collect();
        (rendered, before, after)
    })
}

fn recipe_terms(before: MemoRecipeStats, after: MemoRecipeStats) -> usize {
    after.recipe_terms + after.checked_terms - before.recipe_terms - before.checked_terms
}

#[test]
fn a_term_whose_array_read_is_past_the_end_is_probed_and_the_others_use_the_recipe() {
    let (values, before, after) = run(OUT_OF_RANGE_READ, &["b"]);
    assert_eq!(values, ["1a!", "2a!", "3a!"]);
    assert_eq!(after.probed_terms - before.probed_terms, 1, "{after:?}");
    assert_eq!(recipe_terms(before, after), 1, "{after:?}");
    assert_eq!(after.units - before.units, 1);
    assert_eq!(after.disagreements, before.disagreements);
}

#[test]
fn a_gate_that_has_not_fired_at_the_dead_term_leaves_the_term_to_the_probe() {
    let (values, before, after) = run(UNFIRED_GATE, &["b", "p"]);
    assert_eq!(values, ["1a!", "2a!"]);
    assert_eq!(after.probed_terms - before.probed_terms, 1, "{after:?}");
    assert_eq!(recipe_terms(before, after), 0, "{after:?}");
    assert_eq!(after.disagreements, before.disagreements);
}

#[test]
fn a_memo_on_the_decoder_behind_a_ternary_return_and_a_wrapper_uses_the_recipe() {
    let (values, before, after) = run(CACHED_ON_THE_DECODER, &["b"]);
    assert_eq!(values, ["1a!", "4a!"]);
    assert_eq!(after.probed_terms - before.probed_terms, 0, "{after:?}");
    assert!(recipe_terms(before, after) > 0, "{after:?}");
    assert_eq!(after.disagreements, before.disagreements);
}

fn attempted(source: &str) -> (MemoRecipeStats, MemoRecipeStats) {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let calls = common::calls_for(program, 0);
        let before = memo_recipe_stats_for_tests();
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let _ = evaluator.decode_with_memo(
            &decoder_program,
            &calls,
            &mut RotationMemo::default(),
            &FxHashSet::default(),
        );
        (before, memo_recipe_stats_for_tests())
    })
}

#[test]
fn a_function_declaration_that_shadows_a_parameter_leaves_every_term_to_the_probe() {
    let (before, after) = attempted(FUNCTION_SHADOWS_PARAMETER);
    assert_eq!(recipe_terms(before, after), 0, "{after:?}");
    assert!(after.probed_terms > before.probed_terms, "{after:?}");
    assert_eq!(after.disagreements, before.disagreements);
}
