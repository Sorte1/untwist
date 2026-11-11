use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{mismatches, node_or_skip, Mode};

const REVERSING_HEAD: &str = r#"function evil() { return ["x"]; }
var c = 1, q, r, s;
function arr() {
  var h = ['gol', 'olleh', 'dlrow', 'cba', 'fed'];
  arr = function () { return h; };
  return arr();
}
function dec(a, b) {
  a = a - 100;
  var e = arr();
  return e[a].split("").reverse().join("");
}
"#;

const PLAIN_HEAD: &str = r#"function evil() { return "EVIL"; }
var c = 1, k = "de";
function arr() {
  var h = ['gol', 'olleh', 'dlrow', 'cba', 'fed'];
  arr = function () { return h; };
  return arr();
}
function dec(a, b) {
  a = a - 100;
  var e = arr();
  return e[a];
}
"#;

const PLAIN_ROWS: [(&str, &str); 2] = [
    (
        "decoder replaced through the global object in the test",
        "console.log(dec((globalThis.dec = evil, c) ? 102 : 103));",
    ),
    (
        "decoder replaced by a helper given this in the test",
        "function patch(o) { o[k + \"c\"] = evil; }\nconsole.log(dec((patch(this), c) ? 102 : 103));",
    ),
];

const REVERSING_ROWS: [(&str, &str); 10] = [
    (
        "key field written in the test",
        "const o = { k: \"zzz\" };\nvar P = String.prototype;\nconsole.log(dec((o.k = \"split\", c) ? 102 : 103));\nP[o.k] = evil;\nconsole.log(dec(101));",
    ),
    (
        "code text written in the test",
        "const o = { k: \"1\" };\nconsole.log(dec((o.k = \"dec = evil\", c) ? 102 : 103));\nFunction(o.k)();\nconsole.log(dec(101));",
    ),
    (
        "code text for new Function written in the test",
        "const o = { k: \"1\" };\nconsole.log(dec((o.k = \"return dec = evil\", c) ? 102 : 103));\nnew Function(o.k)();\nconsole.log(dec(101));",
    ),
    (
        "decoder name written in the test",
        "const o = { k: \"zzz\" };\nfunction patch(g) { g[o.k] = evil; }\nconsole.log(dec((o.k = \"dec\", c) ? 102 : 103));\npatch(globalThis);\nconsole.log(dec(101));",
    ),
    (
        "object escapes to a writer in the test",
        "const o = { k: \"zzz\" };\nvar P = String.prototype;\nfunction mut(x) { x.k = \"split\"; }\nconsole.log(dec((mut(o), c) ? 102 : 103));\nP[o.k] = evil;\nconsole.log(dec(101));",
    ),
    (
        "call site of a patcher in the test",
        "function patch(g) { g[\"de\" + \"c\"] = evil; } patch({});\nconsole.log(dec((patch(globalThis), c) ? 102 : 103));\nconsole.log(dec(101));",
    ),
    (
        "object aliased in the test",
        "const o = { k: \"zzz\" }; var P = String.prototype;\nconsole.log(dec((q = o, c) ? 102 : 103));\nq.k = \"split\"; P[o.k] = evil;\nconsole.log(dec(101));",
    ),
    (
        "field deleted in the test",
        "const o = { k: \"zzz\" }; var P = String.prototype; Object.prototype.k = \"split\";\nconsole.log(dec((delete o.k, c) ? 102 : 103));\nP[o.k] = evil;\nconsole.log(dec(101));",
    ),
    (
        "key field written in the test, prototype patched by a helper",
        "const o = { k: \"zzz\" }; function patch(g) { g[o.k] = evil; }\nconsole.log(dec((o.k = \"split\", c) ? 102 : 103));\npatch(String.prototype);\nconsole.log(dec(101));",
    ),
    (
        "key field written in the test, prototype read through __proto__",
        "const o = { k: \"zzz\" }; var P = \"\".__proto__;\nconsole.log(dec((o.k = \"split\", c) ? 102 : 103));\nP[o.k] = evil;\nconsole.log(dec(101));",
    ),
];

const HARMLESS_ROWS: [(&str, &str); 3] = [
    (
        "plain test",
        "console.log(dec(c ? 102 : 103));\nconsole.log(dec(101));",
    ),
    (
        "sequence test without writes",
        "console.log(dec((q, c) ? 102 : 103));\nconsole.log(dec(101));",
    ),
    (
        "nested branches",
        "console.log(dec(c ? (r ? 101 : 102) : 103));\nconsole.log(dec(104));",
    ),
];

fn transformed(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn programs(head: &str, rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, tail)| (label.to_string(), format!("{head}{tail}\n")))
        .collect()
}

fn hostile_programs() -> Vec<(String, String)> {
    let mut all = programs(PLAIN_HEAD, &PLAIN_ROWS);
    all.extend(programs(REVERSING_HEAD, &REVERSING_ROWS));
    all
}

fn agrees_with_node(programs: &[(String, String)]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run = |source: &str| transformed(source);
    let modes: [Mode<'_>; 1] = [("default", &run)];
    let wrong = mismatches(&node, programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_test_that_can_replace_the_decoder_or_its_inputs_keeps_the_call_whole() {
    for (label, source) in hostile_programs() {
        let output = transformed(&source);
        assert!(
            output.contains("dec(("),
            "{label}: the ternary call was rewritten:\n{output}"
        );
        assert!(
            !output.contains("? dec(") && !output.contains("? \""),
            "{label}: the ternary call was split or decoded:\n{output}"
        );
    }
}

#[test]
fn a_test_that_can_replace_the_decoder_or_its_inputs_prints_what_node_prints() {
    agrees_with_node(&hostile_programs());
}

#[test]
fn a_harmless_ternary_argument_is_decoded_in_both_branches() {
    for (label, source) in programs(REVERSING_HEAD, &HARMLESS_ROWS) {
        let output = transformed(&source);
        assert!(
            !output.contains("dec("),
            "{label}: a decoder call was left:\n{output}"
        );
    }
    agrees_with_node(&programs(REVERSING_HEAD, &HARMLESS_ROWS));
}
