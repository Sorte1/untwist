use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::{assert_transform, insta, transform_with};

fn conservative(source: &str) -> String {
    let alloc = Allocator::default();
    let opts = PassOptions {
        extended: false,
        ..PassOptions::default()
    };
    transform_with(
        &alloc,
        source,
        &[untwist_unminify::default_parameters::boxed],
        opts,
    )
}

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "function f() {\n  var e = (arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : \"foo\");\n  if ((x === undefined)) {\n    x = 1;\n  }\n}\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::default_parameters::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn multiple_default_parameters() {
    assert_transform!(
        r#"function f() {
          var e = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : "foo";
          var f = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @r#"function f(e = "foo", f = 5) {}"#
    );
}

#[test]
fn default_parameter_before_the_last_one() {
    assert_transform!(
        r#"function f() {
          var x = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
          var y = arguments.length > 1 ? arguments[1] : undefined;
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @"function f(x = 1, y) {}"
    );
}

#[test]
fn default_parameter_with_value_true_and_false() {
    assert_transform!(
        r#"function f() {
          var x = arguments.length > 0 && arguments[0] !== undefined && arguments[0];
          var y = arguments.length <= 1 || arguments[1] === undefined || arguments[1];
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @"function f(x = false, y = true) {}"
    );
}

#[test]
fn default_parameter_with_gap_before_the_last_one() {
    assert_transform!(
        r#"function f(e) {
          var x = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : {};
          var y = arguments.length > 5 ? arguments[5] : undefined;
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @"function f(e, x = {}, _param, _param2, _param3, y) {}"
    );
}

#[test]
fn default_destructuring_parameter() {
    assert_transform!(
        r#"function f() {
          let {
            x,
            y
          } = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
          let [z] = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : [];
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @"function f({ x, y } = {}, [z] = []) {}"
    );
}

#[test]
fn default_parameter_loose_under_conservative() {
    insta::assert_snapshot!(
        conservative(r#"function f(x, y) {
          if (x === undefined) { x = 1; }
        }"#),
        @"function f(x = 1, y) {}"
    );
}

#[test]
fn ignore_when_other_statements_are_in_between() {
    insta::assert_snapshot!(
        conservative(r#"function f(x, y) {
          if (x === undefined) { x = 1; }
          y = undefined;
          if (y === undefined) { y = 1; }
        }"#),
        @r#"
function f(x = 1, y) {
  y = undefined;
  if (y === undefined) {
    y = 1;
  }
}
"#
    );
}

#[test]
fn a_loose_default_that_would_shorten_length_stays_a_guard() {
    assert_transform!(
        r#"function f(x, y) {
          if (y === undefined) { y = 1; }
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @r#"
function f(x, y) {
  if (y === undefined) {
    y = 1;
  }
}
"#
    );
}

#[test]
fn a_loose_default_after_an_existing_default_keeps_length_and_is_restored() {
    assert_transform!(
        r#"function f(x = 0, y) {
          if (y === undefined) { y = 1; }
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @"function f(x = 0, y = 1) {}"
    );
}

#[test]
fn a_plain_argument_that_would_grow_length_stays_a_declaration() {
    assert_transform!(
        r#"function f() {
          var a = arguments.length > 0 ? arguments[0] : undefined;
          return a;
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @r#"
function f() {
  var a = arguments.length > 0 ? arguments[0] : undefined;
  return a;
}
"#
    );
}

#[test]
fn a_default_past_a_gap_that_would_grow_length_stays_a_declaration() {
    assert_transform!(
        r#"function f(a) {
          var c = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 9;
          return a + c;
        }"#,
        [untwist_unminify::default_parameters::boxed],
        @r#"
function f(a) {
  var c = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 9;
  return a + c;
}
"#
    );
}

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[untwist_unminify::default_parameters::boxed],
        PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::default_parameters::boxed],
    )
}

const ARROW_READS_OUTER_ARGUMENTS: &str = r#"function outer() {
  return () => {
    var b = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 5;
    return b;
  };
}
console.log(outer(7)(), outer()(), outer(7)(9));"#;

const SLOT_HOLDS_ANOTHER_PARAMETER: &str = r#"function d1(x = 0, a, b) {
  var c = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 5;
  return [b, c];
}
console.log(String(d1(1, 2, 3)), String(d1(1, 2)));"#;

const BODY_HAS_DIRECTIVE: &str = r#"function d2(a) {
  "use strict";
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  return b;
}
console.log(d2(1), d2(1, 2));"#;

const DEFAULT_READS_BODY_VARIABLE: &str = r#"var c = "outer";
function d3(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : c;
  var c = "inner";
  return b;
}
console.log(d3(1), d3(1, 2));"#;

const DEFAULT_AWAITS: &str = r#"async function d4(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : await a;
  return b;
}
d4(Promise.resolve(3)).then((value) => console.log(value));"#;

const NAME_IS_A_PARAMETER: &str = r#"function d5(b) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  return b;
}
console.log(d5(1), d5(1, 2));"#;

const ARGUMENTS_IS_A_PARAMETER: &str = r#"function d8(arguments) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  return b;
}
console.log(d8([1, 2]), d8("x"), d8([1], 9));"#;

const ARGUMENTS_IS_A_BODY_FUNCTION: &str = r#"function d9(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  return b;
  function arguments(p, q) {}
}
console.log(d9(1), d9(1, 7));"#;

const PARAMETER_NAME_IS_REPEATED: &str = r#"function d10(a, a) {
  var b = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : 5;
  return [a, b];
}
console.log(String(d10(1, 2)), String(d10(1, 2, 3)));"#;

const DEFAULT_READS_BLOCK_FUNCTION_NAME: &str = r#"var h = "outer";
function e1(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : h;
  { function h() {} }
  return b;
}
console.log(e1(1));"#;

const CLOSURE_READS_REDECLARED_PARAMETER: &str = r#"function e5(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : function () { return a; };
  var a;
  a = 2;
  return b();
}
console.log(e5(1));"#;

const BODY_FUNCTION_TAKES_THE_NAME: &str = r#"function e6(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  return typeof b;
  function b() {}
}
console.log(e6(1));"#;

const BLOCK_FUNCTION_TAKES_THE_NAME: &str = r#"function e7(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  { function b() {} }
  return typeof b;
}
console.log(e7(1));"#;

const CONSTANT_IS_WRITTEN: &str = r#"function e8(a) {
  const b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  try { b = 6; } catch (error) { return "threw"; }
  return b;
}
console.log(e8(1));"#;

const GETTER_TAKES_NO_PARAMETER: &str = r#"var o = {
  get p() {
    var b = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 5;
    return b;
  }
};
console.log(o.p);"#;

const PATTERN_DEFAULT_CALLS_BODY_FUNCTION: &str = r#"function e9() {
  var { x = h() } = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  return x;
  function h() { return 1; }
}
console.log(e9());"#;

const GENERATOR_DEFAULT_RUNS_CODE: &str = r#"var log = [];
function* e2() {
  var b = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : log.push("default");
  yield b;
}
var it = e2();
log.push("called");
it.next();
console.log(log.join());"#;

const PARAMETER_WRITE_SHOWS_IN_ARGUMENTS: &str = r#"function e3(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  a = 9;
  return [arguments[0], b];
}
console.log(String(e3(1)));"#;

const ARGUMENTS_ESCAPES: &str = r#"function poke(list) { list[0] = 9; }
function e4(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  poke(arguments);
  return [a, b];
}
console.log(String(e4(1)));"#;

const OWN_NAME_IS_SHADOWED_IN_THE_BODY: &str = r#"var make = function Order() {
  var b = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : typeof Order;
  var Order = 1;
  return b;
};
console.log(make());"#;

const RULED_OUT_DECLARATIONS: [&str; 17] = [
    OWN_NAME_IS_SHADOWED_IN_THE_BODY,
    PARAMETER_NAME_IS_REPEATED,
    ARROW_READS_OUTER_ARGUMENTS,
    SLOT_HOLDS_ANOTHER_PARAMETER,
    BODY_HAS_DIRECTIVE,
    DEFAULT_READS_BODY_VARIABLE,
    DEFAULT_AWAITS,
    NAME_IS_A_PARAMETER,
    ARGUMENTS_IS_A_PARAMETER,
    ARGUMENTS_IS_A_BODY_FUNCTION,
    DEFAULT_READS_BLOCK_FUNCTION_NAME,
    CLOSURE_READS_REDECLARED_PARAMETER,
    BODY_FUNCTION_TAKES_THE_NAME,
    BLOCK_FUNCTION_TAKES_THE_NAME,
    CONSTANT_IS_WRITTEN,
    GETTER_TAKES_NO_PARAMETER,
    PATTERN_DEFAULT_CALLS_BODY_FUNCTION,
];

const SAFE_LANE_ONLY_DECLARATIONS: [&str; 3] = [
    GENERATOR_DEFAULT_RUNS_CODE,
    PARAMETER_WRITE_SHOWS_IN_ARGUMENTS,
    ARGUMENTS_ESCAPES,
];

fn assert_declaration_stays(source: &str, output: &str) {
    assert!(output.contains("arguments.length >"), "{output}");
    untwist_testing::node::assert_node_agrees(source, output);
}

#[test]
fn a_declaration_that_means_something_else_as_a_parameter_stays_by_default() {
    for source in RULED_OUT_DECLARATIONS
        .iter()
        .chain(&SAFE_LANE_ONLY_DECLARATIONS)
    {
        assert_declaration_stays(source, &default_lane(source));
    }
}

#[test]
fn a_declaration_that_means_something_else_as_a_parameter_stays_under_unsafe() {
    for source in RULED_OUT_DECLARATIONS {
        assert_declaration_stays(source, &unsafe_lane(source));
    }
}

#[test]
fn an_observed_arguments_object_is_assumed_away_under_unsafe() {
    assert!(unsafe_lane(GENERATOR_DEFAULT_RUNS_CODE)
        .contains(r#"function* e2(b = log.push("default"))"#));
    assert!(unsafe_lane(PARAMETER_WRITE_SHOWS_IN_ARGUMENTS).contains("function e3(a, b = 5)"));
    assert!(unsafe_lane(ARGUMENTS_ESCAPES).contains("function e4(a, b = 5)"));
}

const GUARD_READS_LATER_PARAMETER: &str = r#"function d6(a = 1, b, c) {
  if (b === undefined) { b = c; }
  return b;
}
console.log(d6(1, undefined, 3));"#;

const GUARD_CALLS_BODY_FUNCTION: &str = r#"function d7(a = 1, b) {
  if (b === undefined) { b = h(); }
  function h() { return 1; }
  return b;
}
console.log(d7());"#;

const GUARD_IN_AN_ARROW: &str = r#"var l1 = (a = 1, b) => {
  if (b === undefined) { b = 2; }
  return b;
};
console.log(l1(), l1(1, 3));"#;

const GUARD_AWAITS: &str = r#"async function l3(a = 1, b) {
  if (b === undefined) { b = await a; }
  return b;
}
l3(Promise.resolve(3)).then((value) => console.log(value));"#;

const LATER_DEFAULT_READS_THE_GUARDED_PARAMETER: &str = r#"function l4(a = 1, b, c = b) {
  if (b === undefined) { b = 2; }
  return [b, c];
}
console.log(String(l4()));"#;

const GUARDED_PARAMETER_IS_REDECLARED: &str = r#"function l5(a = () => b, b) {
  if (b === undefined) { b = 2; }
  var b;
  return a();
}
console.log(l5());"#;

const GUARD_READS_ITS_OWN_PARAMETER: &str = r#"function l7(a = 1, b) {
  if (b === undefined) { b = b + "x"; }
  return b;
}
console.log(l7());"#;

const GUARD_READS_BLOCK_FUNCTION_NAME: &str = r#"var h = "outer";
function l8(a = 1, b) {
  if (b === undefined) { b = h; }
  { function h() {} }
  return b;
}
console.log(l8());"#;

const GENERATOR_GUARD_RUNS_CODE: &str = r#"var log = [];
function* l6(a = 1, b) {
  if (b === undefined) { b = log.push("default"); }
  yield b;
}
var it = l6();
log.push("called");
it.next();
console.log(log.join());"#;

const RULED_OUT_GUARDS: [&str; 8] = [
    GUARD_READS_LATER_PARAMETER,
    GUARD_CALLS_BODY_FUNCTION,
    GUARD_IN_AN_ARROW,
    GUARD_AWAITS,
    LATER_DEFAULT_READS_THE_GUARDED_PARAMETER,
    GUARDED_PARAMETER_IS_REDECLARED,
    GUARD_READS_ITS_OWN_PARAMETER,
    GUARD_READS_BLOCK_FUNCTION_NAME,
];

fn assert_guard_stays(source: &str, output: &str) {
    assert!(output.contains("if (b === undefined)"), "{output}");
    untwist_testing::node::assert_node_agrees(source, output);
}

#[test]
fn a_guard_that_means_something_else_as_a_default_stays_by_default() {
    for source in RULED_OUT_GUARDS {
        assert_guard_stays(source, &default_lane(source));
    }
    assert_guard_stays(
        GENERATOR_GUARD_RUNS_CODE,
        &default_lane(GENERATOR_GUARD_RUNS_CODE),
    );
}

#[test]
fn a_guard_that_means_something_else_as_a_default_stays_under_unsafe() {
    for source in RULED_OUT_GUARDS {
        assert_guard_stays(source, &unsafe_lane(source));
    }
}

#[test]
fn a_guard_under_a_directive_stays_a_guard() {
    let source = r#"function l2(b) {
  "use strict";
  if (b === undefined) { b = 2; }
  return b;
}
console.log(l2(), l2(3));"#;
    assert_guard_stays(source, &conservative(source));
}

#[test]
fn transpiler_output_still_becomes_default_parameters_by_default() {
    let rows = [
        (
            r#"function f(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : 5;
  var c = arguments.length > 2 && arguments[2] !== undefined ? arguments[2] : a;
  return [a, b, c];
}
console.log(String(f(1)), String(f(1, 2, 3)));"#,
            "function f(a, b = 5, c = a) {",
        ),
        (
            r#"function g() {
  var x = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : 1;
  var y = arguments.length > 1 ? arguments[1] : undefined;
  return [x, y];
}
console.log(String(g()), String(g(4, 5)));"#,
            "function g(x = 1, y) {",
        ),
        (
            r#"function h() {
  let { x, y } = arguments.length > 0 && arguments[0] !== undefined ? arguments[0] : {};
  return [x, y];
}
console.log(String(h()), String(h({ x: 1, y: 2 })));"#,
            "function h({ x, y } = {}) {",
        ),
        (
            r#"function noop() { return "none"; }
function k(options) {
  var done = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : noop;
  var flag = arguments.length > 2 && arguments[2] !== undefined && arguments[2];
  return [done(options), flag];
}
console.log(String(k(1)), String(k(1, (value) => value, true)));"#,
            "function k(options, done = noop, flag = false) {",
        ),
        (
            r#"class A {
  constructor() { this.d = 4; }
  m(a) {
    var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : this.d;
    return [a, b];
  }
}
console.log(String(new A().m(1)), String(new A().m(1, 2)));"#,
            "m(a, b = this.d) {",
        ),
        (
            r#"var make = function Order(a) {
  var b = arguments.length > 1 && arguments[1] !== undefined ? arguments[1] : Order.fallback;
  return [a, b];
};
make.fallback = "xyz";
console.log(String(make(1)), String(make(1, 2)));"#,
            "function Order(a, b = Order.fallback) {",
        ),
        (
            r#"function t(x = 0, y) {
  if (y === undefined) { y = x + 1; }
  return [x, y];
}
console.log(String(t()), String(t(1, 5)));"#,
            "function t(x = 0, y = x + 1) {",
        ),
        (
            r#"function u(x = 0, y, z) {
  if (y === undefined) { y = "y"; }
  if (z === undefined) { z = y + "z"; }
  return [x, y, z];
}
console.log(String(u()), String(u(1, 2, 3)));"#,
            r#"function u(x = 0, y = "y", z = y + "z") {"#,
        ),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}
