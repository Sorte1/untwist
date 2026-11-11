mod common;

use untwist_testing::assert_transform;

#[test]
fn atob() {
    assert_transform!(
        r#"atob("aGVsbG8=");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"("hello");"#
    );
}

#[test]
fn atob_that_throws_is_left_alone() {
    assert_transform!(
        r#"atob("-");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"atob("-");"#
    );
}

#[test]
fn atob_of_a_length_one_input_is_left_alone() {
    assert_transform!(
        r#"atob("a");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"atob("a");"#
    );
}

#[test]
fn unescape_decode_uri_and_decode_uri_component() {
    assert_transform!(
        r#"unescape("%41");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"("A");"#
    );
    assert_transform!(
        r#"decodeURI("%41");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"("A");"#
    );
    assert_transform!(
        r#"decodeURIComponent("%41");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"("A");"#
    );
}

#[test]
fn a_shadowed_global_is_left_alone() {
    assert_transform!(
        "function f(atob) {\n  return atob(\"aGVsbG8=\");\n}",
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"
    function f(atob) {
      return atob("aGVsbG8=");
    }
    "#
    );
}

#[test]
fn a_non_literal_argument_is_left_alone() {
    assert_transform!(
        "atob(x);",
        [untwist_deobfuscate::evaluate_globals::boxed],
        @"atob(x);"
    );
}

#[test]
fn a_wrong_argument_count_is_left_alone() {
    assert_transform!(
        r#"atob("a", "b");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"atob("a", "b");"#
    );
    assert_transform!(
        "atob();",
        [untwist_deobfuscate::evaluate_globals::boxed],
        @"atob();"
    );
}

#[test]
fn decode_uri_preserves_reserved_octets() {
    assert_transform!(
        r#"decodeURI("%3B%41");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"("%3BA");"#
    );
}

#[test]
fn a_malformed_percent_sequence_is_left_alone() {
    assert_transform!(
        r#"decodeURIComponent("%");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"decodeURIComponent("%");"#
    );
}

#[test]
fn an_unrelated_global_call_is_left_alone() {
    assert_transform!(
        r#"parseInt("42");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"parseInt("42");"#
    );
}

#[test]
fn a_member_call_is_left_alone() {
    assert_transform!(
        r#"globalThis.atob("aGVsbG8=");"#,
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"globalThis.atob("aGVsbG8=");"#
    );
}

#[test]
fn a_call_the_folder_cannot_prove_notes_once_even_across_a_forced_second_iteration() {
    let alloc = untwist_testing::Allocator::default();
    let (code, notes) = common::run_ctors(
        &alloc,
        "const obj = {};\nobj.a = 1;\nf(obj);\nf(obj);\ng(atob(\"-\"));",
        &[
            untwist_deobfuscate::merge_object_assignments::boxed,
            untwist_deobfuscate::evaluate_globals::boxed,
        ],
    );
    assert_eq!(
        code,
        "const obj = { a: 1 };\nf(obj);\nf(obj);\ng(atob(\"-\"));"
    );
    assert_eq!(
        notes,
        vec!["atob was not evaluated: the folder could not prove a result".to_string()]
    );
}

#[test]
fn issue_157_a_non_ascii_result_round_trips_through_the_printer() {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::{Expression, Statement};
    use untwist_core::parse::parse_program;

    for (source, expected) in [
        (r#"f(decodeURIComponent("%D0%96"));"#, "\u{0416}"),
        (r#"f(decodeURIComponent("%E2%80%A8"));"#, "\u{2028}"),
        (r#"f(decodeURIComponent("%F0%9F%98%80"));"#, "\u{1F600}"),
    ] {
        let alloc = Allocator::default();
        let printed = untwist_testing::transform(
            &alloc,
            source,
            &[untwist_deobfuscate::evaluate_globals::boxed],
        );

        let reparse = Allocator::default();
        let parsed = parse_program(&reparse, &printed);
        assert!(
            parsed.diagnostics.is_empty(),
            "{printed}: {:?}",
            parsed.diagnostics
        );
        let Statement::ExpressionStatement(stmt) = &parsed.program.body[0] else {
            panic!("expression statement: {printed}");
        };
        let Expression::CallExpression(call) = &stmt.expression else {
            panic!("call: {printed}");
        };
        let Some(Expression::StringLiteral(literal)) =
            call.arguments.first().and_then(|a| a.as_expression())
        else {
            panic!("the decoded string was not substituted: {printed}");
        };
        assert_eq!(literal.value.as_str(), Some(expected), "{printed}");
    }
}

#[test]
fn issue_157_the_js_line_terminators_are_escaped_rather_than_written_raw() {
    let alloc = oxc_allocator::Allocator::default();
    let printed = untwist_testing::transform(
        &alloc,
        r#"f(decodeURIComponent("%E2%80%A8"));"#,
        &[untwist_deobfuscate::evaluate_globals::boxed],
    );
    assert!(!printed.contains('\u{2028}'), "{printed}");
}

fn default_lane(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_deobfuscate::evaluate_globals::boxed],
        untwist_core::pass::PassOptions::default(),
    )
}

#[test]
fn an_untouched_global_still_folds_in_the_default_lane() {
    assert_eq!(
        default_lane("console.log(atob(\"aGk=\"));"),
        "console.log(\"hi\");"
    );
}

#[test]
fn a_global_the_program_assigns_is_left_alone_in_the_default_lane() {
    let source =
        "atob = function (s) {\n  return \"patched:\" + s;\n};\nconsole.log(atob(\"aGk=\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_assigned_inside_a_function_is_left_alone_in_the_default_lane() {
    let source = "function patch() {\n  decodeURIComponent = String;\n}\npatch();\nconsole.log(decodeURIComponent(\"%41\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_assigned_after_the_call_is_left_alone_in_the_default_lane() {
    let source =
        "function show() {\n  console.log(unescape(\"%41\"));\n}\nunescape = String;\nshow();";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_a_block_function_declares_is_left_alone_in_the_default_lane() {
    let source = "{\n  function decodeURI(s) {\n    return \"mine:\" + s;\n  }\n}\nconsole.log(decodeURI(\"%41\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn only_the_assigned_global_is_left_alone_in_the_default_lane() {
    assert_eq!(
        default_lane("atob = String;\nconsole.log(atob(\"aGk=\"), unescape(\"%41\"));"),
        "atob = String;\nconsole.log(atob(\"aGk=\"), \"A\");"
    );
}

#[test]
fn an_assigned_global_still_folds_under_unsafe() {
    assert_transform!(
        "atob = String;\nconsole.log(atob(\"aGk=\"));",
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"
    atob = String;
    console.log("hi");
    "#
    );
}

#[test]
fn a_global_replaced_through_global_this_is_left_alone_in_the_default_lane() {
    let source =
        "globalThis.atob = function (s) {\n  return \"patched\";\n};\nconsole.log(atob(\"aGk=\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_replaced_through_a_computed_key_on_an_alias_is_left_alone_in_the_default_lane() {
    let source = "var g = globalThis;\ng[\"unes\" + \"cape\"] = function () {\n  return \"p2\";\n};\nconsole.log(unescape(\"%41\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_replaced_through_window_with_a_literal_key_is_left_alone_in_the_default_lane() {
    let source = "window[\"decodeURI\"] = String;\nconsole.log(decodeURI(\"%41\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_replaced_through_top_level_this_is_left_alone_in_the_default_lane() {
    let source = "this.decodeURIComponent = String;\nconsole.log(decodeURIComponent(\"%41\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn a_global_defined_on_the_global_object_is_left_alone_in_the_default_lane() {
    let source =
        "Object.defineProperty(self, \"atob\", { value: String });\nconsole.log(atob(\"aGk=\"));";
    assert_eq!(default_lane(source), source);
}

#[test]
fn only_the_global_replaced_through_the_global_object_is_left_alone_in_the_default_lane() {
    assert_eq!(
        default_lane("window.atob = String;\nconsole.log(atob(\"aGk=\"), unescape(\"%41\"));"),
        "window.atob = String;\nconsole.log(atob(\"aGk=\"), \"A\");"
    );
}

#[test]
fn a_global_replaced_through_the_global_object_still_folds_under_unsafe() {
    assert_transform!(
        "globalThis.atob = String;\nconsole.log(atob(\"aGk=\"));",
        [untwist_deobfuscate::evaluate_globals::boxed],
        @r#"
    globalThis.atob = String;
    console.log("hi");
    "#
    );
}
