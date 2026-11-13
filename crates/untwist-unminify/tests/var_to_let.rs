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
fn a_read_from_a_later_closure_in_the_function_body_becomes_const() {
    assert_converts(
        "  var a = g();\n  return () => a;",
        "  const a = g();\n  return () => a;",
    );
}

#[test]
fn a_read_from_a_later_function_expression_becomes_const() {
    assert_converts(
        "  var a = g();\n  var h = function () {\n    return a;\n  };\n  return h;",
        "  const a = g();\n  const h = function () {\n    return a;\n  };\n  return h;",
    );
}

#[test]
fn a_closure_written_var_becomes_let() {
    assert_converts(
        "  var a = 0;\n  return () => a++;",
        "  let a = 0;\n  return () => a++;",
    );
}

#[test]
fn a_read_from_a_hoisted_function_declaration_keeps_the_var() {
    assert_stays("  var a = g();\n  return h;\n  function h() {\n    return a;\n  }");
}

#[test]
fn a_read_from_a_closure_in_its_own_initialiser_keeps_the_var() {
    assert_stays("  var a = run(() => a);\n  use(a);");
}

#[test]
fn a_read_from_a_closure_before_the_declaration_keeps_the_var() {
    assert_converts(
        "  var h = () => a;\n  var a = g();\n  return h;",
        "  const h = () => a;\n  var a = g();\n  return h;",
    );
}

#[test]
fn a_read_from_a_closure_in_a_block_keeps_the_var() {
    assert_stays("  if (c) {\n    var a = g();\n    fs.push(() => a);\n  }");
}

#[test]
fn a_read_from_a_closure_in_a_loop_body_keeps_the_var() {
    assert_stays("  while (c()) {\n    var a = g();\n    fs.push(() => a);\n  }");
}

#[test]
fn a_read_from_a_closure_inside_a_later_loop_becomes_const() {
    assert_converts(
        "  var a = g();\n  for (const k of ks) {\n    fs.push(() => a + k);\n  }",
        "  const a = g();\n  for (const k of ks) {\n    fs.push(() => a + k);\n  }",
    );
}

#[test]
fn a_read_from_a_later_class_keeps_the_var() {
    assert_stays("  var a = g();\n  return class {\n    m() {\n      return a;\n    }\n  };");
}

#[test]
fn a_parameter_default_closure_does_not_see_the_body_var() {
    let source = "function f(p = () => a) {\n  var a = g();\n  return p;\n}";
    let expected = "function f(p = () => a) {\n  const a = g();\n  return p;\n}";
    assert_eq!(run(source), expected);
}

#[test]
fn a_for_head_read_from_a_closure_keeps_the_var() {
    assert_stays("  for (var i = 0; i < 3; i++) fs.push(() => i);");
}

#[test]
fn a_captured_var_with_a_direct_eval_stays() {
    assert_stays("  var a = g();\n  eval(\"\");\n  return () => a;");
}

#[test]
fn a_captured_var_named_like_a_block_function_stays() {
    assert_stays("  var a = g();\n  {\n    function a() {}\n  }\n  return () => a;");
}

#[test]
fn a_captured_var_in_a_labelled_statement_stays() {
    assert_stays("  l: var a = g();\n  return () => a;");
}

#[test]
fn a_captured_var_in_a_generator_body_becomes_const() {
    let source = "function* f() {\n  var a = yield 1;\n  yield () => a;\n}";
    let expected = "function* f() {\n  const a = yield 1;\n  yield () => a;\n}";
    assert_eq!(run(source), expected);
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

#[test]
fn a_trailing_var_written_once_in_a_block_moves_to_that_write() {
    assert_converts(
        "  if (c) {\n    a = g();\n    use(a);\n  }\n  return;\n  var a;",
        "  if (c) {\n    const a = g();\n    use(a);\n  }\n  return;",
    );
}

#[test]
fn a_trailing_var_written_again_after_its_first_write_becomes_let() {
    assert_converts(
        "  a = g();\n  a = h(a);\n  return a;\n  var a;",
        "  let a = g();\n  a = h(a);\n  return a;",
    );
}

#[test]
fn the_core_js_get_built_in_shape_moves_into_its_branch() {
    let source = "var f = function (k, n) {\n  if (arguments.length < 2) {\n    v = m[k];\n    if (ok(v)) {\n      return v;\n    } else {\n      return undefined;\n    }\n  } else {\n    return m[k] && m[k][n];\n  }\n  var v;\n};";
    let expected = "var f = function (k, n) {\n  if (arguments.length < 2) {\n    const v = m[k];\n    if (ok(v)) {\n      return v;\n    } else {\n      return undefined;\n    }\n  } else {\n    return m[k] && m[k][n];\n  }\n};";
    assert_eq!(run(source), expected);
}

#[test]
fn a_trailing_var_assigned_in_two_branches_moves_to_the_top() {
    assert_converts(
        "  if (c) {\n    a = 1;\n  } else {\n    a = 2;\n  }\n  return a;\n  var a;",
        "  let a;\n  if (c) {\n    a = 1;\n  } else {\n    a = 2;\n  }\n  return a;",
    );
}

#[test]
fn a_trailing_var_read_before_its_first_write_moves_to_the_top() {
    assert_converts(
        "  use(a);\n  a = 1;\n  return a;\n  var a;",
        "  let a;\n  use(a);\n  a = 1;\n  return a;",
    );
}

#[test]
fn a_trailing_var_typeof_before_its_write_moves_to_the_top() {
    assert_converts(
        "  use(typeof a);\n  a = 1;\n  return a;\n  var a;",
        "  let a;\n  use(typeof a);\n  a = 1;\n  return a;",
    );
}

#[test]
fn a_trailing_var_read_by_its_own_write_moves_to_the_top() {
    assert_converts(
        "  a = a || {};\n  return a;\n  var a;",
        "  let a;\n  a = a || {};\n  return a;",
    );
}

#[test]
fn a_trailing_var_captured_by_a_closure_in_a_loop_moves_to_the_top() {
    assert_converts(
        "  for (i = 0; i < 3; i++) {\n    a = i;\n    fs.push(() => a);\n  }\n  return fs;\n  var a;",
        "  let a;\n  for (i = 0; i < 3; i++) {\n    a = i;\n    fs.push(() => a);\n  }\n  return fs;",
    );
}

#[test]
fn a_trailing_var_written_in_a_loop_body_becomes_a_body_const() {
    assert_converts(
        "  while (c()) {\n    a = g();\n    use(a);\n  }\n  var a;",
        "  while (c()) {\n    const a = g();\n    use(a);\n  }",
    );
}

#[test]
fn a_trailing_var_read_after_its_loop_moves_to_the_top() {
    assert_converts(
        "  while (c()) {\n    a = g();\n  }\n  use(a);\n  var a;",
        "  let a;\n  while (c()) {\n    a = g();\n  }\n  use(a);",
    );
}

#[test]
fn a_trailing_var_used_across_switch_cases_moves_to_the_top() {
    assert_converts(
        "  switch (k) {\n    case 0:\n      a = 1;\n    case 1:\n      use(a);\n  }\n  var a;",
        "  let a;\n  switch (k) {\n    case 0: a = 1;\n    case 1: use(a);\n  }",
    );
}

#[test]
fn a_trailing_var_within_one_switch_case_moves_to_the_top() {
    assert_converts(
        "  switch (k) {\n    case 0:\n      a = 1;\n      use(a);\n  }\n  var a;",
        "  let a;\n  switch (k) {\n    case 0:\n      a = 1;\n      use(a);\n  }",
    );
}

#[test]
fn a_trailing_var_that_is_also_a_parameter_stays() {
    let source = "function f(a) {\n  a = 1;\n  return arguments[0];\n  var a;\n}";
    assert_eq!(run(source), source);
}

#[test]
fn a_trailing_var_redeclared_stays() {
    assert_stays("  a = 1;\n  var a;\n  use(a);\n  var a;");
}

#[test]
fn a_trailing_var_with_a_direct_eval_stays() {
    assert_stays("  a = 1;\n  eval(\"use(a)\");\n  var a;");
}

#[test]
fn a_trailing_var_named_like_a_block_function_stays() {
    assert_stays("  a = 1;\n  {\n    function a() {}\n  }\n  use(a);\n  var a;");
}

#[test]
fn a_trailing_var_written_in_a_try_and_read_in_its_finally_moves_to_the_top() {
    assert_converts(
        "  try {\n    a = g();\n  } finally {\n    use(a);\n  }\n  var a;",
        "  let a;\n  try {\n    a = g();\n  } finally {\n    use(a);\n  }",
    );
}

#[test]
fn a_trailing_var_inside_a_generator_moves_to_its_write() {
    let source = "function* f() {\n  if (c) {\n    a = yield 1;\n    yield a;\n  }\n  var a;\n}";
    let expected = "function* f() {\n  if (c) {\n    const a = yield 1;\n    yield a;\n  }\n}";
    assert_eq!(run(source), expected);
}

#[test]
fn a_trailing_var_in_a_text_read_function_stays() {
    let source = "var F = function (s) {\n  A = \"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=\";\n  var a = \"\";\n  var c = a + F;\n  return c + A + s;\n  var A;\n};";
    assert_eq!(run(source), source);
}

#[test]
fn a_script_level_trailing_var_stays() {
    let source = "a = 1;\nuse(a);\nvar a;";
    assert_eq!(run(source), source);
}

#[test]
fn a_catch_parameter_of_the_same_name_does_not_block_a_later_write() {
    assert_converts(
        "  try {} catch (a) {\n    b = a;\n  }\n  a = 1;\n  use(a, b);\n  var a, b;",
        "  let b;\n  try {} catch (a) {\n    b = a;\n  }\n  const a = 1;\n  use(a, b);",
    );
}

#[test]
fn a_leading_bare_var_is_left_to_the_ordinary_conversion() {
    assert_converts(
        "  var a;\n  if (c) {\n    a = 1;\n  }\n  use(a);",
        "  let a;\n  if (c) {\n    a = 1;\n  }\n  use(a);",
    );
}

#[test]
fn a_trailing_var_captured_by_a_closure_made_after_its_write_moves_to_the_write() {
    assert_converts(
        "  i = (n) => n * 2;\n  const u = function (n) {\n    return i(n);\n  };\n  return u;\n  var i;",
        "  const i = (n) => n * 2;\n  const u = function (n) {\n    return i(n);\n  };\n  return u;",
    );
}

#[test]
fn a_trailing_var_captured_by_a_hoisted_function_moves_to_the_top() {
    assert_converts(
        "  a = 1;\n  return h;\n  function h() {\n    return a;\n  }\n  var a;",
        "  var a;\n  a = 1;\n  return h;\n  function h() {\n    return a;\n  }",
    );
}

#[test]
fn a_trailing_var_captured_inside_a_loop_body_block_moves_to_the_top() {
    assert_converts(
        "  while (c()) {\n    if (d) {\n      a = g();\n      fs.push(() => a);\n    }\n  }\n  var a;",
        "  let a;\n  while (c()) {\n    if (d) {\n      a = g();\n      fs.push(() => a);\n    }\n  }",
    );
}

#[test]
fn a_trailing_var_captured_by_a_closure_in_a_loop_after_its_write_moves_to_the_write() {
    assert_converts(
        "  a = g();\n  for (const k of ks) {\n    fs.push(() => a + k);\n  }\n  var a;",
        "  const a = g();\n  for (const k of ks) {\n    fs.push(() => a + k);\n  }",
    );
}
