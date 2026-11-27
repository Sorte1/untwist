use untwist_testing::assert_transform;
use untwist_unminify::split_merged_vars;

#[test]
fn a_redeclared_var_with_reads_on_both_sides_splits() {
    assert_transform!(
        "function f(t) { var o = g(t); h(o); var o = k(t); return o; }",
        [split_merged_vars::boxed],
        @"
    function f(t) {
      var o = g(t);
      h(o);
      var o_2__ = k(t);
      return o_2__;
    }
    "
    );
}

#[test]
fn a_reused_parameter_splits() {
    assert_transform!(
        "function f(t) { t = t.style; t.color = 1; return t; }",
        [split_merged_vars::boxed],
        @"
    function f(t) {
      var t_2__ = t.style;
      t_2__.color = 1;
      return t_2__;
    }
    "
    );
}

#[test]
fn a_captured_binding_stays() {
    assert_transform!(
        "function f(e) { var k = e; var g = function () { return k; }; k = e * 3; return g() + k; }",
        [split_merged_vars::boxed],
        @"
    function f(e) {
      var k = e;
      var g = function () {
        return k;
      };
      k = e * 3;
      return g() + k;
    }
    "
    );
}

#[test]
fn a_mapped_arguments_parameter_stays() {
    assert_transform!(
        "function f(t) { t = t.style; return t + arguments[0]; } function s(t) { 'use strict'; t = t.style; return t + arguments[0]; }",
        [split_merged_vars::boxed],
        @r#"
    function f(t) {
      t = t.style;
      return t + arguments[0];
    }
    function s(t) {
      "use strict";
      var t_2__ = t.style;
      return t_2__ + arguments[0];
    }
    "#
    );
}

#[test]
fn a_block_function_or_catch_parameter_of_the_same_name_stays() {
    assert_transform!(
        "function f(t) { var o = t; g(o); var o = 2; { function o() {} } return o; } function c(t) { var o = t; g(o); o = 2; try {} catch (o) { var o = 3; } return o; }",
        [split_merged_vars::boxed],
        @"
    function f(t) {
      var o = t;
      g(o);
      var o = 2;
      {
        function o() {}
      }
      return o;
    }
    function c(t) {
      var o = t;
      g(o);
      o = 2;
      try {} catch (o) {
        var o = 3;
      }
      return o;
    }
    "
    );
}

#[test]
fn a_write_in_a_nested_block_follows_its_segment() {
    assert_transform!(
        "function f(e) { var k = e; g(k); var k = 5; if (e) { k = 6; } return k; }",
        [split_merged_vars::boxed],
        @"
    function f(e) {
      var k = e;
      g(k);
      var k_2__ = 5;
      if (e) {
        k_2__ = 6;
      }
      return k_2__;
    }
    "
    );
}

#[test]
fn a_lead_split_in_the_same_traversal_renames_every_read() {
    assert_transform!(
        "function f(e) { if ((e = e || {}).a === undefined) e.a = 1; return e.a; }",
        [untwist_unminify::lead_assignment::boxed, untwist_unminify::sequence::boxed, split_merged_vars::boxed],
        @"
    function f(e) {
      var e_2__ = e || {};
      if (e_2__.a === undefined) e_2__.a = 1;
      return e_2__.a;
    }
    "
    );
}
