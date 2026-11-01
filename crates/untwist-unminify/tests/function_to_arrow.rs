use untwist_core::pass::PassOptions;
use untwist_testing::{transform_with, Allocator};
use untwist_unminify::function_to_arrow;

fn run_with(source: &str, unsafe_transforms: bool, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[function_to_arrow::boxed],
        PassOptions {
            unsafe_transforms,
            extended,
            ..PassOptions::default()
        },
    )
}

fn in_function(body: &str) -> String {
    format!("function f() {{\n{body}\n}}")
}

fn assert_safe(body: &str, expected: &str) {
    assert_eq!(
        run_with(&in_function(body), false, true),
        in_function(expected)
    );
}

fn assert_stays(body: &str, unsafe_transforms: bool) {
    let source = in_function(body);
    assert_eq!(run_with(&source, unsafe_transforms, true), source);
}

#[test]
fn a_binding_only_ever_called_becomes_an_arrow() {
    assert_safe(
        "  var g = function (a) {\n    return a + 1;\n  };\n  return g(1) + g(2);",
        "  var g = (a) => {\n    return a + 1;\n  };\n  return g(1) + g(2);",
    );
}

#[test]
fn a_call_before_the_declaration_still_counts_as_a_call() {
    assert_safe(
        "  function h() {\n    return g();\n  }\n  var g = function () {\n    return 1;\n  };\n  return h();",
        "  function h() {\n    return g();\n  }\n  var g = () => {\n    return 1;\n  };\n  return h();",
    );
}

#[test]
fn an_unused_own_name_is_dropped() {
    assert_safe(
        "  var g = function named() {\n    return 1;\n  };\n  return g();",
        "  var g = () => {\n    return 1;\n  };\n  return g();",
    );
}

#[test]
fn conservative_leaves_every_function_alone() {
    let source = in_function("  var g = function () {\n    return 1;\n  };\n  return g();");
    assert_eq!(run_with(&source, true, false), source);
}

#[test]
fn a_constructed_binding_stays() {
    assert_stays("  var g = function () {};\n  return new g();", true);
}

#[test]
fn a_binding_read_as_a_value_stays() {
    assert_stays("  var g = function () {};\n  return g.prototype;", true);
}

#[test]
fn a_binding_passed_along_stays() {
    assert_stays("  var g = function () {};\n  use(g);", true);
}

#[test]
fn a_reassigned_binding_stays() {
    assert_stays(
        "  var g = function () {};\n  g = function () {};\n  return g();",
        true,
    );
}

#[test]
fn a_function_using_this_stays() {
    assert_stays(
        "  var g = function () {\n    return this;\n  };\n  return g();",
        true,
    );
}

#[test]
fn a_function_with_an_arrow_using_this_stays() {
    assert_stays(
        "  var g = function () {\n    return () => this;\n  };\n  return g()();",
        true,
    );
}

#[test]
fn a_function_using_arguments_stays() {
    assert_stays(
        "  var g = function () {\n    return arguments.length;\n  };\n  return g();",
        true,
    );
}

#[test]
fn a_function_using_new_target_stays() {
    assert_stays(
        "  var g = function () {\n    return new.target;\n  };\n  return g();",
        true,
    );
}

#[test]
fn a_generator_stays() {
    assert_stays("  var g = function* () {};\n  return g();", true);
}

#[test]
fn a_function_calling_its_own_name_stays() {
    assert_stays(
        "  var g = function h(n) {\n    return n && h(n - 1);\n  };\n  return g(2);",
        true,
    );
}

#[test]
fn duplicate_parameters_stay() {
    assert_stays(
        "  var g = function (a, a) {\n    return a;\n  };\n  return g(1, 2);",
        true,
    );
}

#[test]
fn a_function_with_a_nested_function_using_this_becomes_an_arrow() {
    assert_safe(
        "  var g = function () {\n    return function () {\n      return this;\n    };\n  };\n  return g();",
        "  var g = () => {\n    return function () {\n      return this;\n    };\n  };\n  return g();",
    );
}

#[test]
fn a_script_level_binding_stays() {
    let source = "var g = function () {};\ng();";
    assert_eq!(run_with(source, true, true), source);
}

#[test]
fn a_direct_eval_keeps_every_function() {
    assert_stays(
        "  var g = function () {};\n  eval(\"new g()\");\n  return g();",
        true,
    );
}

#[test]
fn a_callback_becomes_an_arrow_only_when_unsafe() {
    let body = "  return list.map(function (x) {\n    return x + 1;\n  });";
    assert_stays(body, false);
    assert_eq!(
        run_with(&in_function(body), true, true),
        in_function("  return list.map((x) => {\n    return x + 1;\n  });")
    );
}

#[test]
fn a_named_callback_stays() {
    assert_stays(
        "  return list.map(function named(x) {\n    return x;\n  });",
        true,
    );
}

#[test]
fn a_callback_using_this_stays() {
    assert_stays(
        "  return list.map(function (x) {\n    return this[x];\n  });",
        true,
    );
}

#[test]
fn a_constructed_callee_argument_stays() {
    assert_stays("  return new Thing(function () {});", true);
}
