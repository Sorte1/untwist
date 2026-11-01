mod common;

const PROBE_BAIL: &str = "\
var G = 'g';
function a() { var h = ['p', 'q', 'r', 's']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); return e[i] + G; }
console.log(b(0)); console.log(b(1));";

const PROBE_BN: &str = "\
function a() { var h = ['p', 'q', 'r', 's']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); b.n = (b.n || 0) + 1; return e[i] + b.n; }
function k() { return b(1); }
console.log(b(0)); console.log(k()); console.log(b(2));";

const PROBE_CHAIN: &str = "\
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
console.log(b(0));
console.log(b(1));
console.log(b(2));
console.log(b(3));";

const PROBE_GATE: &str = "\
function a() { var h = ['p', 'q', 'r', 's']; a = function () { return h; }; return a(); }
function b(i) {
  var e = a();
  b = function (j) { return a()[j]; };
  return e[i] + '!';
}
function f() { return b(1); }
console.log(b(0));
console.log(f());
console.log(b(2));";

fn assert_no_stamp_names(code: &str) {
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_"), "{code}");
}

#[test]
fn a_decoder_that_bails_on_an_outer_global_keeps_its_original_names() {
    let (code, notes) = common::run_passes(PROBE_BAIL);
    assert_no_stamp_names(&code);
    assert!(code.contains("function a("), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(code.contains('G'), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("reads a binding outside itself")),
        "node prints pg then qg (b(0)+G, b(1)+G); the outer global read must refuse the unit \
         and the note must survive: {notes:?}"
    );
}

#[test]
fn an_unconditional_self_property_counter_reached_through_a_helper_keeps_its_original_names() {
    let (code, notes) = common::run_passes(PROBE_BN);
    assert_no_stamp_names(&code);
    assert!(code.contains("function a("), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(code.contains("function k("), "{code}");
    assert!(
        notes.iter().any(|note| note
            .contains("a stateful decoder keeps state that is not a proven memo or init gate")),
        "node prints p1 q2 r3 (b(0), k()->b(1), b(2), the counter advancing with node's own \
         call order); the unit must refuse rather than inline a source-order guess: {notes:?}"
    );
}

#[test]
fn a_self_rewrite_chain_keeps_its_original_names() {
    let (code, notes) = common::run_passes(PROBE_CHAIN);
    assert_no_stamp_names(&code);
    assert!(code.contains("function a("), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note
                .contains("a stateful decoder writes a variable outside its own machinery")),
        "node prints px qy rz sz (each call sees the settled function from the previous \
         call); the second-level self-rewrite must refuse rather than inline a stale \
         snapshot: {notes:?}"
    );
}

#[test]
fn a_self_rewrite_outside_the_forwarding_template_keeps_its_original_names() {
    let (code, notes) = common::run_passes(PROBE_GATE);
    assert_no_stamp_names(&code);
    assert!(code.contains("function a("), "{code}");
    assert!(code.contains("function b("), "{code}");
    assert!(code.contains("function f("), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note
                .contains("a stateful decoder writes a variable outside its own machinery")),
        "node prints p! q r (the outer call's own '!' suffix belongs only to that call, not \
         to the rewritten function's later calls); a non-forwarding self-rewrite must \
         refuse: {notes:?}"
    );
}
