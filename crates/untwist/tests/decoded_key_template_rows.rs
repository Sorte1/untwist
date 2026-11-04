use untwist::{Options, Output, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const DECODER_CALL: &str = "console.log(b(";

const UNIT: &str = "function a() { var h = ['fromCharCode', 'return (function() ', '{}.constructor(\"return this\")( )', 'name', 'log', 'constructor', 'prototype', 'unused', 'String.fromCharCode = ', 'function () { return \"X\"; }', 'j', 'String', 'Str']; a = function () { return h; }; return a(); }
function b(c, d) {
var e = a();
return (b = function (f, g) {
f = f - 0;
var i = e[f];
return String.fromCharCode(i.charCodeAt(0)) + i.slice(1);
}), b(c, d);
}
(function (c, d) {
var e = c();
var tries = 0;
while (!![]) {
try {
if (b(4) === 'log') break;
else e['push'](e['shift']());
} catch (h) { e['push'](e['shift']()); }
if (tries++ > 1000) break;
}
})(a, 0);
";

fn program(body: &str) -> String {
    format!("{UNIT}{body}\nconsole.log(b(3));\n")
}

fn safe_lane(source: &str) -> Output {
    Pipeline::new(Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn decoded(source: &str) -> bool {
    !safe_lane(source).code().contains(DECODER_CALL)
}

const DECODED_SHAPES: [(&str, &str); 4] = [
    (
        "a built-in read by a decoded key",
        "console.log(String[b(0)](104, 105));",
    ),
    (
        "code text concatenated from decoded strings",
        "var g = Function(b(1) + b(2) + \");\")();\nconsole.log(typeof g);",
    ),
    (
        "a concatenated key on this in a class constructor",
        "class C {\nconstructor(v) {\nthis[b(3) + \"x\"] = v;\n}\n}\nconsole.log(new C(1).namex);",
    ),
    (
        "a concatenated decoded key read on the global object",
        "console.log(typeof globalThis[b(3) + \"x\"]);",
    ),
];

const PATCHING_SHAPES: [(&str, &str); 8] = [
    (
        "a built-in key that decodes to constructor",
        "console.log(String[b(5)](\"return 1\")());",
    ),
    (
        "a built-in key that decodes to prototype, then written through",
        "const p = String[b(6)];\np.slice = function () { return \"X\"; };\nconsole.log(\"ab\".slice(1));",
    ),
    (
        "code text that writes a built-in the decoder reads",
        "Function(b(8) + b(9))();\nconsole.log(String.fromCharCode(65));",
    ),
    (
        "code text with a parameter in it",
        "function run(x) { return Function(b(1) + x)(); }\nconsole.log(typeof run(\"1\"));",
    ),
    (
        "code text with a method call in it",
        "var g = Function(b(1) + [b(2)].join(\"\") + \");\")();\nconsole.log(typeof g);",
    ),
    (
        "code text built through a proxy object that is later written",
        "const o = { j: function (x, y) { return x + y; } };\no.j = function (x, y) { return y; };\nvar g = Function(o[b(10)](b(1), b(2)) + \");\")();\nconsole.log(typeof g);",
    ),
    (
        "a class method called with the global object as this",
        "class C {\nm(k, v) {\nthis[k] = v;\n}\n}\nC.prototype.m.call(globalThis, b(11), { fromCharCode: function () { return \"X\"; } });\nconsole.log(String.fromCharCode(65));",
    ),
    (
        "a concatenated global key that spells a root the decoder reads",
        "globalThis[b(12) + \"ing\"] = { fromCharCode: function () { return \"X\"; } };\nconsole.log(String.fromCharCode(65));",
    ),
];

#[test]
fn a_key_or_code_text_built_from_decoded_strings_is_judged_by_its_value() {
    for (label, body) in DECODED_SHAPES {
        let source = program(body);
        assert!(decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn a_decoded_key_or_text_that_may_patch_a_built_in_still_refuses() {
    for (label, body) in PATCHING_SHAPES {
        let source = program(body);
        assert!(!decoded(&source), "{label}:\n{}", safe_lane(&source).code());
    }
}

#[test]
fn every_decoded_shape_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let fixtures: Vec<(String, String)> = DECODED_SHAPES
        .iter()
        .map(|(label, body)| (label.to_string(), program(body)))
        .collect();
    let lane = |source: &str| safe_lane(source).code().to_string();
    let runs: [Mode<'_>; 1] = [("safe", &lane)];
    let wrong = mismatches(&node, &fixtures, &runs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
