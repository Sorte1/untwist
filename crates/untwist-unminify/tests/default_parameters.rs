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
