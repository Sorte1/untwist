use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const HEADER: &str = "\
function sa() { var h = ['log', 'hello', 'world']; sa = function () { return h; }; return sa(); }
function dec(c, d) { var e = sa(); return (dec = function (f, g) { f = f - 0x0; var i = e[f];
  return String.fromCharCode(i.charCodeAt(0)) + i.slice(1); }), dec(c, d); }
var E = function () { return 'X'; };
";

const TAIL: &str = "console.log(dec(0x1), dec(0x2));";

const DECODED: &str = "(\"hello\", \"world\")";

const SPLIT_PATCHES: [(&str, &str); 15] = [
    (
        "split across two parameters",
        "Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
    (
        "split through new Function",
        "new Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
    (
        "split by a comment parameter",
        "Function(\"x\", \"/*\", \"*/ a = (String.fromCharCode = E)\", \"\")();",
    ),
    (
        "split with a body",
        "Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"return 1\")();",
    ),
    (
        "split with a function literal",
        "Function(\"x\", \"a = (0\", \"String).fromCharCode = function () { return 'X'; }\", \"\")();",
    ),
    (
        "split and stored",
        "var q = Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\"); q();",
    ),
    (
        "split inside a template in a parameter",
        "Function(\"x\", \"a = `${0\", \"String.fromCharCode = E}`\", \"\")();",
    ),
    (
        "split inside an eval string",
        "eval('Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")()');",
    ),
    (
        "split inside a setTimeout string",
        "setTimeout('Function(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")()');\n\
         setTimeout(function () { console.log(dec(0x1), dec(0x2)); }, 9);",
    ),
    (
        "split through a function's constructor",
        "(function () {}).constructor(\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
    (
        "split through a spread list",
        "Function(...[\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\"])();",
    ),
    (
        "split through call",
        "Function.call(null, \"x\", \"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
    (
        "split through apply",
        "Function.apply(null, [\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\"])();",
    ),
    (
        "split through Reflect.construct",
        "Reflect.construct(Function, [\"x\", \"a = (0\", \"String).fromCharCode = E\", \"\"])();",
    ),
    (
        "split through a template argument",
        "Function(\"x\", \"a = (0\", `String).fromCharCode = E`, \"\")();",
    ),
];

const FUNCTION_TEXT_PATCHES: [(&str, &str); 4] = [
    (
        "a function argument under a patched toString",
        "Function.prototype.toString = function () { return 'a'; };\n\
         Function(function () {}, \"String.fromCharCode = function () { return 'X'; }\")();",
    ),
    (
        "a function argument with its own toString",
        "var f = function () {}; f.toString = function () { return 'a'; };\n\
         Function(f, \"String.fromCharCode = function () { return 'X'; }\")();",
    ),
    (
        "a function argument with its own toString through new Function",
        "var f = function () {}; f.toString = function () { return 'a'; };\n\
         new Function(f, \"String.fromCharCode = function () { return 'X'; }\")();",
    ),
    (
        "a lone function argument under a patched toString",
        "Function.prototype.toString = function () {\n\
         return \"String.fromCharCode = function () { return 'X'; }\"; };\n\
         Function(function () {})();",
    ),
];

const WHOLE_PATCHES: [(&str, &str); 5] = [
    (
        "whole in the body",
        "Function(\"String.fromCharCode = E\")();",
    ),
    (
        "whole in the body after a parameter",
        "Function(\"a\", \"String.fromCharCode = E\")();",
    ),
    (
        "whole in one parameter",
        "Function(\"x\", \"a = String\", \"b = (a.fromCharCode = E)\", \"\")();",
    ),
    (
        "split through an array index",
        "Function(\"x = 1\", \"a = [0\", \"String][1].fromCharCode = E\", \"\")();",
    ),
    (
        "split from the first parameter",
        "Function(\"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
];

const HARMLESS: [(&str, &str); 3] = [
    (
        "two parameters and a body",
        "Function(\"a\", \"b\", \"return a + b\")(1, 2);",
    ),
    (
        "one parameter list and a body",
        "new Function(\"a, b\", \"return a + b\")(1, 2);",
    ),
    (
        "a line comment that hides the rest of the parameters",
        "Function(\"x //\", \"a = (0\", \"String).fromCharCode = E\", \"\")();",
    ),
];

fn program(patch: &str) -> String {
    format!("{HEADER}{patch}\n{TAIL}\n")
}

fn safe(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn decoded_rows(rows: &[(&str, &str)]) -> Vec<String> {
    rows.iter()
        .filter(|(_, patch)| safe(&program(patch)).contains(DECODED))
        .map(|(label, _)| label.to_string())
        .collect()
}

fn programs(rows: &[(&str, &str)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(label, patch)| (label.to_string(), program(patch)))
        .collect()
}

#[test]
fn a_built_in_patch_split_across_function_arguments_refuses_the_decode() {
    let decoded = decoded_rows(&SPLIT_PATCHES);
    assert!(decoded.is_empty(), "decoded anyway: {decoded:?}");
}

#[test]
fn a_function_argument_turned_into_text_refuses_the_decode() {
    let decoded = decoded_rows(&FUNCTION_TEXT_PATCHES);
    assert!(decoded.is_empty(), "decoded anyway: {decoded:?}");
}

#[test]
fn a_built_in_patch_inside_one_function_argument_refuses_the_decode() {
    let decoded = decoded_rows(&WHOLE_PATCHES);
    assert!(decoded.is_empty(), "decoded anyway: {decoded:?}");
}

#[test]
fn a_function_built_from_harmless_arguments_still_decodes() {
    let decoded = decoded_rows(&HARMLESS);
    assert_eq!(decoded.len(), HARMLESS.len(), "decoded only {decoded:?}");
}

#[test]
fn every_function_constructor_row_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run_safe = |source: &str| safe(source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let rows: Vec<(&str, &str)> = SPLIT_PATCHES
        .iter()
        .chain(&FUNCTION_TEXT_PATCHES)
        .chain(&WHOLE_PATCHES)
        .chain(&HARMLESS)
        .copied()
        .collect();
    let all = programs(&rows);
    let dir = tempfile::tempdir().unwrap();
    let patched: Vec<(&str, &str)> = SPLIT_PATCHES
        .iter()
        .chain(&FUNCTION_TEXT_PATCHES)
        .copied()
        .collect();
    for (label, source) in programs(&patched)
        .into_iter()
        .filter(|(_, source)| !source.contains("setTimeout('"))
    {
        let seen = node_rows::printed(&node, dir.path(), &source).unwrap();
        assert!(
            seen.contains("Xello Xorld"),
            "{label}: node printed {seen:?}"
        );
    }
    let wrong = node_rows::mismatches(&node, &all, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const FOLD_TAIL: &str = "console.log('abc'.charAt(0));";

const FOLD_PATCHES: [(&str, &str); 3] = [
    (
        "a function argument with its own toString",
        "var f = function () {}; f.toString = function () { return 'a'; };\n\
         Function(f, \"String.prototype.charAt = function () { return 'X'; }\")();",
    ),
    (
        "a function argument under a patched toString",
        "Function.prototype.toString = function () { return 'a'; };\n\
         Function(function () {}, \"String.prototype.charAt = function () { return 'X'; }\")();",
    ),
    (
        "a function argument through new Function",
        "var f = function () {}; f.toString = function () { return 'a'; };\n\
         new Function(f, \"String.prototype.charAt = function () { return 'X'; }\")();",
    ),
];

fn fold_programs() -> Vec<(String, String)> {
    FOLD_PATCHES
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLD_TAIL}\n")))
        .collect()
}

#[test]
fn a_function_argument_turned_into_text_keeps_a_built_in_call_unfolded() {
    let folded: Vec<String> = fold_programs()
        .into_iter()
        .filter(|(_, source)| safe(source).contains("console.log(\"a\")"))
        .map(|(label, _)| label)
        .collect();
    assert!(folded.is_empty(), "folded anyway: {folded:?}");
}

#[test]
fn a_function_argument_turned_into_text_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let run_safe = |source: &str| safe(source);
    let modes: [Mode<'_>; 1] = [("default", &run_safe)];
    let programs = fold_programs();
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in &programs {
        let seen = node_rows::printed(&node, dir.path(), source).unwrap();
        assert!(seen.contains("X\n"), "{label}: node printed {seen:?}");
    }
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
