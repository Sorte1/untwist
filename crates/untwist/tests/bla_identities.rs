use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

fn lane(source: &str, unsafe_transforms: bool, commonjs_goal: bool) -> String {
    Pipeline::new(Options {
        unsafe_transforms,
        commonjs_goal,
        ..Options::default()
    })
    .run(source)
    .code()
    .to_string()
}

fn commonjs(source: &str) -> String {
    lane(source, false, true)
}

fn commonjs_unsafe(source: &str) -> String {
    lane(source, true, true)
}

fn script(source: &str) -> String {
    lane(source, false, false)
}

const FRAME_SEED: &str = r#"var seed = 0x1234 | 1;
function piece(flag) {
  const frame = typeof seed === "number" ? seed : 2300845415;
  const word = ((frame ^ 1168984216) >>> 0 | ((frame ^ 1168984216) >>> 0) - 1 >>> 0) >>> 0 >= ((frame ^ 1168984216) >>> 0 ^ ((frame ^ 1168984216) >>> 0) - 1 >>> 0) >>> 0 ? "ln" : "plic";
  return flag + word;
}
console.log(piece("a"));
"#;

const NEVER_GUARD: &str = r#"function guarded(input = 0) {
  const u = (input ^ 668265263) >>> 0;
  const never = (u | u - 1 >>> 0) >>> 0 < (u ^ u - 1 >>> 0) >>> 0;
  if (!never) return input ^ 49092;
  globalThis.process?.dlopen?.({ exports: {} }, "native.node");
  return input;
}
console.log(guarded(7));
"#;

const PARAMETER_LEAF: &str = r#"function judged(value) {
  return ((value ^ 5) >>> 0 | ((value ^ 5) >>> 0) - 1 >>> 0) >>> 0 < ((value ^ 5) >>> 0 ^ ((value ^ 5) >>> 0) - 1 >>> 0) >>> 0;
}
module.exports = judged;
"#;

const HOISTED_READ: &str = r#"function outer() {
  const r = early();
  const u = 9 >>> 0;
  return r;
  function early() {
    try {
      return (u | u - 1 >>> 0) >>> 0 < (u ^ u - 1 >>> 0) >>> 0;
    } catch (error) {
      return error.name;
    }
  }
}
console.log(outer());
"#;

const COUNTER: &str = r#"console.log("start");
var counter = 0xeb | 1;
function check(depth) {
  if (depth > 0 && (Math.imul(534532529, Math.imul(2260162385, counter | 27) - Math.imul(2260162385, counter & 27) + 2031794264 | 0) + 3163730728 | 0) !== (counter ^ 27 | 0)) {
    console.log("junk");
  }
  while (depth && (Math.imul(1537760857, Math.imul(1205406185, counter ^ 74) + Math.imul(2410812370, counter & 74) + 1365432373 | 0) + 2943836051 | 0) !== (counter + 74 | 0)) {
    depth--;
  }
  return depth;
}
console.log(check(2), check.length >= 0);
"#;

const COUNTER_READ_EARLY: &str = r#"console.log(check(2));
var counter = 0xeb | 1;
function check(depth) {
  if ((Math.imul(1537760857, Math.imul(1205406185, counter ^ 74) + Math.imul(2410812370, counter & 74) + 1365432373 | 0) + 2943836051 | 0) !== (counter + 74 | 0)) {
    return "early";
  }
  return depth;
}
"#;

const GUARDED_PATCH: &str = r#"var counter = 235;
function patcher(hook) {
  function expose() {
    return Map.prototype;
  }
  let cursor = { target: expose, next: null };
  if (hook && (Math.imul(1537760857, Math.imul(1205406185, counter ^ 74) + Math.imul(2410812370, counter & 74) + 1365432373 | 0) + 2943836051 | 0) !== (counter + 74 | 0)) {
    hook(cursor.target());
  }
  return 1;
}
module.exports = patcher;
"#;

const LIVE_PATCH: &str = r#"var counter = 235;
function patcher(hook) {
  hook(Map.prototype);
  return (Math.imul(1537760857, Math.imul(1205406185, counter ^ 74) + Math.imul(2410812370, counter & 74) + 1365432373 | 0) + 2943836051 | 0) !== (counter + 74 | 0);
}
console.log(patcher(function (proto) { proto.constructor.constructor("Math.imul = () => 0")(); }));
"#;

fn programs() -> Vec<(String, String)> {
    [
        ("frame-seed", FRAME_SEED),
        ("never-guard", NEVER_GUARD),
        ("parameter-leaf", PARAMETER_LEAF),
        ("hoisted-read", HOISTED_READ),
        ("counter", COUNTER),
        ("counter-read-early", COUNTER_READ_EARLY),
        ("guarded-patch", GUARDED_PATCH),
        ("live-patch", LIVE_PATCH),
    ]
    .into_iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect()
}

#[test]
fn every_identity_program_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let runs: [Mode<'_>; 2] = [("default", &commonjs), ("unsafe", &commonjs_unsafe)];
    let wrong = node_rows::commonjs_mismatches(&node, &programs(), &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_or_never_below_xor_on_plain_numbers_folds() {
    for code in [commonjs(FRAME_SEED), commonjs_unsafe(FRAME_SEED)] {
        assert!(!code.contains(">>> 0 >="), "{code}");
        assert!(!code.contains("plic"), "{code}");
    }
    let code = commonjs(NEVER_GUARD);
    assert!(!code.contains("dlopen"), "{code}");
    assert!(!code.contains(">>> 0 <"), "{code}");
}

#[test]
fn an_identity_over_a_value_that_may_not_be_a_plain_number_stays() {
    assert!(commonjs(PARAMETER_LEAF).contains(">>> 0 <"));
    assert!(commonjs(HOISTED_READ).contains(">>> 0 <"));
}

#[test]
fn a_counter_never_written_decides_its_predicates() {
    for code in [commonjs(COUNTER), commonjs_unsafe(COUNTER)] {
        assert!(!code.contains("Math.imul"), "{code}");
        assert!(!code.contains("junk"), "{code}");
    }
}

#[test]
fn a_counter_read_before_its_declaration_keeps_its_predicates() {
    assert!(commonjs(COUNTER_READ_EARLY).contains("Math.imul"));
}

#[test]
fn a_script_global_counter_keeps_its_predicates() {
    assert!(script(COUNTER).contains("Math.imul"));
}

#[test]
fn a_patch_reachable_only_from_a_dead_guard_does_not_block_math_imul() {
    let code = commonjs(GUARDED_PATCH);
    assert!(!code.contains("Math.imul"), "{code}");
    assert!(commonjs(LIVE_PATCH).contains("Math.imul"));
}
