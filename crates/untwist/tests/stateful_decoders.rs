use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

enum Expect {
    Refuse(&'static str),
    Decode(&'static str),
    Partial(&'static str),
}

struct Row {
    source: &'static str,
    prints: &'static str,
    expect: Expect,
}

const STAMPS: [&str; 2] = ["__STRING_ARRAY__", "__DECODE_"];

fn modes() -> [Options; 2] {
    [
        Options::default(),
        Options {
            extended: false,
            unsafe_transforms: true,
            ..Options::default()
        },
    ]
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn printed(node: &Path, text: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("program.js");
    std::fs::write(&path, text).unwrap();
    run(node, &path)
        .unwrap()
        .map(|out| out.stdout)
        .unwrap_or_default()
}

fn check_shape(row: &Row, code: &str) {
    for stamp in STAMPS {
        assert!(!code.contains(stamp), "a stamp name is left: {code}");
    }
    match row.expect {
        Expect::Refuse(call) => assert!(code.contains(call), "the call must stay: {code}"),
        Expect::Decode(literal) | Expect::Partial(literal) => {
            assert!(code.contains(literal), "the call must decode: {code}")
        }
    }
}

fn check(row: &Row) {
    let node = node_or_skip();
    if let Some(node) = &node {
        assert_eq!(printed(node, row.source), row.prints, "node on the input");
    }
    for options in modes() {
        let code = Pipeline::new(options).run(row.source).code().to_string();
        check_shape(row, &code);
        if let Some(node) = &node {
            assert_eq!(
                printed(node, &code),
                row.prints,
                "node on the output: {code}"
            );
        }
    }
}

const DIRECT_CLOSURE_MEMO: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f,g){ var e=a(); if (b[f]===undefined){ b[f]=e[f]+g; } return b[f]; }
function later(){ return b(0,'y'); }
console.log(b(0,'x')); console.log(later()); console.log(b(1,'x'));"#;

const REWRITTEN_DIRECT_MEMO: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f,g){ var e=a(); return (b=function(f,g){ if (b[f]===undefined){ b[f]=e[f]+g; } return b[f]; }), b(f,g); }
function later(){ return b(0,'y'); }
console.log(b(0,'x')); console.log(later()); console.log(b(1,'x'));"#;

const ARRAY_COUNTER_ALIAS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.A=[0]; b.g=true; } var A=b.A; A[0]=A[0]+1; return e[f]+A[0]; }), b(f); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const ARGUMENTS_COUNTER: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.A=arguments; b.g=true; } var A=b.A; A[0]=A[0]+1; return e[f]+A[0]; }), b(f); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const SELF_ALIAS_COUNTER: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var s=b; s.n=(s.n||0)+1; return e[i]+s.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const ARRAY_AFTER_DISCOVERY: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.A=[0]; b.g=true; } if (b.A.k===undefined){ b.A.k=1; } var A=b.A; A[0]=A[0]+1; return e[f]+A[0]; }), b(f); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const TWO_UNITS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f,g){ var e=a(); return (b=function(f,g){ if (b.g===undefined){ b.c={}; b.g=true; } if (b.c[f]===undefined){ b.c[f]=e[f]; } return b.c[f]; }), b(f,g); }
function x(){ var h=['t0','t1','t2']; x=function(){return h;}; return x(); }
function y(f){ var e=x(); return (y=function(f){ if (y.g===undefined){ y.A=[0]; y.g=true; } if (y.A.k===undefined){ y.A.k=1; } var A=y.A; A[0]=A[0]+1; return e[f]+A[0]; }), y(f); }
function later(){ return y(1) + b(1); }
console.log(b(0), y(0)); console.log(later()); console.log(y(2), b(2));"#;

const GATE_READS_ARGUMENTS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.g=true; b.first=arguments[0]; } return e[f]+b.first; }), b(f); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const PLAIN_GATE_NO_REWRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); if (b.g===undefined){ b.g=true; b.first=e[f]; } return e[f]+b.first; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const GATE_THROWS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.g=true; if (f===0) throw 'boom'; } return e[f]; }), b(f); }
function later(){ try { return b(1); } catch(x) { return 'caught'; } }
try { console.log(b(0)); } catch(x) { console.log('c0'); } console.log(later()); console.log(b(2));"#;

const MEMO_ALIAS_ESCAPE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function d(i,k){ var e=a(); return (d=function(i,k){ if (d.g===undefined){ d.c={}; d.g=true; } var C=d.c; if (C[i]===undefined){ C[i]=e[i]+k; } return C[i]; }), d(i,k); }
var kk = ['x','y'][Math.floor(Math.random()*0)];
console.log(d(0,kk)); console.log(d(0,'z')); console.log(d(1,'z'));"#;

const GATE_READS_PARAM: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(f){ var e=a(); return (b=function(f){ if (b.g===undefined){ b.g=true; b.first=f; } return e[f]+b.first; }), b(f); }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const PARTIAL_STATELESS: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
var kk = [0,1][Math.floor(Math.random()*0)];
console.log(b(kk)); console.log(b(1)); console.log(b(2));"#;

const TEMPLATE_HIDDEN_WRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + `${b.n=(b.n||0)+1}`; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const FOR_IN_HIDDEN_WRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); for (var q in {z:1}) { b.n=(b.n||0)+1; } return e[i] + b.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const NESTED_FUNCTION_WRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); (function(){ b.n=(b.n||0)+1; })(); return e[i] + b.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const OBJECT_KEY_HIDDEN_WRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var o={[b.n=(b.n||0)+1]:1}; return e[i] + Object.keys(o)[0]; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const ARROW_WRITE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); var inc=()=>{ b.n=(b.n||0)+1; }; inc(); return e[i] + b.n; }
function later(){ return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const GATE_MEMO_FULL_KEY: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f + g] === undefined) { b.c[f + g] = e[f] + g; }
    return b.c[f + g];
  }), b(f, g);
}
function later() { return b(0, 'y'); }
console.log(b(0, 'x')); console.log(later()); console.log(b(1, 'x'));"#;

const GATE_MEMO_PARTIAL_KEY: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f] + g; }
    return b.c[f];
  }), b(f, g);
}
function later() { return b(0, 'y'); }
console.log(b(0, 'x')); console.log(later()); console.log(b(1, 'x'));"#;

const GATE_AND_MEMO: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f, g);
}
function later() { return b(1); }
console.log(b(0), later(), b(2), b(1));"#;

const INDEX_AND_PASSWORD_MEMO: &str = r#"function a() { var h = ['r0', 'r1', 'r2']; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.c === undefined) { b.c = {}; }
    if (b.c[f + g] === undefined) { b.c[f + g] = e[f] + g; }
    return b.c[f + g];
  }), b(f, g);
}
function later() { return b(0, 'y'); }
console.log(b(0, 'x'), later(), b(1, 'y'), b(0, 'x'));"#;

const NESTED_STATEFUL_DECODERS: &str = r#"function a() { var h = ['n0', 'n1']; a = function () { return h; }; return a(); }
function c(f) {
  var e = a();
  return (c = function (f) {
    if (c.k === undefined) { c.m = {}; c.k = true; }
    if (c.m[f] === undefined) { c.m[f] = e[f]; }
    return c.m[f];
  }), c(f);
}
function b(f) {
  var e = a();
  if (b.k === undefined) { b.k = true; }
  return c(f) + e[f];
}
function later() { return b(1); }
console.log(b(0), later(), b(0));"#;

const SIBLING_HELPER: &str = r#"function a() { var h = ['t0', 't1', 't2']; a = function () { return h; }; return a(); }
function c(f, g) {
  var e = a();
  return (c = function (f, g) {
    if (c.m === undefined) { c.m = {}; }
    if (c.m[f] === undefined) { c.m[f] = e[f]; }
    return c.m[f];
  }), c(f, g);
}
function b(f, g) {
  var e = a();
  return c(f) + e[f];
}
function later() { return b(1); }
console.log(b(0), later(), c(2), b(1));"#;

const GATE_METHOD_COUNTS_THROUGH_THIS: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.g === undefined) { b.m = function () { this.n = (this.n || 0) + 1; }; b.g = true; }
    b.m();
    return e[f] + b.n;
  }), b(f);
}
function later() { return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const CAPTURED_ARGUMENTS_MEMO: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { b.r = function (x, y) { return x + y; }; c = arguments; b.k = true; }
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = b.r(e[f], g); c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
function later() { return b(1, 'x'); }
console.log(b(0, 'x')); console.log(later()); console.log(b(2, 'x')); console.log(b(0, 'x'));"#;

const CAPTURED_ARGUMENTS_MEMO_PARTIAL_KEY: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { b.r = function (x, y) { return x + y; }; c = arguments; b.k = true; }
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = b.r(e[f], g); c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
function later() { return b(0, 'y'); }
console.log(b(0, 'x')); console.log(later()); console.log(b(1, 'x'));"#;

const ESCAPED_CALL_TO_A_CAPTURED_ARGUMENTS_MEMO: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { b.r = function (x, y) { return x + y; }; c = arguments; b.k = true; }
    var n = f + e[0];
    var hit = c[n];
    var v;
    if (!hit) { v = b.r(e[f], g); c[n] = v; } else { v = hit; }
    return v;
  }), b(c, d);
}
var k = String.fromCharCode(121);
console.log(b(0, k)); console.log(b(0, 'x')); console.log(b(1, 'x'));"#;

const COUNTER_IN_A_TEMPLATE_ARRAY_ELEMENT: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); var t = [0, `${b.n = (b.n || 0) + 1}`]; return e[i] + b.n; }
function later() { return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const NESTED_VAR_SHADOWS_THE_DECODER: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(i) { var e = a(); var t = function () { var b = {}; b.n = 1; return b.n; }; return e[i] + t(); }
function later() { return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const CAPTURED_ARGUMENTS_READ_BEFORE_ANY_WRITE: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  return (b = function (f, g) {
    if (b.k === undefined) { c = arguments; b.k = true; }
    return e[f] + c[0];
  }), b(c, d);
}
function later() { return b(1); }
console.log(b(0)); console.log(later()); console.log(b(2));"#;

const GATE_REPLACES_A_MEMO_CONTAINER: &str = r#"function a() { var h = ['s0', 's1', 's2']; a = function () { return h; }; return a(); }
function b(f) {
  var e = a();
  return (b = function (f) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.h === undefined) { b.c = {}; b.h = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f]; }
    return b.c[f];
  }), b(f);
}
console.log(b(0), b(1));"#;

#[test]
fn a_memo_on_the_decoders_own_closure_is_refused() {
    check(&Row {
        source: DIRECT_CLOSURE_MEMO,
        prints: "s0x\ns0x\ns1x\n",
        expect: Expect::Refuse("console.log(b(0, \"x\"))"),
    });
}

#[test]
fn a_memo_on_the_rewritten_decoders_own_closure_is_refused() {
    check(&Row {
        source: REWRITTEN_DIRECT_MEMO,
        prints: "s0x\ns0x\ns1x\n",
        expect: Expect::Refuse("console.log(b(0, \"x\"))"),
    });
}

#[test]
fn a_gate_made_array_counted_through_a_local_alias_is_refused() {
    check(&Row {
        source: ARRAY_COUNTER_ALIAS,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_gate_stored_arguments_object_counted_through_a_local_alias_is_refused() {
    check(&Row {
        source: ARGUMENTS_COUNTER,
        prints: "s11\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counter_written_through_a_local_alias_of_the_decoder_is_refused() {
    check(&Row {
        source: SELF_ALIAS_COUNTER,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn an_array_counter_after_a_gate_key_discovery_is_refused() {
    check(&Row {
        source: ARRAY_AFTER_DISCOVERY,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counting_unit_beside_a_clean_unit_is_refused_alone() {
    check(&Row {
        source: TWO_UNITS,
        prints: "s0 t01\nt12s1\nt23 s2\n",
        expect: Expect::Refuse("y(0)"),
    });
}

#[test]
fn a_gate_that_reads_arguments_is_refused() {
    check(&Row {
        source: GATE_READS_ARGUMENTS,
        prints: "s00\ns10\ns20\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_plain_gate_that_reads_the_call_is_refused() {
    check(&Row {
        source: PLAIN_GATE_NO_REWRITE,
        prints: "s0s0\ns1s0\ns2s0\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_gate_that_throws_for_one_call_is_refused() {
    check(&Row {
        source: GATE_THROWS,
        prints: "c0\ns1\ns2\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_memo_reached_through_an_alias_with_a_dynamic_call_is_refused() {
    check(&Row {
        source: MEMO_ALIAS_ESCAPE,
        prints: "s0x\ns0x\ns1z\n",
        expect: Expect::Refuse("d(0, \"z\")"),
    });
}

#[test]
fn a_gate_that_reads_a_parameter_is_refused() {
    check(&Row {
        source: GATE_READS_PARAM,
        prints: "s00\ns10\ns20\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_partly_decoded_stateless_unit_gets_its_names_back() {
    check(&Row {
        source: PARTIAL_STATELESS,
        prints: "s0\ns1\ns2\n",
        expect: Expect::Partial("\"s1\""),
    });
}

#[test]
fn a_counter_written_inside_a_template_literal_is_refused() {
    check(&Row {
        source: TEMPLATE_HIDDEN_WRITE,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counter_written_inside_a_for_in_body_is_refused() {
    check(&Row {
        source: FOR_IN_HIDDEN_WRITE,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counter_written_inside_a_nested_function_is_refused() {
    check(&Row {
        source: NESTED_FUNCTION_WRITE,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counter_written_inside_a_computed_object_key_is_refused() {
    check(&Row {
        source: OBJECT_KEY_HIDDEN_WRITE,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_counter_written_inside_an_arrow_is_refused() {
    check(&Row {
        source: ARROW_WRITE,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_gate_made_memo_keyed_on_every_read_argument_decodes() {
    check(&Row {
        source: GATE_MEMO_FULL_KEY,
        prints: "s0x\ns0y\ns1x\n",
        expect: Expect::Decode("\"s0x\""),
    });
}

#[test]
fn a_gate_made_memo_whose_key_omits_a_read_argument_is_refused() {
    check(&Row {
        source: GATE_MEMO_PARTIAL_KEY,
        prints: "s0x\ns0x\ns1x\n",
        expect: Expect::Refuse("console.log(b(0, \"x\"))"),
    });
}

#[test]
fn an_init_gate_plus_an_index_keyed_memo_decodes() {
    check(&Row {
        source: GATE_AND_MEMO,
        prints: "s0 s1 s2 s1\n",
        expect: Expect::Decode("\"s0\""),
    });
}

#[test]
fn a_memo_keyed_on_index_and_password_decodes() {
    check(&Row {
        source: INDEX_AND_PASSWORD_MEMO,
        prints: "r0x r0y r1y r0x\n",
        expect: Expect::Decode("\"r0x\""),
    });
}

#[test]
fn a_stateful_decoder_nesting_another_is_left_to_the_read_only_rule() {
    check(&Row {
        source: NESTED_STATEFUL_DECODERS,
        prints: "n0n0 n1n1 n0n0\n",
        expect: Expect::Refuse("console.log(b(0)"),
    });
}

#[test]
fn a_sibling_decoder_helper_is_left_to_the_read_only_rule() {
    check(&Row {
        source: SIBLING_HELPER,
        prints: "t0t0 t1t1 t2 t1t1\n",
        expect: Expect::Refuse("console.log(b(0)"),
    });
}

#[test]
fn a_method_stored_by_a_gate_counting_through_this_is_refused() {
    check(&Row {
        source: GATE_METHOD_COUNTS_THROUGH_THIS,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn an_arguments_object_captured_by_a_gate_serves_as_a_consistent_memo() {
    check(&Row {
        source: CAPTURED_ARGUMENTS_MEMO,
        prints: "s0x\ns1x\ns2x\ns0x\n",
        expect: Expect::Decode("\"s2x\""),
    });
}

#[test]
fn an_arguments_memo_whose_key_omits_a_read_argument_is_refused() {
    check(&Row {
        source: CAPTURED_ARGUMENTS_MEMO_PARTIAL_KEY,
        prints: "s0x\ns0x\ns1x\n",
        expect: Expect::Refuse("console.log(b(0, \"x\"))"),
    });
}

#[test]
fn a_gate_that_replaces_a_registered_memo_container_is_refused() {
    check(&Row {
        source: GATE_REPLACES_A_MEMO_CONTAINER,
        prints: "s0 s1\n",
        expect: Expect::Refuse("console.log(b(0), b(1))"),
    });
}

#[test]
fn a_dynamic_call_to_a_decoder_with_a_captured_arguments_memo_is_refused() {
    check(&Row {
        source: ESCAPED_CALL_TO_A_CAPTURED_ARGUMENTS_MEMO,
        prints: "s0y\ns0y\ns1x\n",
        expect: Expect::Refuse("console.log(b(0, \"x\"))"),
    });
}

#[test]
fn a_counter_written_inside_a_template_array_element_is_refused() {
    check(&Row {
        source: COUNTER_IN_A_TEMPLATE_ARRAY_ELEMENT,
        prints: "s01\ns12\ns23\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}

#[test]
fn a_nested_functions_var_shadowing_the_decoder_decodes() {
    check(&Row {
        source: NESTED_VAR_SHADOWS_THE_DECODER,
        prints: "s01\ns11\ns21\n",
        expect: Expect::Decode("console.log(\"s01\")"),
    });
}

#[test]
fn a_gate_captured_arguments_object_read_before_any_write_is_refused() {
    check(&Row {
        source: CAPTURED_ARGUMENTS_READ_BEFORE_ANY_WRITE,
        prints: "s00\ns10\ns20\n",
        expect: Expect::Refuse("console.log(b(0))"),
    });
}
