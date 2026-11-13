use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::{transform_with, Allocator};
use untwist_unminify::var_to_let;

fn run(source: &str) -> String {
    let alloc = Allocator::default();
    let ctors: Vec<PassCtor<'_>> = vec![var_to_let::boxed];
    let options = PassOptions {
        unsafe_transforms: true,
        extended: true,
        ..PassOptions::default()
    };
    transform_with(&alloc, source, &ctors, options)
}

fn in_function(body: &str) -> String {
    format!("function f() {{\n{body}\n}}")
}

fn assert_moves(body: &str, expected: &str) {
    assert_eq!(run(&in_function(body)), in_function(expected));
}

fn assert_stays(body: &str) {
    let source = in_function(body);
    assert_eq!(run(&source), source);
}

#[test]
fn a_wall_of_wrapper_flags_moves_next_to_each_wrapper() {
    assert_moves(
        "  var a;\n  var hasA;\n  var b;\n  var hasB;\n  var c = {};\n  function reqA() {\n    if (hasA) {\n      return a;\n    }\n    hasA = 1;\n    return a = 1;\n  }\n  function reqB() {\n    if (hasB) {\n      return b;\n    }\n    hasB = 1;\n    return b = c;\n  }\n  return [reqA, reqB];",
        "  var c = {};\n  var a;\n  var hasA;\n  function reqA() {\n    if (hasA) {\n      return a;\n    }\n    hasA = 1;\n    return a = 1;\n  }\n  var b;\n  var hasB;\n  function reqB() {\n    if (hasB) {\n      return b;\n    }\n    hasB = 1;\n    return b = c;\n  }\n  return [reqA, reqB];",
    );
}

#[test]
fn a_bare_var_already_before_its_first_use_stays() {
    assert_stays("  var a;\n  var b;\n  use(a, b);\n  function h() {\n    return a + b;\n  }");
}

#[test]
fn a_bare_var_merges_into_its_later_declaration() {
    assert_moves(
        "  var a;\n  g();\n  var a = {};\n  use(a);\n  return () => a;",
        "  g();\n  const a = {};\n  use(a);\n  return () => a;",
    );
}

#[test]
fn a_bare_var_redeclared_in_a_block_moves_before_the_block() {
    assert_moves(
        "  var a;\n  g();\n  if (c) {\n    var a = 1;\n  }\n  return h;\n  function h() {\n    return a;\n  }",
        "  g();\n  var a;\n  if (c) {\n    var a = 1;\n  }\n  return h;\n  function h() {\n    return a;\n  }",
    );
}

#[test]
fn a_bare_var_used_before_it_goes_to_its_first_use_at_the_top() {
    assert_moves(
        "  use(a);\n  var a;\n  g();\n  return h;\n  function h() {\n    return a;\n  }",
        "  var a;\n  use(a);\n  g();\n  return h;\n  function h() {\n    return a;\n  }",
    );
}

#[test]
fn an_unused_bare_var_stays() {
    assert_stays("  var a;\n  g();\n  h();");
}

#[test]
fn a_bare_var_in_a_block_stays_in_place() {
    assert_stays("  if (c) {\n    var a;\n    g();\n    h(a);\n  }\n  return a;");
}

#[test]
fn a_script_level_bare_var_stays() {
    let source = "var a;\ng();\nfunction h() {\n  return a;\n}";
    assert_eq!(run(source), source);
}

#[test]
fn a_bare_var_beside_a_direct_eval_stays() {
    assert_stays("  var a;\n  g();\n  eval(\"a = 1\");\n  return h;\n  function h() {\n    return a;\n  }");
}

#[test]
fn a_bare_var_named_like_a_block_function_stays() {
    assert_stays("  var a;\n  g();\n  {\n    function a() {}\n  }\n  return a;");
}

#[test]
fn a_bare_var_naming_a_parameter_stays() {
    let source = "function f(a) {\n  var a;\n  g();\n  return h;\n  function h() {\n    return a;\n  }\n}";
    assert_eq!(run(source), source);
}

#[test]
fn a_bare_var_inside_a_handed_out_guard_stays() {
    let source = "function f() {\n  var a;\n  g();\n  var b = 1;\n  return function () {\n    return a;\n  };\n}";
    assert_eq!(run(source), source);
}

#[test]
fn a_shadowed_name_in_an_inner_function_is_not_a_use() {
    assert_moves(
        "  var a;\n  var g = function (a) {\n    return a;\n  };\n  use(g);\n  return h;\n  function h() {\n    return a;\n  }",
        "  const g = function (a) {\n    return a;\n  };\n  use(g);\n  var a;\n  return h;\n  function h() {\n    return a;\n  }",
    );
}

#[test]
fn a_bare_var_never_moves_past_a_return() {
    assert_moves(
        "  var a;\n  g();\n  return h;\n  function h() {\n    return a;\n  }",
        "  g();\n  var a;\n  return h;\n  function h() {\n    return a;\n  }",
    );
}

#[test]
fn sinking_is_idempotent() {
    let once = run(&in_function(
        "  var a;\n  var b;\n  g();\n  use(b);\n  h();\n  return k;\n  function k() {\n    return a;\n  }",
    ));
    assert_eq!(run(&once), once);
}
