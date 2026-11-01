mod common;

use std::panic::{catch_unwind, AssertUnwindSafe};

use rustc_hash::FxHashSet;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Evaluator, InterpEvaluator};
use untwist_eval::rotation::RotationMemo;
use untwist_eval::state::{set_state_tracing, state_tracing, Shape};

#[test]
fn state_tracing_guard_clears_the_flag_after_an_early_return() {
    fn drive_and_bail() -> Result<(), &'static str> {
        let shape = Shape::default();
        let _guard = set_state_tracing(Some(&shape));
        assert!(state_tracing());
        Err("boom")?;
        Ok(())
    }
    assert!(drive_and_bail().is_err());
    assert!(!state_tracing());
}

#[test]
fn state_tracing_guard_clears_the_flag_after_a_panic() {
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let shape = Shape::default();
        let _guard = set_state_tracing(Some(&shape));
        assert!(state_tracing());
        panic!("boom");
    }));
    assert!(caught.is_err());
    assert!(!state_tracing());
}

fn assert_refused_with_escapes(
    source: &str,
    array: &str,
    decoders: &[&str],
    escaped_decoder: usize,
    expected_reason: &str,
) {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, array, decoders, None);
        let calls = common::decoder_calls_anywhere(program, decoders);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut escaped_decoders = FxHashSet::default();
        escaped_decoders.insert(escaped_decoder);
        let result = evaluator.decode_with_memo(
            &decoder_program,
            &calls,
            &mut RotationMemo::default(),
            &escaped_decoders,
        );
        assert_eq!(
            result.err().map(|bail| bail.to_string()),
            Some(format!("unsupported construct: {expected_reason}")),
            "source: {source}"
        );
    });
}

fn assert_refused(source: &str, array: &str, decoders: &[&str], expected_reason: &str) {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, array, decoders, None);
        let calls = common::decoder_calls_anywhere(program, decoders);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let result = evaluator.decode(&decoder_program, &calls);
        assert_eq!(
            result.err().map(|bail| bail.to_string()),
            Some(format!("unsupported construct: {expected_reason}")),
            "source: {source}"
        );
    });
}

fn assert_decodes(source: &str, array: &str, decoders: &[&str], expected: &[&str]) {
    common::with_program(source, |program| {
        let decoder_program = common::decoder_program(program, array, decoders, None);
        let calls = common::decoder_calls_anywhere(program, decoders);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let values = evaluator
            .decode(&decoder_program, &calls)
            .unwrap_or_else(|bail| panic!("source: {source}, expected a decode, got {bail}"));
        let rendered: Vec<String> = values.iter().map(common::render_decoded).collect();
        let expected: Vec<String> = expected.iter().map(|s| format!("string:{s}")).collect();
        assert_eq!(rendered, expected, "source: {source}");
    });
}

const CALL_HISTORY_COUNTER: &str = "\
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
b(0); b(1); b(2); b(3);";

#[test]
fn an_unconditional_self_property_counter_folded_into_the_return_is_refused() {
    assert_refused(
        CALL_HISTORY_COUNTER,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
    );
}

const ENCLOSING_COUNTER: &str = "\
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
b(0);";

#[test]
fn a_counter_declared_in_the_enclosing_scope_is_refused() {
    assert_refused(
        ENCLOSING_COUNTER,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const ARRAY_WRITES_DECODER_CONTAINER: &str = "\
function a() { var h = ['z0', 'z1']; b.c = {}; return h; }
function b(f) {
  var e = a();
  if (b.c[f] === undefined) { b.c[f] = e[f]; }
  return b.c[f];
}
b(0);";

#[test]
fn the_array_function_writing_a_decoders_own_container_is_refused() {
    assert_refused(
        ARRAY_WRITES_DECODER_CONTAINER,
        "a",
        &["b"],
        "a stateful decoder's state is written by a different machinery function",
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
k(1);
b(0);";

#[test]
fn a_sibling_decoder_writing_the_first_decoders_container_is_refused() {
    assert_refused(
        SIBLING_DECODER_WRITES_CONTAINER,
        "a",
        &["b", "k"],
        "a stateful decoder's state is written by a different machinery function",
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
b(0);";

#[test]
fn two_independent_gates_sharing_the_same_guarded_property_are_refused() {
    assert_refused(
        TWO_GATES_SHARE_A_FLAG,
        "a",
        &["b"],
        "two init gates share the same guarded property",
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
b(0);";

#[test]
fn a_sloppy_mode_this_write_is_refused() {
    assert_refused(
        SLOPPY_THIS_WRITE,
        "a",
        &["b"],
        "a stateful decoder writes through a sloppy-mode this",
    );
}

const KEY_OMITS_A_PARAMETER_THE_CODE_READS_ELSEWHERE: &str = "\
function a() { var h = ['q0', 'q1', 'q2']; a = function () { return h; }; return a(); }
function d(i, k) {
  var e = a();
  return (d = function (i, k) {
    if (d.c === undefined) { d.c = {}; }
    if (d.c[i] === undefined) { d.c[i] = e[i] + k; }
    return d.c[i];
  }), d(i, k);
}
d(0, 'x'); d(1, 'y'); d(0, 'z');";

#[test]
fn a_memo_key_that_omits_a_read_parameter_is_refused() {
    assert_refused(
        KEY_OMITS_A_PARAMETER_THE_CODE_READS_ELSEWHERE,
        "a",
        &["d"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
    );
}

const A_GLOBAL_WRITTEN_BESIDE_A_NESTED_FUNCTIONS_VAR: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) {
  var e = a();
  var t = function () { var s = 0; return s; };
  s = (typeof s === 'number' ? s : 0) + 1;
  return e[i] + s + t();
}
b(0); b(1);";

#[test]
fn a_var_of_a_nested_function_expression_does_not_declare_the_decoders_write_target() {
    assert_refused(
        A_GLOBAL_WRITTEN_BESIDE_A_NESTED_FUNCTIONS_VAR,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const OUTER_PARAMETER_BESIDE_A_CATCH_PARAMETER: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    try { e.x; } catch (c) { }
    c = (c | 0) + 1;
    return e[f] + c;
  }), b(c, d);
}
b(0); b(1);";

#[test]
fn a_catch_parameter_does_not_hide_a_write_to_the_outer_frame() {
    assert_refused(
        OUTER_PARAMETER_BESIDE_A_CATCH_PARAMETER,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const OUTER_PARAMETER_BESIDE_A_FOR_LET_BINDING: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    for (let c = 0; c < 0; c++) { }
    c = (c | 0) + 1;
    return e[f] + c;
  }), b(c, d);
}
b(0); b(1);";

#[test]
fn a_for_let_binding_does_not_hide_a_write_to_the_outer_frame() {
    assert_refused(
        OUTER_PARAMETER_BESIDE_A_FOR_LET_BINDING,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const OUTER_PARAMETER_BESIDE_A_FOR_OF_CONST_BINDING: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    for (const c of []) { }
    c = (c | 0) + 1;
    return e[f] + c;
  }), b(c, d);
}
b(0); b(1);";

#[test]
fn a_for_of_const_binding_does_not_hide_a_write_to_the_outer_frame() {
    assert_refused(
        OUTER_PARAMETER_BESIDE_A_FOR_OF_CONST_BINDING,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const OUTER_PARAMETER_WITHOUT_A_BLOCK_BINDING: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    
    c = (c | 0) + 1;
    return e[f] + c;
  }), b(c, d);
}
b(0); b(1);";

#[test]
fn a_write_to_the_outer_frame_is_refused() {
    assert_refused(
        OUTER_PARAMETER_WITHOUT_A_BLOCK_BINDING,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const A_CATCH_PARAMETER_WRITTEN_INSIDE_ITS_CLAUSE: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) {
  var e = a();
  try { null.x; } catch (s) { s = 1; }
  for (let k = 0; k < 1; k++) { k = k + 1; }
  return e[i];
}
b(0); b(1);";

#[test]
fn a_catch_parameter_and_a_for_let_binding_written_in_their_own_blocks_still_decode() {
    assert_decodes(
        A_CATCH_PARAMETER_WRITTEN_INSIDE_ITS_CLAUSE,
        "a",
        &["b"],
        &["s0", "s1"],
    );
}

const HELPER_CALL_TO_A_SIBLING_DECODER: &str = "\
function a() { var h = ['t0', 't1', 't2']; a = function () { return h; }; return a(); }
function c(f, g) {
  var e = a();
  return (c = function (f, g) {
    if (c.cache === undefined) { c.cache = {}; }
    if (c.cache[f] === undefined) { c.cache[f] = e[f]; }
    return c.cache[f];
  }), c(f, g);
}
function b(f, g) {
  return c(f) + '!';
}
b(0); b(1);";

#[test]
fn a_decoder_calling_a_sibling_decoder_as_a_plain_helper_decodes() {
    assert_decodes(
        HELPER_CALL_TO_A_SIBLING_DECODER,
        "a",
        &["b", "c"],
        &["t0!", "t1!"],
    );
}

const A_STATEFUL_DECODER_NESTS_ANOTHER: &str = "\
function a() { var h = ['n0', 'n1']; a = function () { return h; }; return a(); }
function c(f) {
  var e = a();
  return (c = function (f) {
    if (c.g === undefined) { c.c = {}; c.g = true; }
    if (c.c[f] === undefined) { c.c[f] = e[f]; }
    return c.c[f];
  }), c(f);
}
function b(f) {
  if (b.g === undefined) { b.g = true; }
  return c(f) + '!';
}
b(0); b(1); b(0);";

#[test]
fn a_stateful_decoder_calling_another_stateful_decoder_decodes_with_independent_frames() {
    assert_decodes(
        A_STATEFUL_DECODER_NESTS_ANOTHER,
        "a",
        &["b", "c"],
        &["n0!", "n1!", "n0!"],
    );
}

const OBFUSCATOR_IO_BASE64_STYLE_GATE_AND_MEMO: &str = "\
function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
b(0); b(1); b(2);";

#[test]
fn an_init_gate_plus_an_index_keyed_memo_decodes() {
    assert_decodes(
        OBFUSCATOR_IO_BASE64_STYLE_GATE_AND_MEMO,
        "a",
        &["b"],
        &["s0", "s1", "s2"],
    );
}

const RC4_STYLE_MEMO_KEYED_ON_INDEX_AND_PASSWORD: &str = "\
function a() { var h = ['r0', 'r1', 'r2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.c === undefined) { b.c = {}; }
    if (b.c[f + g] === undefined) { b.c[f + g] = e[f]; }
    return b.c[f + g];
  }), b(f, g);
}
b(0, 'x'); b(1, 'y');";

#[test]
fn a_memo_keyed_on_every_read_argument_decodes() {
    assert_decodes(
        RC4_STYLE_MEMO_KEYED_ON_INDEX_AND_PASSWORD,
        "a",
        &["b"],
        &["r0", "r1"],
    );
}

const DYNAMIC_CALL_TO_A_MEMO_OWNER: &str = "\
function a() { var h = ['q0', 'q1', 'q2']; a = function () { return h; }; return a(); }
function d(i, k) {
  var e = a();
  return (d = function (i, k) {
    if (d.c === undefined) { d.c = {}; }
    if (d.c[i] === undefined) { d.c[i] = e[i]; }
    return d.c[i];
  }), d(i, k);
}
d(0, 'k2');";

#[test]
fn a_dynamic_call_to_a_memo_owning_decoder_refuses_the_unit() {
    assert_refused_with_escapes(
        DYNAMIC_CALL_TO_A_MEMO_OWNER,
        "a",
        &["d"],
        0,
        "a stateful decoder is referenced outside the calls untwist proves",
    );
}

const MEMO_OWNER_USED_AS_A_VALUE: &str = "\
function a() { var h = ['p0', 'p1', 'p2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
b(0); b(1); b(2);";

#[test]
fn a_memo_owning_decoder_passed_as_a_value_refuses_the_unit() {
    assert_refused_with_escapes(
        MEMO_OWNER_USED_AS_A_VALUE,
        "a",
        &["b"],
        0,
        "a stateful decoder is referenced outside the calls untwist proves",
    );
}

const SELF_REWRITE_CHAIN: &str = "\
function a() { var h = ['p', 'q', 'r', 's']; a = function () { return h; }; return a(); }
function b(i) {
  var e = a();
  b = function (j) {
    var e2 = a();
    b = function (k) { return a()[k] + 'z'; };
    return e2[j] + 'y';
  };
  return e[i] + 'x';
}
b(0); b(1); b(2); b(3);";

#[test]
fn a_self_rewrite_chain_is_refused() {
    assert_refused(
        SELF_REWRITE_CHAIN,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
    );
}

const SELF_REWRITE_OUTSIDE_THE_TEMPLATE: &str = "\
function a() { var h = ['p', 'q', 'r', 's']; a = function () { return h; }; return a(); }
function b(i) {
  var e = a();
  b = function (j) { return a()[j]; };
  return e[i] + '!';
}
function f() { return b(1); }
b(0); f(); b(2);";

#[test]
fn a_self_rewrite_whose_outer_body_is_not_the_forwarding_template_is_refused() {
    assert_refused(
        SELF_REWRITE_OUTSIDE_THE_TEMPLATE,
        "a",
        &["b"],
        "a stateful decoder writes a variable outside its own machinery",
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
b(0); b(1);";

#[test]
fn a_memo_container_created_as_a_non_plain_object_is_refused() {
    assert_refused(
        MEMO_CONTAINER_IS_AN_ARRAY_NOT_PLAIN,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
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
b(0); b(0);";

#[test]
fn a_cache_hit_that_returns_a_modified_value_is_refused() {
    assert_refused(
        A_CACHE_HIT_RETURNS_A_MODIFIED_VALUE,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
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
b(0); b(1);";

#[test]
fn a_gate_that_returns_out_of_its_own_body_is_refused() {
    assert_refused(
        A_GATE_RETURNS_OUT_OF_ITS_OWN_BODY,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
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
b(0); b(1);";

#[test]
fn a_gate_that_calls_a_decoder_is_refused() {
    assert_refused(
        A_GATE_CALLS_A_DECODER,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
    );
}

const AN_ARRAY_HELD_BY_A_GATE: &str = "\
function a() { var h = ['n0', 'n1']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.g === undefined) { b.c = ['k0', 'k1']; b.g = true; }
    return e[f] + b.c[f];
  }), b(f);
}
b(0); b(1);";

#[test]
fn an_array_held_by_a_gate_and_read_through_a_number_key_is_refused() {
    assert_refused(
        AN_ARRAY_HELD_BY_A_GATE,
        "a",
        &["b"],
        "a stateful decoder keeps state that is not a proven memo or init gate",
    );
}
