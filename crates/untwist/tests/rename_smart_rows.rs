use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{mismatches, node_or_skip, printed, Mode};

const EVIDENCE: &str = "var _0x1a = [3, 1, 2];
function _0x2b(_0x3c) {
  var _0x4d = 0;
  for (var _0x5e = 0; _0x5e < _0x3c.length; _0x5e++) {
    for (var _0x6f = 0; _0x6f < 2; _0x6f++) _0x4d += _0x3c[_0x5e] * _0x6f;
  }
  var _0x7a = _0x3c.map(function (_0x8b, _0x9c) { return _0x8b + _0x9c; });
  var _0xad = _0x3c.reduce(function (_0xbe, _0xcf) { return _0xbe + _0xcf; }, 0);
  var _0xd0 = _0x3c.slice().sort(function (_0xe1, _0xf2) { return _0xe1 - _0xf2; });
  try { JSON.parse('{'); } catch (_0x13) { _0x4d += _0x13.message.length; }
  var _0x24 = JSON.parse(String(_0x3c.length));
  var { width: _0x35, height: _0x46 } = { width: _0x24, height: 5 };
  var _0x57 = new Map([[1, 2]]);
  var _0x68 = 'a,b'.split(',');
  return [_0x4d, _0x7a, _0xad, _0xd0, _0x35 * _0x46, _0x57.get(1), _0x68];
}
function Factory() {
  var _0x8a = function () { return _0x2b(_0x1a); };
  this.compute = _0x8a;
}
console.log(JSON.stringify(new Factory().compute()));";

const CALLBACK_NAME_CAPTURED: &str = "var item = 'outer', index = 'i', array = 'a';
[1, 2].forEach(function (_0x1, _0x2, _0x3) { console.log(_0x1, _0x2, _0x3.length, item, index, array); });";

const CATCH_NAME_CAPTURED: &str = "function f(err) {
  try { throw 1; } catch (_0x2) { return [_0x2, err]; }
}
console.log(f(5));";

const CATCH_INSIDE_CATCH: &str = "try { throw 1; } catch (_0x1) {
  try { throw 2; } catch (_0x2) { console.log(_0x1, _0x2); }
}";

const ELEMENT_NAME_CAPTURED: &str =
    "var document = { createElement: function (t) { return { tag: t }; } };
var canvas = 'global';
function g() { var _0x3 = document.createElement('canvas'); return _0x3.tag + ' ' + canvas; }
console.log(g());";

const PROPERTY_NAME_CAPTURED: &str = "function run() { return 'outer run'; }
function h() { var _0x4 = function () { return run(); }; this.run = _0x4; return _0x4(); }
console.log(h.call({}));";

const PROPERTY_ON_A_PLAIN_OBJECT: &str = "var o = {};
function h() { var _0x4 = function () { return 1; }; o.XhqXeD = _0x4; return _0x4(); }
console.log(h());";

const TEXT_READ_BY_CONCAT: &str =
    "function _0x1(_0x2) { var _0x3 = '' + _0x4; return _0x3.length + _0x2; }
var _0x4 = function (_0x5) { var _0x6 = _0x5 * 2; return _0x6; };
console.log(_0x1(1), _0x4(2));";

const TEXT_READ_BY_TO_STRING: &str = "function _0x1(_0x2) { return _0x2 + 1; }
var _0x3 = _0x1;
console.log(_0x3.toString().length, Function.prototype.toString.call(function (_0x4) { return _0x4; }).length);";

const TEXT_READ_BY_TEMPLATE: &str = "var _0x7 = 5;
var _0x1 = function (_0x2) { return _0x2 + _0x7; };
console.log(`${_0x1}`.length, String(_0x1).length, /_0x2/.test(_0x1));";

const TEXT_READ_BY_CALLEE: &str =
    "function _0x1(_0x2) { return arguments.callee.toString().length + _0x2; }
console.log(_0x1(1));";

const RESERVED_EVIDENCE: &str =
    "var document = { createElement: function (t) { return { tag: t }; } };
function k() {
  'use strict';
  var _0x1 = document.createElement('var');
  var _0x2 = document.createElement('arguments');
  var _0x3 = document.createElement('eval');
  var _0x4 = function () { return 1; };
  var o = {};
  o.delete = _0x4;
  o.class = _0x4;
  return [_0x1.tag, _0x2.tag, _0x3.tag, o.delete(), arguments.length];
}
console.log(JSON.stringify(k()));";

const SIBLING_COUNTERS: &str =
    "function a() { var s = 0; for (var _0x1 = 0; _0x1 < 3; _0x1++) s += _0x1; return s; }
function b() { var s = 0; for (var _0x2 = 0; _0x2 < 4; _0x2++) s += _0x2; return s; }
console.log(a(), b());";

const COUNTER_CAPTURED: &str = "var fns = [];
for (let _0x1 = 0; _0x1 < 3; _0x1++) fns.push(function () { return _0x1; });
var i = 'outer';
console.log(fns.map(function (_0x2) { return _0x2() + i; }).join());";

const EVAL_SEES_NAMES: &str = "function k(_0x6) { var _0x7 = [1].map(function (_0x8) { return _0x8; }); return eval('_0x6 + _0x7[0]'); }
console.log(k(1));";

const WITH_SEES_NAMES: &str = "var o = { item: 'from with' };
function w(_0x1) { with (o) { return [1].map(function (_0x2) { return item + _0x2; }); } }
console.log(w(0));";

const ANNEX_B_EVIDENCE: &str = "function g() { var _0x1 = [1].map(function (_0x2) { return _0x2; }); { function _0x1() { return 2; } } return typeof _0x1; }
console.log(g());";

const NAME_READ: &str = "var _0x1 = function () {}; var o = {}; o.handler = _0x1;
console.log(_0x1.name, o.handler.name);";

const PROMISE_PARAMS: &str = "new Promise(function (_0x1, _0x2) { _0x1(3); }).then(function (_0x3) { console.log(_0x3); }, function (_0x4) { console.log('no', _0x4); });
Promise.reject(new Error('x')).catch(function (_0x5) { console.log(_0x5.message); });";

const MODULE_EXPORT: &str = "export var _0x1 = [1].map(function (_0x2) { return _0x2 * 2; });
console.log(_0x1);";

fn options(rename: RenameMode, unsafe_transforms: bool) -> Options {
    Options {
        rename,
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn smart(source: &str) -> String {
    Pipeline::new(options(RenameMode::Smart, false))
        .run(source)
        .code()
        .to_string()
}

fn rows(programs: &[(&str, &str)]) -> Vec<String> {
    let Some(node) = node_or_skip() else {
        return Vec::new();
    };
    let programs: Vec<(String, String)> = programs
        .iter()
        .map(|(label, source)| (label.to_string(), source.to_string()))
        .collect();
    let modes: [Mode<'_>; 1] = [("smart", &smart)];
    mismatches(&node, &programs, &modes)
}

#[test]
fn smart_names_bindings_from_their_evidence() {
    let code = smart(EVIDENCE);
    for expected in [
        "for (let i = 0;",
        "for (let j = 0;",
        "function (item, index)",
        "function (acc, item)",
        "function (a, b)",
        "catch (err)",
        "const data = JSON.parse",
        "width, height",
        "const map = new Map",
        "const parts = ",
        "const compute = function",
        "this.compute = compute",
    ] {
        assert!(code.contains(expected), "missing {expected:?} in:\n{code}");
    }
    assert!(!code.contains("_0x"), "{code}");
}

#[test]
fn smart_reuses_a_name_in_sibling_scopes() {
    let code = smart(SIBLING_COUNTERS);
    assert_eq!(code.matches("for (let i = 0;").count(), 2, "{code}");
    assert!(!code.contains("i2"), "{code}");
}

#[test]
fn smart_leaves_human_written_names_and_bare_short_names_alone() {
    let source = "function total(values) { return values.reduce(function (e, t) { return e + t; }, 0) + [1].map(function (n) { return n; })[0]; }\nconsole.log(total([1, 2]));";
    let code = smart(source);
    assert!(code.contains("function total(values)"), "{code}");
    assert!(code.contains("function (acc, item)"), "{code}");
    assert!(code.contains("function (item)"), "{code}");
}

#[test]
fn smart_numbers_a_name_an_outer_binding_already_holds() {
    let code = smart(CALLBACK_NAME_CAPTURED);
    assert!(code.contains("function (item2, index2, array2)"), "{code}");
    let code = smart(CATCH_NAME_CAPTURED);
    assert!(code.contains("catch (err2)"), "{code}");
    let code = smart(ELEMENT_NAME_CAPTURED);
    assert!(code.contains("canvas2"), "{code}");
}

#[test]
fn smart_never_hands_out_a_reserved_word() {
    let code = smart(RESERVED_EVIDENCE);
    for word in [
        "var var ",
        "const var ",
        "const arguments ",
        "const eval ",
        "const delete ",
        "const class ",
    ] {
        assert!(!code.contains(word), "{word:?} in:\n{code}");
    }
}

#[test]
fn smart_keeps_what_eval_can_see() {
    let code = smart(EVAL_SEES_NAMES);
    assert!(code.contains("_0x6") && code.contains("_0x7"), "{code}");
}

#[test]
fn node_agrees_on_evidence_named_output() {
    let wrong = rows(&[
        ("evidence", EVIDENCE),
        ("callback name captured", CALLBACK_NAME_CAPTURED),
        ("catch name captured", CATCH_NAME_CAPTURED),
        ("catch inside catch", CATCH_INSIDE_CATCH),
        ("element name captured", ELEMENT_NAME_CAPTURED),
        ("property name captured", PROPERTY_NAME_CAPTURED),
        ("reserved evidence", RESERVED_EVIDENCE),
        ("sibling counters", SIBLING_COUNTERS),
        ("counter captured", COUNTER_CAPTURED),
        ("eval sees names", EVAL_SEES_NAMES),
        ("with sees names", WITH_SEES_NAMES),
        ("annex b evidence", ANNEX_B_EVIDENCE),
        ("name read", NAME_READ),
        ("promise params", PROMISE_PARAMS),
        ("property on a plain object", PROPERTY_ON_A_PLAIN_OBJECT),
    ]);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_module_keeps_its_exported_name() {
    let code = smart(MODULE_EXPORT);
    assert!(code.contains("_0x1"), "{code}");
    assert!(code.contains("function (item)"), "{code}");
}

#[test]
fn a_function_whose_source_is_read_keeps_every_name_it_holds() {
    let code = smart(TEXT_READ_BY_CONCAT);
    assert!(code.contains("_0x5") && code.contains("_0x6"), "{code}");
    let code = smart(TEXT_READ_BY_TO_STRING);
    assert!(code.contains("_0x2") && code.contains("_0x4"), "{code}");
    let code = smart(TEXT_READ_BY_TEMPLATE);
    assert!(code.contains("_0x2") && code.contains("_0x7"), "{code}");
}

#[test]
fn a_self_read_through_callee_leaves_the_program_unrenamed() {
    let out = Pipeline::new(options(RenameMode::Smart, false)).run(TEXT_READ_BY_CALLEE);
    assert!(out.code().contains("_0x2"), "{}", out.code());
    assert!(
        out.notes
            .iter()
            .any(|note| note.message == untwist_rename::smart::TEXT_READ_UNKNOWN),
        "{:?}",
        out.notes
    );
}

#[test]
fn property_names_name_a_binding_only_on_this_exports_or_a_prototype() {
    let code = smart(PROPERTY_ON_A_PLAIN_OBJECT);
    assert!(!code.contains("xhqXeD"), "{code}");
}

fn unrenamed(source: &str, unsafe_transforms: bool) -> String {
    Pipeline::new(options(RenameMode::None, unsafe_transforms))
        .run(source)
        .code()
        .to_string()
}

fn renaming_changes_behaviour(
    node: &std::path::Path,
    programs: &[(String, String)],
    lanes: &[(&str, bool)],
) -> Vec<String> {
    let scratch = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for (name, source) in programs {
        for (lane, unsafe_transforms) in lanes {
            let before = printed(node, scratch.path(), &unrenamed(source, *unsafe_transforms));
            let renamed = Pipeline::new(options(RenameMode::Smart, *unsafe_transforms))
                .run(source)
                .code()
                .to_string();
            let after = printed(node, scratch.path(), &renamed);
            if before.is_err() || before != after {
                wrong.push(format!("{name} ({lane}): {before:?} became {after:?}"));
            }
        }
    }
    wrong
}

#[test]
fn renaming_never_changes_what_a_read_of_function_source_sees() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = [
        ("text read by concat", TEXT_READ_BY_CONCAT),
        ("text read by toString", TEXT_READ_BY_TO_STRING),
        ("text read by template", TEXT_READ_BY_TEMPLATE),
        ("text read by callee", TEXT_READ_BY_CALLEE),
    ]
    .iter()
    .map(|(label, source)| (label.to_string(), source.to_string()))
    .collect();
    let wrong = renaming_changes_behaviour(&node, &programs, &[("safe", false), ("unsafe", true)]);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn obfuscator_samples_behave_the_same_after_smart_renaming() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = format!(
        "{}/../../fixtures/obfuscator-io",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut programs = Vec::new();
    for prefix in [
        "preset-default",
        "preset-low",
        "cff-1",
        "encoding-rc4",
        "encoding-base64",
        "calls-transform-1",
        "dead-code-1",
        "self-defending",
        "wrappers-function-params-5",
        "split-strings-2",
    ] {
        for sample in ["control-1", "mixed-1", "classes-1", "functions-1"] {
            let name = format!("{prefix}--{sample}.js");
            let source = std::fs::read_to_string(format!("{dir}/{name}")).unwrap();
            programs.push((name, source));
        }
    }
    let wrong = renaming_changes_behaviour(&node, &programs, &[("safe", false), ("unsafe", true)]);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_random_property_name_names_nothing() {
    let source =
        "function F(_0x1) { this.rpGpQJ = _0x1; this.label = _0x1; }\nconsole.log(new F(1).label);";
    let code = smart(source);
    assert!(!code.contains("rpGpQJ = rpGpQJ"), "{code}");
    let source =
        "function Widget(_0x1) { this.onLoad = _0x1; }\nconsole.log(typeof new Widget(1).onLoad);";
    assert!(
        smart(source).contains("function Widget(onLoad)"),
        "{}",
        smart(source)
    );
}
