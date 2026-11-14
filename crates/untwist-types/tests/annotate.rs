use untwist_types::{annotate, Mode};

fn ts(source: &str) -> String {
    annotate(source, Mode::Ts).code
}

fn jsdoc(source: &str) -> String {
    annotate(source, Mode::Jsdoc).code
}

#[test]
fn a_closed_function_gets_its_parameter_and_return_types() {
    let out = ts("(function () {\n  function twice(n) {\n    return n * 2;\n  }\n  console.log(twice(1), twice(2));\n})();\n");
    assert!(out.contains("function twice(n: 1 | 2): number"), "{out}");
}

#[test]
fn a_let_with_a_closed_write_set_is_a_union() {
    let out = ts("(function () {\n  let mode = \"idle\";\n  function go() {\n    mode = \"run\";\n  }\n  go();\n  console.log(mode.length);\n})();\n");
    assert!(
        out.contains("let mode: \"idle\" | \"run\" = \"idle\""),
        "{out}"
    );
}

#[test]
fn a_const_literal_is_redundant() {
    let source = "(function () {\n  const a = 1;\n  console.log(a);\n})();\n";
    assert_eq!(ts(source), source);
}

#[test]
fn direct_eval_refuses_the_file() {
    let source = "(function () {\n  function f(n) {\n    return n * 2;\n  }\n  eval(\"\");\n  f(1);\n})();\n";
    assert_eq!(ts(source), source);
}

#[test]
fn a_function_reading_arguments_keeps_its_parameters_bare() {
    let out = ts("(function () {\n  function f(n) {\n    arguments[0] = \"s\";\n    return n;\n  }\n  f(1);\n})();\n");
    assert!(!out.contains("n: "), "{out}");
}

#[test]
fn jsdoc_mode_writes_comments() {
    let out = jsdoc("(function () {\n  function twice(n) {\n    return n * 2;\n  }\n  console.log(twice(1));\n})();\n");
    assert!(
        out.contains("/** @param {1} n @returns {number} */ function twice(n)"),
        "{out}"
    );
}
