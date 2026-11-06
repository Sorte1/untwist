mod common;

use oxc_allocator::Allocator;
use untwist_core::ast_util::rename::UNIQUE_NAME_SUFFIX_CAP;
use untwist_core::parse::parse_program;
use untwist_core::pass::{PassOptions, RunReport};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::run_deobfuscate;
use untwist_testing::assert_transform;

#[test]
fn keep_true_branch() {
    assert_transform!(
        "if (\"a\" === \"a\") {\n  console.log(\"foo\");\n} else {\n  console.log(\"bar\");\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"console.log("foo");"#
    );
}

#[test]
fn keep_false_branch() {
    assert_transform!(
        "if (\"a\" !== \"a\") {\n  console.log(\"foo\");\n} else {\n  console.log(\"bar\");\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"console.log("bar");"#
    );
}

#[test]
fn remove_false_branch_without_else() {
    assert_transform!(
        "if (\"a\" !== \"a\") {\n  console.log(\"foo\");\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @""
    );
}

#[test]
fn merge_scopes() {
    assert_transform!(
        "let foo = 1;\nif (\"a\" === \"a\") {\n  let foo = 2;\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let foo = 1;
    let _foo = 2;
    "#
    );
}

#[test]
fn a_negated_test_is_handled() {
    assert_transform!(
        "if (!(\"a\" === \"b\")) {\n  a();\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @"a();"
    );
}

#[test]
fn a_conditional_expression_is_folded() {
    assert_transform!(
        "var x = \"a\" === \"a\" ? 1 : 2;",
        [untwist_deobfuscate::dead_code::boxed],
        @"var x = 1;"
    );
}

#[test]
fn a_non_string_comparison_is_left_alone() {
    assert_transform!(
        "if (1 === 1) {\n  a();\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    if (1 === 1) {
      a();
    }
    "#
    );
}

#[test]
fn a_side_effecting_or_non_literal_test_is_left_alone() {
    assert_transform!(
        "if (++counter) {\n  a();\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    if (++counter) {
      a();
    }
    "#
    );
    assert_transform!(
        "if (typeof x) {\n  a();\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    if (typeof x) {
      a();
    }
    "#
    );
}

#[test]
fn two_merges_into_the_same_scope_do_not_duplicate_bindings() {
    assert_transform!(
        "if (\"a\" === \"a\") {\n  let bar = 2;\n  console.log(bar);\n}\nif (\"b\" === \"b\") {\n  let bar = 3;\n  console.log(bar);\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let bar = 2;
    console.log(bar);
    let _bar = 3;
    console.log(_bar);
    "#
    );
}

#[test]
fn two_colliding_blocks_get_foo_then_foo2() {
    assert_transform!(
        "let foo = 1;\nif (\"a\" === \"a\") {\n  let foo = 2;\n  console.log(foo);\n}\nif (\"b\" === \"b\") {\n  let foo = 3;\n  console.log(foo);\n}\nconsole.log(foo);",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let foo = 1;
    let _foo = 2;
    console.log(_foo);
    let _foo2 = 3;
    console.log(_foo2);
    console.log(foo);
    "#
    );
}

#[test]
fn nested_merges_see_what_the_inner_merge_already_moved() {
    assert_transform!(
        "let foo = 1;\nif (\"a\" === \"a\") {\n  if (\"b\" === \"b\") {\n    let foo = 2;\n    console.log(foo);\n  }\n}\nconsole.log(foo);",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let foo = 1;
    let _foo = 2;
    console.log(_foo);
    console.log(foo);
    "#
    );
}

#[test]
fn a_name_forwarded_through_moved_still_collides_with_a_later_sibling() {
    assert_transform!(
        "if (\"a\" === \"a\") {\n  let bar = 1;\n  console.log(bar);\n}\nif (\"b\" === \"b\") {\n  if (\"c\" === \"c\") {\n    let bar = 2;\n    console.log(bar);\n  }\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let bar = 1;
    console.log(bar);
    let _bar = 2;
    console.log(_bar);
    "#
    );
}

#[test]
fn a_three_level_nesting_still_collides_with_a_flat_sibling() {
    assert_transform!(
        "if (\"a\" === \"a\") {\n  if (\"b\" === \"b\") {\n    if (\"c\" === \"c\") {\n      let bar = 1;\n      console.log(bar);\n    }\n  }\n}\nif (\"d\" === \"d\") {\n  let bar = 2;\n  console.log(bar);\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let bar = 1;
    console.log(bar);
    let _bar = 2;
    console.log(_bar);
    "#
    );
}

#[test]
fn the_forwarded_name_is_checked_even_when_the_nested_chain_comes_first() {
    assert_transform!(
        "if (\"b\" === \"b\") {\n  if (\"c\" === \"c\") {\n    let bar = 2;\n    console.log(bar);\n  }\n}\nif (\"a\" === \"a\") {\n  let bar = 1;\n  console.log(bar);\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let bar = 2;
    console.log(bar);
    let _bar = 1;
    console.log(_bar);
    "#
    );
}

#[test]
fn a_symbol_forwarded_through_two_merge_levels_keeps_exactly_one_final_name() {
    let alloc = Allocator::default();
    let source = "let bar = 1;\nif (\"w\" === \"w\") {\n  let _bar = 0;\n  console.log(_bar);\n}\nif (\"x\" === \"x\") {\n  if (\"y\" === \"y\") {\n    let bar = 2;\n    console.log(bar);\n  }\n}";
    let code = untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::dead_code::boxed]);
    assert_eq!(
        code,
        "let bar = 1;\nlet _bar = 0;\nconsole.log(_bar);\nlet _bar2 = 2;\nconsole.log(_bar2);"
    );
    assert_eq!(code.matches("_bar2").count(), 2, "{code}");
    assert!(!code.contains("let bar = 2"), "{code}");
}

#[test]
fn a_binding_that_shadows_an_ancestor_is_renamed_too() {
    assert_transform!(
        "let foo = 1;\nfunction g() {\n  console.log(foo);\n  if (\"a\" === \"a\") {\n    let foo = 2;\n    console.log(foo);\n  }\n}\ng();",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    let foo = 1;
    function g() {
      console.log(foo);
      let _foo = 2;
      console.log(_foo);
    }
    g();
    "#
    );
}

#[test]
fn a_bodiless_if_inside_a_loop_is_braced_before_the_pass_runs() {
    let (code, _notes) = common::run_passes("while (x) if (\"a\" === \"a\") f();");
    assert_eq!(code.trim(), "while (x) {\n  f();\n}");
}

fn run_deobfuscate_with(source: &str, unsafe_transforms: bool) -> (String, RunReport) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        unsafe_transforms,
        extended: false,
        max_iterations: Some(10),
        fuel: None,
        ..PassOptions::default()
    };
    let (_sem, report) = run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    (print(&parsed.program, PrintStyle::Pretty), report)
}

const AN_OPAQUE_IF_BESIDE_A_CONTROL_FLOW_OBJECT: &str =
    "({ YhxvC: \"default\" }).YhxvC;\nif (\"a\" === \"a\") {\n  g();\n}";

#[test]
fn a_default_run_folds_the_if_and_reports_the_count_under_dead_code() {
    let (code, report) = run_deobfuscate_with(AN_OPAQUE_IF_BESIDE_A_CONTROL_FLOW_OBJECT, false);
    assert!(!code.contains("if (\"a\" === \"a\")"), "{code}");
    assert_eq!(
        report.fixpoint_per_pass,
        vec![
            ("merge-strings", 0),
            ("dead-code", 1),
            ("control-flow-object", 1),
            ("control-flow-switch", 0),
        ]
    );
}

#[test]
fn an_unsafe_run_removes_the_if_and_reports_the_count_under_dead_code() {
    let (code, report) = run_deobfuscate_with(AN_OPAQUE_IF_BESIDE_A_CONTROL_FLOW_OBJECT, true);
    assert!(!code.contains("if (\"a\" === \"a\")"), "{code}");
    assert_eq!(
        report.fixpoint_per_pass,
        vec![
            ("merge-strings", 0),
            ("dead-code", 1),
            ("control-flow-object", 1),
            ("control-flow-switch", 0),
        ]
    );
}

fn every_fresh_foo_bound() -> String {
    let mut source = String::from("let _foo = 0;\n");
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        source.push_str(&format!("let _foo{suffix} = 0;\n"));
    }
    source
}

fn run_dead_code(source: &str) -> (String, RunReport) {
    let alloc = Allocator::default();
    untwist_testing::transform_report(
        &alloc,
        source,
        &[untwist_deobfuscate::dead_code::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    )
}

#[test]
fn a_merge_whose_fresh_name_budget_is_exhausted_is_refused_not_captured() {
    let mut source = every_fresh_foo_bound();
    source.push_str(
        "let foo = 1;\nif (\"a\" === \"a\") {\n  let foo = 2;\n  console.log(foo);\n}\nconsole.log(foo);",
    );
    let (code, report) = run_dead_code(&source);
    assert!(
        code.contains("if (\"a\" === \"a\")"),
        "the branch was merged onto a taken name: {code}"
    );
    assert!(
        report.notes.iter().any(|note| note
            .message
            .contains(untwist_deobfuscate::notes::DEAD_CODE_UNMERGEABLE)),
        "{:?}",
        report.notes
    );
}

const A_BLOCK_WHOSE_SECOND_BINDING_EXHAUSTS: &str =
    "if (\"a\" === \"a\") {\n  let bar = 2;\n  let foo = 2;\n  console.log(bar, foo);\n}";

#[test]
fn a_refused_merge_leaves_every_binding_name_and_the_tally_untouched() {
    let mut source = every_fresh_foo_bound();
    source.push_str("let bar = 1;\nlet foo = 1;\n");
    source.push_str(A_BLOCK_WHOSE_SECOND_BINDING_EXHAUSTS);
    source.push_str("\nconsole.log(bar, foo);");
    let (code, report) = run_dead_code(&source);
    assert!(
        code.contains(A_BLOCK_WHOSE_SECOND_BINDING_EXHAUSTS),
        "the kept block is not byte-identical to the input: {code}"
    );
    assert!(!code.contains("_bar"), "a staged rename leaked: {code}");
    assert_eq!(report.per_pass, vec![("dead-code", 0)]);
    assert_eq!(report.total_changes, 0);
    assert!(
        report.notes.iter().any(|note| note
            .message
            .contains(untwist_deobfuscate::notes::DEAD_CODE_UNMERGEABLE)),
        "{:?}",
        report.notes
    );
}

fn run_default_lane(source: &str) -> (String, RunReport) {
    let alloc = Allocator::default();
    untwist_testing::transform_report(
        &alloc,
        source,
        &[untwist_deobfuscate::dead_code::boxed],
        PassOptions::default(),
    )
}

fn default_lane(source: &str) -> String {
    run_default_lane(source).0
}

#[test]
fn the_default_lane_keeps_the_taken_branch() {
    assert_eq!(
        default_lane("if (\"a\" === \"a\") {\n  f();\n} else {\n  g();\n}"),
        "f();"
    );
    assert_eq!(
        default_lane("if (\"a\" !== \"a\") {\n  f();\n} else {\n  g();\n}"),
        "g();"
    );
    assert_eq!(default_lane("if (\"a\" !== \"a\") {\n  f();\n}"), "");
    assert_eq!(default_lane("x = \"a\" === \"b\" ? 1 : 2;"), "x = 2;");
}

#[test]
fn the_default_lane_keeps_a_var_the_discarded_branch_declares() {
    assert_eq!(
        default_lane(
            "if (\"a\" !== \"a\") {\n  var x = 1, { y } = o;\n  for (var i in o) {}\n} else {\n  f();\n}\nconsole.log(x, y, i);"
        ),
        "var x, y, i;\nf();\nconsole.log(x, y, i);"
    );
    assert_eq!(
        default_lane("if (\"a\" !== \"a\") {\n  var x = function () {\n    var z;\n  };\n}"),
        "var x;"
    );
}

#[test]
fn the_default_lane_drops_a_var_only_the_discarded_branch_uses() {
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" !== \"a\") {\n    var x = 1;\n    h(x);\n  }\n  return 2;\n}"
        ),
        "function g() {\n  return 2;\n}"
    );
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" === \"a\") {\n    f();\n  } else {\n    var x, y = x;\n  }\n  return y;\n}"
        ),
        "function g() {\n  var y;\n  f();\n  return y;\n}"
    );
}

#[test]
fn the_default_lane_keeps_a_discarded_var_that_other_code_can_still_see() {
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" !== \"a\") {\n    var x = 1;\n  }\n  return typeof x;\n}"
        ),
        "function g() {\n  var x;\n  return typeof x;\n}"
    );
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" !== \"a\") {\n    var x = 1;\n  }\n  return function () {\n    return x;\n  };\n}"
        ),
        "function g() {\n  var x;\n  return function () {\n    return x;\n  };\n}"
    );
    assert_eq!(
        default_lane("if (\"a\" !== \"a\") {\n  var x = 1;\n}"),
        "var x;"
    );
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" !== \"a\") {\n    var x = 1;\n  }\n  return eval(\"x\");\n}"
        ),
        "function g() {\n  var x;\n  return eval(\"x\");\n}"
    );
}

#[test]
fn the_default_lane_keeps_the_conditional_when_the_discarded_branch_declares_a_function() {
    let sloppy = "if (\"a\" !== \"a\") {\n  {\n    function f() {}\n  }\n}\nconsole.log(f);";
    assert_eq!(
        default_lane(&format!("\"use strict\";\n{sloppy}")),
        "\"use strict\";\nconsole.log(f);"
    );
    let (code, report) = run_default_lane(sloppy);
    assert!(code.starts_with("if (\"a\" !== \"a\") {"), "{code}");
    assert!(
        report.notes.iter().any(|note| note
            .message
            .contains(untwist_deobfuscate::notes::DEAD_CODE_DISCARDED_FUNCTION)),
        "{:?}",
        report.notes
    );
}

#[test]
fn the_default_lane_merges_a_block_whose_bindings_are_all_fresh() {
    assert_eq!(
        default_lane(
            "function g() {\n  if (\"a\" === \"a\") {\n    let y = 1;\n    return y;\n  }\n}"
        ),
        "function g() {\n  let y = 1;\n  return y;\n}"
    );
}

#[test]
fn the_default_lane_renames_a_merged_binding_that_shadows_an_outer_one() {
    assert_eq!(
        default_lane(
            "let x = 0;\nfunction g() {\n  if (\"a\" === \"a\") {\n    let x = 1;\n    h(x);\n  }\n  return x;\n}"
        ),
        "let x = 0;\nfunction g() {\n  let _x = 1;\n  h(_x);\n  return x;\n}"
    );
}

#[test]
fn the_default_lane_keeps_the_block_when_a_merge_could_change_a_name() {
    for (source, kept) in [
        (
            "function g() {\n  if (\"a\" === \"a\") {\n    let x = 1;\n  }\n  return typeof x;\n}",
            "  {\n    let x = 1;\n  }\n",
        ),
        (
            "function g() {\n  if (\"a\" === \"a\") {\n    let x = 1;\n  }\n  return eval(\"typeof x\");\n}",
            "  {\n    let x = 1;\n  }\n",
        ),
        (
            "function g() {\n  if (\"a\" === \"a\") {\n    let x = 1;\n  }\n  with (o) {\n    h();\n  }\n}",
            "  {\n    let x = 1;\n  }\n",
        ),
        (
            "function g() {\n  h(f);\n  if (\"a\" === \"a\") {\n    function f() {}\n  }\n}",
            "  {\n    function f() {}\n  }\n",
        ),
        (
            "if (\"a\" === \"a\") {\n  let x = 1;\n  console.log(x);\n}",
            "{\n  let x = 1;\n  console.log(x);\n}",
        ),
    ] {
        let code = default_lane(source);
        assert!(!code.contains("\"a\" === \"a\""), "{code}");
        assert!(code.contains(kept), "{source}\n=> {code}");
    }
}

#[test]
fn the_default_lane_braces_a_bodiless_function_it_keeps() {
    assert_eq!(
        default_lane("if (\"a\" === \"a\") function f() {}"),
        "{\n  function f() {}\n}"
    );
}

#[test]
fn a_false_literal_if_is_removed() {
    assert_transform!(
        "if (false) {\n  while (true) {}\n}\nrun();",
        [untwist_deobfuscate::dead_code::boxed],
        @"run();"
    );
}

#[test]
fn a_truthy_literal_keeps_the_consequent() {
    assert_transform!(
        "if (1) {\n  a();\n} else {\n  b();\n}",
        [untwist_deobfuscate::dead_code::boxed],
        @"a();"
    );
}

#[test]
fn a_dead_while_loop_is_removed() {
    assert_transform!(
        "while (null) {\n  a();\n}\nrun();",
        [untwist_deobfuscate::dead_code::boxed],
        @"run();"
    );
}

#[test]
fn a_hoisted_var_in_a_dead_branch_is_kept_without_its_initialiser() {
    assert_transform!(
        "if (0) {\n  var x = 1;\n}\nrun();",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    var x;
    run();
    "#
    );
}

#[test]
fn a_literal_and_picks_its_left_operand() {
    assert_transform!(
        "var a = 0 && f();\nvar b = 1 && g();\nvar c = \"x\" || h();",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    var a = 0;
    var b = g();
    var c = "x";
    "#
    );
}

#[test]
fn a_reference_picked_from_a_literal_ternary_is_kept_wrapped() {
    assert_transform!(
        "(true ? a.b : c)();",
        [untwist_deobfuscate::dead_code::boxed],
        @"(true ? a.b : c)();"
    );
}

#[test]
fn an_identifier_picked_from_a_literal_ternary_is_folded_except_under_typeof() {
    assert_transform!(
        "var a = true ? b : c;\nvar d = typeof (true ? e : f);",
        [untwist_deobfuscate::dead_code::boxed],
        @r#"
    var a = b;
    var d = typeof (true ? e : f);
    "#
    );
}
