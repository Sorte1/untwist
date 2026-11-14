use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const PROGRAMS: &[(&str, &str)] = &[
    (
        "f1-class",
        r##"(function(){ var o = {abcde: "x", fghij: "q"}; class K { static m(){ return eval("#!\n(function(){ { function o(){} } return o.abcde })()"); } } console.log(K.m(), o.fghij); })();"##,
    ),
    (
        "f1-hashbang",
        r##"(function(){ "use strict"; var o = {abcde: "x", fghij: "q"}; console.log(eval("#!h\n1; (function(){ { function o(){} } return o.abcde })()"), o.fghij); })();"##,
    ),
    (
        "f1-htmlclose",
        r##"(function(){ "use strict"; var o = {abcde: "x", fghij: "q"}; console.log(eval("--> h\n1; (function(){ { function o(){} } return o.abcde })()"), o.fghij); })();"##,
    ),
    (
        "f1-rename",
        r##""use strict"; (function(){ var value = "x"; console.log(eval("#!\n(function(){ { function value(){} } return value })()")); })();"##,
    ),
    (
        "f2-global",
        r##"with ({undefined: 5}) { console.log(void 0); }
with ({Infinity: 5}) { console.log(1 / 0); }
var p = {String: {fromCharCode: function(){ return "W"; }}, JSON: {parse: function(){ return 9; }}, atob: function(){ return "Z"; }};
with (p) { console.log(String.fromCharCode(65), JSON.parse("[1]"), atob("QQ==")); }"##,
    ),
    (
        "f2-sequence",
        r##"(function(){ var p = {}; var x = 0; with (p) { x += (p.x = 5, 1); } console.log(x, p.x); })();"##,
    ),
    (
        "f3-htmlclose",
        r##"var c = 1, y = 0;
c
--> y
console.log(c);"##,
    ),
    (
        "f3-htmlopen",
        r##"var a = 5, b = 3;
var x = a<!--b
;
console.log(x, b);"##,
    ),
    (
        "f4-ifelse",
        r##"var first = "o", second = "p"; function t(){ { let f = 1; { if (false) ; else function f(){} } } return first + second; } console.log(t());"##,
    ),
    (
        "f4-if",
        r##"var first = "o", second = "p"; function t(){ { let f = 1; { if (true) function f(){} } } return first + second; } console.log(t());"##,
    ),
    (
        "f4-labelled",
        r##"var first = "o", second = "p"; function t(){ { let f = 1; { l: function f(){} } } return first + second; } console.log(t());"##,
    ),
    (
        "f4-mixed",
        r##"var first = "o", second = "p"; function t(){ { function plain(){} } { let f = 1; { l: function f(){} } } return first + second; } console.log(t());"##,
    ),
    (
        "f4-plain",
        r##"var first = "o", second = "p"; function t(){ { let f = 1; { function f(){} } } return first + second; } console.log(t());"##,
    ),
    (
        "f4-target",
        r##"function t(){ var f = 1; { l: function f(){} } return typeof f; } console.log(t());"##,
    ),
    (
        "f4-typeof",
        r##"function t(){ { let f = 1; { l: function f(){} } } return typeof b; } console.log(t());"##,
    ),
    (
        "f5-selfdefending",
        r##"var g = "outer";
(function () { { const g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); const h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h(); { function g() {} } } console.log(typeof g); })();"##,
    ),
    (
        "s2",
        r##"var _0x4a2b = ["hello", "world", "log"];
function _0x1f3c(_0x1, _0x2) { _0x1 = _0x1 - 0; var _0x3 = _0x4a2b[_0x1]; return _0x3; }
with ({_0x1f3c: function(){ return "W"; }}) { console.log(_0x1f3c("0x0"), _0x1f3c("0x1")); }
console.log(_0x1f3c("0x0"));"##,
    ),
    (
        "dc1",
        r##"var f = "outer"; function t(){ if ("abc" === "xyz") { function f(){} } return typeof f; } console.log(t());
function u(){ { let o = 1; if ("abc" === "xyz") { console.log(o); } { function o(){} } } return typeof o; } console.log(u());
function v(){ { if ("abc" !== "xyz") { let o = 1; { function o(){} } } } return typeof o; } console.log(v());"##,
    ),
    (
        "f1-var",
        r##"var f = "outer"; function t(){ if ("abc" === "xyz") { var f = 1; } return typeof f } console.log(t());"##,
    ),
    (
        "f1-forvar",
        r##"var f = "outer"; function t(){ if ("abc" === "xyz") { for (var f of []) { function g(){} } } return typeof f + typeof g } console.log(t());"##,
    ),
    (
        "f2-var-arguments",
        r##"function t(){ var arguments; return typeof arguments + arguments.length } console.log(t(1,2));"##,
    ),
    (
        "f2-unsafe-arguments",
        r##"function t(){ if ("abc" === "xyz") { function arguments(){ return "abc" } } return arguments.length } console.log(t(1, 2));"##,
    ),
    (
        "f3-catch-var",
        r##"(function(){ var e = {abcde: "x", fghij: "q"}; eval("try { throw 1 } catch (e) { var e = 2 }"); console.log(typeof e, e && e.fghij); })();"##,
    ),
    (
        "f3-let",
        r##"(function(){ let e = {abcde: "x", fghij: "q"}; try { eval("try { throw 1 } catch (e) { var e = 2 }"); } catch (x) { console.log(x.name) } console.log(typeof e, e.fghij); })();"##,
    ),
    (
        "f3-rename",
        r##"(function(){ var e = "outer"; (function(){ console.log(typeof e); eval("try { throw 1 } catch (e) { var e = 2 }"); console.log(typeof e); e = 3; })(); console.log(e); })();"##,
    ),
    (
        "f5-selfdefending-eval",
        r##"var g = "outer";
(function () { const g = (function () { let a = true; return function (b, c) { const d = a ? function () { if (c) { const e = c.apply(b, arguments); c = null; return e; } } : function () {}; a = false; return d; }; })(); const h = g(this, function () { return h.toString().search('(((.+)+)+)+$').toString().constructor(h).search('(((.+)+)+)+$'); }); h(); console.log(eval("typeof g")); })();"##,
    ),
    (
        "f5-debug-eval",
        r##"var dpf = "outer";
(function (k) { { const dpf = function (ret) {
  function debuggerProtection(counter) {
    if (typeof counter === "string") {
      return function (arg) {}.constructor("while (true) {}").apply("counter");
    } else if (("" + counter / counter).length !== 1 || counter % 20 === 0) {
      (function () {
        return true;
      }).constructor("debugger").call("action");
    } else {
      (function () {
        return false;
      }).constructor("debugger").apply("stateObject");
    }
    debuggerProtection(++counter);
  }
  try {
    if (ret) {
      return debuggerProtection;
    } else {
      debuggerProtection(0);
    }
  } catch (y) {}
};
 console.log(typeof eval(k)); } })("dpf");"##,
    ),
];

fn run(unsafe_transforms: bool, rename: RenameMode, source: &str) -> String {
    let options = Options {
        unsafe_transforms,
        rename,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn uses_direct_eval_of_a_local(program: &str) -> bool {
    program.contains("eval(")
}

#[test]
fn every_a35r_probe_prints_what_node_prints_in_the_lanes_its_premises_cover() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = PROGRAMS
        .iter()
        .map(|(label, program)| (label.to_string(), program.to_string()))
        .collect();
    let default = |source: &str| run(false, RenameMode::None, source);
    let smart = |source: &str| run(false, RenameMode::Smart, source);
    let mangle = |source: &str| run(false, RenameMode::Mangle, source);
    let simple = |source: &str| run(false, RenameMode::Simple, source);
    let safe_lanes: [Mode<'_>; 4] = [
        ("default", &default),
        ("rename smart", &smart),
        ("rename mangle", &mangle),
        ("rename simple", &simple),
    ];
    let mut wrong = node_rows::mismatches(&node, &programs, &safe_lanes);
    let (direct_eval, static_code): (Vec<_>, Vec<_>) = programs
        .into_iter()
        .partition(|(_, program)| uses_direct_eval_of_a_local(program));
    assert_eq!(direct_eval.len(), 9, "{direct_eval:?}");
    let unsafe_lane = |source: &str| run(true, RenameMode::None, source);
    let unsafe_simple = |source: &str| run(true, RenameMode::Simple, source);
    let unsafe_mangle = |source: &str| run(true, RenameMode::Mangle, source);
    let unsafe_lanes: [Mode<'_>; 3] = [
        ("unsafe", &unsafe_lane),
        ("unsafe rename simple", &unsafe_simple),
        ("unsafe rename mangle", &unsafe_mangle),
    ];
    wrong.extend(node_rows::mismatches(&node, &static_code, &unsafe_lanes));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
