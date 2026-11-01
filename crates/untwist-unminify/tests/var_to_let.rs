use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::{transform_with, Allocator};
use untwist_unminify::{let_to_const, var_to_let};

fn options(extended: bool) -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended,
        ..PassOptions::default()
    }
}

fn run_with(source: &str, with_let_to_const: bool, extended: bool) -> String {
    let alloc = Allocator::default();
    let mut ctors: Vec<PassCtor<'_>> = vec![var_to_let::boxed];
    if with_let_to_const {
        ctors.insert(0, let_to_const::boxed);
    }
    transform_with(&alloc, source, &ctors, options(extended))
}

fn run(source: &str) -> String {
    run_with(source, false, true)
}

fn in_function(body: &str) -> String {
    format!("function f() {{\n{body}\n}}")
}

fn assert_converts(body: &str, expected: &str) {
    assert_eq!(run(&in_function(body)), in_function(expected));
}

fn assert_stays(body: &str) {
    let source = in_function(body);
    assert_eq!(run(&source), source);
}

#[test]
fn a_constant_var_used_only_after_it_in_its_block_becomes_const() {
    assert_converts("  var a = g();\n  use(a);", "  const a = g();\n  use(a);");
}

#[test]
fn a_block_var_becomes_a_block_const() {
    assert_converts(
        "  if (c) {\n    var a = g();\n    use(a);\n  }",
        "  if (c) {\n    const a = g();\n    use(a);\n  }",
    );
}

#[test]
fn conservative_leaves_every_var_alone() {
    let source = in_function("  var a = g();\n  use(a);");
    assert_eq!(run_with(&source, false, false), source);
}

#[test]
fn a_written_var_becomes_let() {
    assert_converts(
        "  var a = g();\n  a = h(a);\n  use(a);",
        "  let a = g();\n  a = h(a);\n  use(a);",
    );
}

#[test]
fn a_declaration_mixing_a_constant_and_a_written_var_stays() {
    let source = in_function("  var a = g(), b = h();\n  b = 2;\n  use(a, b);");
    assert_eq!(run_with(&source, true, true), source);
}

#[test]
fn a_var_inside_a_self_reading_guard_stays() {
    let source = "var F = function (s) {\n  var A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";\n  var a = \"\";\n  var c = a + F;\n  return c + A + s;\n};";
    assert_eq!(run(source), source);
}

#[test]
fn a_script_level_var_stays_a_global_property() {
    let source = "var a = g();\nuse(a);";
    assert_eq!(run(source), source);
}

#[test]
fn a_read_before_the_declaration_keeps_the_var() {
    assert_stays("  use(a);\n  var a = g();\n  use(a);");
}

#[test]
fn a_typeof_before_the_declaration_keeps_the_var() {
    assert_stays("  use(typeof a);\n  var a = g();\n  use(a);");
}

#[test]
fn a_write_before_the_declaration_keeps_the_var() {
    assert_stays("  a = 1;\n  var a = g();\n  use(a);");
}

#[test]
fn a_read_in_its_own_initialiser_keeps_the_var() {
    assert_stays("  var a = a || {};\n  use(a);");
}

#[test]
fn a_read_outside_the_block_keeps_the_var() {
    assert_stays("  if (c) {\n    var a = g();\n  }\n  use(a);");
}

#[test]
fn a_read_from_a_closure_keeps_the_var() {
    assert_stays("  var a = g();\n  return () => a;");
}

#[test]
fn a_read_from_a_class_field_keeps_the_var() {
    assert_stays("  var a = g();\n  return class {\n    x = a;\n  };");
}

#[test]
fn a_parameter_of_the_same_name_keeps_the_var() {
    let source = "function f(a) {\n  var a = g();\n  use(a);\n}";
    assert_eq!(run(source), source);
}

#[test]
fn a_catch_parameter_of_the_same_name_keeps_the_var() {
    assert_stays("  try {} catch (a) {\n    var a = g();\n    use(a);\n  }");
}

#[test]
fn a_redeclared_var_stays() {
    assert_stays("  var a = g();\n  use(a);\n  var a = h();\n  use(a);");
}

#[test]
fn a_var_without_an_initialiser_in_a_loop_stays() {
    assert_stays("  while (c()) {\n    var a;\n    if (d) a = 1;\n    use(a);\n  }");
}

#[test]
fn a_var_without_an_initialiser_outside_loops_becomes_let() {
    assert_converts(
        "  var a;\n  a = g();\n  use(a);",
        "  let a;\n  a = g();\n  use(a);",
    );
}

#[test]
fn a_never_written_var_without_an_initialiser_stays() {
    assert_stays("  var a;\n  use(a);");
}

#[test]
fn a_for_in_head_never_written_becomes_const() {
    assert_converts(
        "  for (var k in o) {\n    use(k);\n  }",
        "  for (const k in o) {\n    use(k);\n  }",
    );
}

#[test]
fn a_var_with_an_initialiser_in_a_loop_body_becomes_const() {
    assert_converts(
        "  while (c()) {\n    var a = g();\n    use(a);\n  }",
        "  while (c()) {\n    const a = g();\n    use(a);\n  }",
    );
}

#[test]
fn a_for_head_becomes_let() {
    assert_converts(
        "  for (var i = 0; i < 3; i++) use(i);",
        "  for (let i = 0; i < 3; i++) use(i);",
    );
}

#[test]
fn a_for_head_read_after_the_loop_stays() {
    assert_stays("  for (var i = 0; i < 3; i++) use(i);\n  use(i);");
}

#[test]
fn a_labelled_for_head_becomes_let() {
    assert_converts(
        "  outer: for (var i = 0; i < 3; i++) use(i);",
        "  outer: for (let i = 0; i < 3; i++) use(i);",
    );
}

#[test]
fn a_written_for_in_head_becomes_let() {
    assert_converts(
        "  for (var k in o) {\n    k += 1;\n    use(k);\n  }",
        "  for (let k in o) {\n    k += 1;\n    use(k);\n  }",
    );
}

#[test]
fn a_for_of_head_read_by_its_own_iterable_stays() {
    assert_stays("  for (var k of k) use(k);");
}

#[test]
fn a_for_in_head_with_an_initialiser_stays() {
    assert_stays("  for (var k = 0 in o) use(k);");
}

#[test]
fn a_switch_case_var_stays() {
    assert_stays("  switch (c) {\n    case 0:\n      var a = g();\n      use(a);\n  }");
}

#[test]
fn a_block_function_of_the_same_name_keeps_the_var() {
    assert_stays("  var a = g();\n  use(a);\n  {\n    function a() {}\n  }\n  use(a);");
}

#[test]
fn a_block_function_nested_inside_keeps_the_var() {
    assert_stays("  var a = g();\n  {\n    function a() {}\n  }");
}

#[test]
fn a_direct_eval_keeps_every_var() {
    assert_stays("  var a = g();\n  eval(\"a = 2\");\n  use(a);");
}

#[test]
fn a_labelled_var_statement_stays() {
    assert_stays("  l: var a = g();\n  use(a);");
}

#[test]
fn a_var_named_let_stays() {
    assert_stays("  var let = g();\n  use(let);");
}

#[test]
fn a_later_declarator_read_by_an_earlier_one_keeps_both() {
    assert_stays("  var a = b, b = g();\n  use(a, b);");
}
