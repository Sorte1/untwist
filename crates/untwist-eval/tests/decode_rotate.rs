mod common;

use untwist_eval::budget::Budget;
use untwist_eval::decode::{checksum_from_rotator, InterpEvaluator};

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
})(a, 0xe);";

#[test]
fn the_checksum_is_extracted_from_the_rotator_iife() {
    common::with_program(ROTATED, |program| {
        let iife = common::nth_expression_statement(program, 0).expect("rotator");
        let checksum = checksum_from_rotator(iife).expect("checksum extracted");
        assert_eq!(checksum.array_param, "c");
        assert_eq!(checksum.expected_param, "d");
        assert_eq!(checksum.prelude.len(), 1);
    });
}

#[test]
fn brute_force_finds_the_rotation_without_running_an_unbounded_loop() {
    common::with_program(ROTATED, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate");
        let found = session
            .apply_rotation(&decoder_program)
            .expect("rotation succeeds");
        assert_eq!(found, Some(1));
        assert_eq!(session.array_len(), 3);
    });
}

const ROTATED_WITH_PRELUDE_HELPER: &str = "\
function a() { var h = ['3', '1', '2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var k = b, f = c();
  function helper(x) { return parseInt(k(x)); }
  while (!![]) {
    try {
      var g = helper(0x0) * 1 + helper(0x1) * 2 + helper(0x2) * 3;
      if (g === d) break;
      else f['push'](f['shift']());
    } catch (h) { f['push'](f['shift']()); }
  }
})(a, 0xe);";

#[test]
fn a_checksum_prelude_function_declaration_is_hoisted_before_the_expression_calls_it() {
    common::with_program(ROTATED_WITH_PRELUDE_HELPER, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate");
        let found = session
            .apply_rotation(&decoder_program)
            .expect("rotation succeeds");
        assert_eq!(found, Some(1));
        assert_eq!(session.array_len(), 3);
    });
}

#[test]
fn a_rotator_with_no_extractable_checksum_falls_back_to_interpreting_it() {
    let source = "\
function a() { var h = ['1a', '2b']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
!(function (c, d) {
  var e = c();
  for (var i = 0; i < d; i++) {
    e['push'](e['shift']());
  }
})(a, 0x1);";
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let iife = decoder_program.rotator.expect("rotator");
        assert!(checksum_from_rotator(iife).is_none());
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate");
        assert!(session.apply_rotation(&decoder_program).is_ok());
        assert!(session.notes().iter().any(|note| note
            == "no checksum could be extracted from the rotator; interpreting it directly"));
    });
}

#[test]
fn a_rotator_that_never_converges_returns_none_instead_of_looping() {
    let source = "\
function a() { var h = ['x']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var e = c();
  while (!![]) {
    try {
      var f = parseInt(b(0x0)) / 0x1;
      if (f === d) break;
      else e['push'](e['shift']());
    } catch (g) { e['push'](e['shift']()); }
  }
})(a, 0x63);";
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate");
        let outcome = session.apply_rotation(&decoder_program);
        assert!(matches!(
            outcome.as_ref().map(Option::is_none),
            Ok(true) | Err(_)
        ));
        if let Err(bail) = outcome {
            assert_eq!(bail.to_string(), "loop iteration cap exceeded");
        }
    });
}

fn large_never_matching_rotator(len: usize) -> String {
    format!(
        "\
function a() {{
  var h = [];
  for (var i = 0; i < {len}; i++) {{ h.push('x'); }}
  a = function () {{ return h; }};
  return a();
}}
function b(c, d) {{
  var e = a();
  return (b = function (f, g) {{ f = f - 0x0; var i = e[f]; return i; }}), b(c, d);
}}
(function (c, d) {{
  var k = b, f = c();
  while (!![]) {{
    try {{
      var g = parseInt(k(0x0)) / 0x1;
      if (g === d) break;
      else f['push'](f['shift']());
    }} catch (h) {{ f['push'](f['shift']()); }}
  }}
}})(a, 0x63);"
    )
}

#[test]
fn a_large_array_with_a_tiny_budget_bails_instead_of_doing_quadratic_native_work() {
    let source = large_never_matching_rotator(2000);
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::new(50_000));
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate survives building the array");
        let start = std::time::Instant::now();
        let outcome = session.apply_rotation(&decoder_program);
        let elapsed = start.elapsed();
        assert_eq!(
            outcome.err().map(|bail| bail.to_string()),
            Some("step budget exhausted".to_string())
        );
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "rotation should bail on budget long before doing O(len^2) native work: {elapsed:?}"
        );
    });
}

#[test]
fn an_interpreted_rotator_that_throws_bails() {
    let source = "\
function a() { var h = ['x', 'y']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) { f = f - 0x0; var i = e[f]; return i; }), b(c, d);
}
(function (c, d) {
  var e = c();
  var tries = 0;
  while (!![]) {
    try {
      if (b(0x0) === 'y') break;
      else e['push'](e['shift']());
    } catch (h) { e['push'](e['shift']()); }
    if (tries++ > 10) break;
  }
  throw 'stop';
})(a, 0x0);";
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("instantiate");
        let outcome = session.apply_rotation(&decoder_program);
        assert!(
            matches!(outcome, Err(ref bail) if bail.to_string().contains("rotator throws")),
            "{outcome:?}"
        );
    });
}
