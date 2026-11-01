mod common;

use oxc_allocator::Allocator;
use untwist_testing::assert_transform;

const NON_CONSTANT_KEY_NOTE: &str =
    "control-flow object not inlined: a computed key is not a compile-time constant string";

#[test]
fn a_called_inline_object_function_is_inlined() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return n === r;\n  }\n}).QuFtJ(u, undefined);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"u === undefined;"
    );
}

#[test]
fn an_uncalled_inline_object_function_becomes_the_function() {
    assert_transform!(
        "a = ({\n  QuFtJ: function QuFtJ(n, r) {\n    return n === r;\n  }\n}).QuFtJ;",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    a = function QuFtJ(n, r) {
      return n === r;
    };
    "#
    );
}

#[test]
fn an_anonymous_uncalled_inline_object_function_is_left_alone() {
    assert_transform!(
        "a = ({\n  QuFtJ: function (n, r) {\n    return n === r;\n  }\n}).QuFtJ;",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    a = { QuFtJ: function (n, r) {
      return n === r;
    } }.QuFtJ;
    "
    );
}

#[test]
fn an_inline_object_string_becomes_the_string() {
    assert_transform!(
        r#"({ YhxvC: "default" }).YhxvC;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"("default");"#
    );
}

#[test]
fn a_key_that_is_not_five_letters_is_ignored() {
    assert_transform!(
        r#"({ abc: "default" }).abc;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"({ abc: "default" }).abc;"#
    );
}

#[test]
fn the_dispatch_call_form_is_inlined() {
    assert_transform!(
        "({\n  HLAuI: function (a, b, c) {\n    return a(b, c);\n  }\n}).HLAuI(f, p, q);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"f(p, q);"
    );
}

#[test]
fn the_spread_form_is_inlined() {
    assert_transform!(
        "({\n  UcfbR: function (a, ...b) {\n    return a(...b);\n  }\n}).UcfbR(f, p, q);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"f(p, q);"
    );
}

#[test]
fn a_computed_constant_key_is_inlined_like_a_static_one() {
    assert_transform!(
        r#"({ ["YhxvC"]: "default" }).YhxvC;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"("default");"#
    );
}

#[test]
fn a_computed_key_that_is_not_a_constant_string_is_refused() {
    assert_transform!(
        r#"({ [k]: "default" }).YhxvC;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"({ [k]: "default" }).YhxvC;"#
    );
}

#[test]
fn a_member_name_missing_from_the_object_is_left_alone() {
    assert_transform!(
        r#"({ QuFtJ: "x" }).other;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"({ QuFtJ: "x" }).other;"#
    );
}

#[test]
fn duplicate_keys_take_the_last_value_so_the_pass_refuses_the_literal() {
    assert_transform!(
        r#"({ QuFtJ: "first", QuFtJ: "second" }).QuFtJ;"#,
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    ({
      QuFtJ: "first",
      QuFtJ: "second"
    }).QuFtJ;
    "#
    );
}

#[test]
fn duplicate_parameter_names_are_refused() {
    assert_transform!(
        "({\n  QuFtJ: function (a, a) {\n    return a + a;\n  }\n}).QuFtJ(\"x\", \"y\");",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r#"
    ({ QuFtJ: function (a, a) {
      return a + a;
    } }).QuFtJ("x", "y");
    "#
    );
}

#[test]
fn a_call_with_more_arguments_than_parameters_is_refused() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return n + r;\n  }\n}).QuFtJ(1, 2, side(3));",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    ({ QuFtJ: function (n, r) {
      return n + r;
    } }).QuFtJ(1, 2, side(3));
    "
    );
}

#[test]
fn a_logical_argument_that_would_short_circuit_differently_is_refused() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return n || r;\n  }\n}).QuFtJ(true, side());",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    ({ QuFtJ: function (n, r) {
      return n || r;
    } }).QuFtJ(true, side());
    "
    );
}

#[test]
fn reordering_operand_arguments_with_side_effects_is_refused() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return r + n;\n  }\n}).QuFtJ(t(1), t(2));",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    ({ QuFtJ: function (n, r) {
      return r + n;
    } }).QuFtJ(t(1), t(2));
    "
    );
}

#[test]
fn splicing_a_member_expression_into_callee_position_is_refused() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return n(r);\n  }\n}).QuFtJ(obj.m, 1);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    ({ QuFtJ: function (n, r) {
      return n(r);
    } }).QuFtJ(obj.m, 1);
    "
    );
}

#[test]
fn a_function_bearing_argument_moves_into_callee_position() {
    assert_transform!(
        "({\n  QuFtJ: function (n, r) {\n    return n(r);\n  }\n}).QuFtJ(function (x) {\n  return x + 1;\n}, 41);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"
    (function (x) {
      return x + 1;
    })(41);
    "
    );
}

#[test]
fn a_near_miss_computed_key_reports_the_ruled_note() {
    let alloc = Allocator::default();
    let (_code, notes) = common::run_ctors(
        &alloc,
        r#"({ [k]: "default" }).YhxvC;"#,
        &[untwist_deobfuscate::control_flow_object::boxed],
    );
    assert_eq!(notes, vec![NON_CONSTANT_KEY_NOTE.to_string()]);
}

#[test]
fn a_non_cff_object_with_a_non_constant_key_reports_no_note() {
    let alloc = Allocator::default();
    let (code, notes) = common::run_ctors(
        &alloc,
        r#"({ [k]: 1 }).length;"#,
        &[untwist_deobfuscate::control_flow_object::boxed],
    );
    assert_eq!(code.trim(), r#"({ [k]: 1 }).length;"#);
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn the_full_driver_inlines_pattern_c_through_the_cleanup_group() {
    let (code, _notes) = common::run_passes(
        "console.log(({\n  QuFtJ: function (n, r) {\n    return n === r;\n  }\n}).QuFtJ(u, undefined));",
    );
    assert_eq!(code.trim(), r#"console.log(u === undefined);"#);
}

#[test]
fn the_inlined_call_form_is_idempotent() {
    let source =
        "({\n  QuFtJ: function (n, r) {\n    return n === r;\n  }\n}).QuFtJ(u, undefined);";
    let first_alloc = Allocator::default();
    let once = untwist_testing::transform(
        &first_alloc,
        source,
        &[untwist_deobfuscate::control_flow_object::boxed],
    );
    let second_alloc = Allocator::default();
    let twice = untwist_testing::transform(
        &second_alloc,
        &once,
        &[untwist_deobfuscate::control_flow_object::boxed],
    );
    assert_eq!(once, twice);
}

#[test]
fn the_spread_form_forwards_a_call_site_that_spreads_its_own_arguments() {
    assert_transform!(
        "({\n  UcfbR: function (a, ...b) {\n    return a(...b);\n  }\n}).UcfbR(f, ...xs, q, ...ys);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @"f(...xs, q, ...ys);"
    );
}

#[test]
fn the_spread_form_refuses_a_callee_argument_that_would_gain_a_receiver() {
    assert_transform!(
        "({\n  UcfbR: function (a, ...b) {\n    return a(...b);\n  }\n}).UcfbR(o.f, p);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r"
    ({ UcfbR: function (a, ...b) {
      return a(...b);
    } }).UcfbR(o.f, p);
    "
    );
}

#[test]
fn the_spread_form_refuses_a_call_site_with_no_callee_to_forward() {
    assert_transform!(
        "({\n  UcfbR: function (a, ...b) {\n    return a(...b);\n  }\n}).UcfbR();",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r"
    ({ UcfbR: function (a, ...b) {
      return a(...b);
    } }).UcfbR();
    "
    );
}

#[test]
fn the_spread_form_refuses_a_call_site_that_spreads_its_callee() {
    assert_transform!(
        "({\n  UcfbR: function (a, ...b) {\n    return a(...b);\n  }\n}).UcfbR(...args);",
        [untwist_deobfuscate::control_flow_object::boxed],
        @r"
    ({ UcfbR: function (a, ...b) {
      return a(...b);
    } }).UcfbR(...args);
    "
    );
}
