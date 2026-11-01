mod common;

use common::{conservative_run, default_run};
use oxc_allocator::Allocator;
use untwist_core::pass::{Pass, Safety};
use untwist_deobfuscate::notes;
use untwist_deobfuscate::var_transformation::boxed;

const SMUGGLED: &str = "function f(a, b, c) {\n  console.log(b);\n  b = 1;\n  c = 2;\n}";

const SMUGGLED_IN_AN_IIFE: &str =
    "(function() {\n  function f(a, b, c) {\n    console.log(b);\n    b = 1;\n    c = 2;\n  }\n})();";

const A_FREE_IDENTIFIER_NAMED_LIKE_THE_FRESH_NAME: &str =
    "c2 = \"GLOBAL\";\nfunction f(c) {\n  c = 2;\n  return c + \":\" + c2;\n}";

const A_TYPEOF_GUARD_ON_THE_FRESH_NAME: &str =
    "function f(c) {\n  c = 2;\n  return typeof c2 === \"undefined\" ? \"no-global\" : \"global\";\n}";

const READ_BEFORE_THE_ASSIGNMENT: &str = "function f(c) {\n  g(c);\n  c = 2;\n  return c;\n}";

const REASSIGNED_TWICE: &str = "function f(c) {\n  c = 1;\n  c = 2;\n}";

const CAPTURED_BY_A_CLOSURE: &str =
    "function f(c) {\n  var g = function () {\n    return c;\n  };\n  c = 2;\n}";

const ASSIGNED_INSIDE_A_BRANCH: &str =
    "function f(c) {\n  if (x) {\n    c = 2;\n  }\n  return c;\n}";

const ASSIGNED_FROM_A_NESTED_FUNCTION: &str =
    "function f(c) {\n  function g() {\n    c = 2;\n  }\n  return g;\n}";

const READS_ARGUMENTS: &str = "function f(c) {\n  c = 2;\n  return arguments[0];\n}";

const A_SCOPE_HAZARD: &str = "function f(c) {\n  with(o) {}\n  c = 2;\n}";

const READS_ITSELF: &str = "function f(c) {\n  c = c + 1;\n  return c;\n}";

const A_DESTRUCTURED_PARAMETER: &str = "function f({ c }) {\n  c = 2;\n  return c;\n}";

const A_REST_PARAMETER: &str = "function f(...c) {\n  c = 2;\n  return c;\n}";

const A_BLOCK_FUNCTION_WITH_THE_PARAMETER_NAME: &str =
    "function f(c) {\n  c = 2;\n  {\n    function c() {}\n  }\n  return typeof c;\n}";

const ARGUMENTS_READ_THROUGH_THE_FUNCTION: &str =
    "function f(c) {\n  c = 2;\n  return g();\n}\nfunction g() {\n  return f.arguments[0];\n}";

fn left_alone(source: &str) {
    let alloc = Allocator::default();
    let code = default_run(&alloc, source, boxed);
    assert_eq!(code.trim(), source.trim());
}

#[test]
fn issue_161_a_smuggled_parameter_becomes_a_var_declaration_by_default() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, SMUGGLED, boxed);
    assert_eq!(
        code.trim(),
        "function f(a, b, c2) {\n  console.log(b);\n  b = 1;\n  var c = 2;\n}"
    );
}

#[test]
fn a_read_after_the_assignment_reads_the_declaration_and_not_the_parameter() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, "function f(c) {\n  c = 2;\n  return c;\n}", boxed);
    assert_eq!(
        code.trim(),
        "function f(c2) {\n  var c = 2;\n  return c;\n}"
    );
}

#[test]
fn the_pass_is_inert_under_conservative() {
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, SMUGGLED, boxed).trim(),
        SMUGGLED.trim()
    );
}

#[test]
fn a_rename_collision_bumps_the_suffix() {
    let alloc = Allocator::default();
    let code = default_run(
        &alloc,
        "function f(c, c2) {\n  console.log(c2);\n  c = 2;\n}",
        boxed,
    );
    assert_eq!(
        code.trim(),
        "function f(c2_2__, c2) {\n  console.log(c2);\n  var c = 2;\n}"
    );
}

#[test]
fn a_smuggled_parameter_of_a_nested_function_is_transformed() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, SMUGGLED_IN_AN_IIFE, boxed);
    assert!(code.contains("function f(a, b, c2)"), "{code}");
    assert!(code.contains("var c = 2;"), "{code}");
}

#[test]
fn the_fresh_name_does_not_capture_a_free_identifier() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_FREE_IDENTIFIER_NAMED_LIKE_THE_FRESH_NAME, boxed);
    assert_eq!(
        code.trim(),
        "c2 = \"GLOBAL\";\nfunction f(c2_2__) {\n  var c = 2;\n  return c + \":\" + c2;\n}"
    );
}

#[test]
fn the_fresh_name_does_not_capture_an_undeclared_global_a_typeof_guard_tests() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_TYPEOF_GUARD_ON_THE_FRESH_NAME, boxed);
    assert_eq!(
        code.trim(),
        "function f(c2_2__) {\n  var c = 2;\n  return typeof c2 === \"undefined\" ? \"no-global\" : \"global\";\n}"
    );
}

#[test]
fn a_parameter_read_before_the_assignment_is_left_alone() {
    left_alone(READ_BEFORE_THE_ASSIGNMENT);
}

#[test]
fn a_parameter_reassigned_twice_is_left_alone() {
    left_alone(REASSIGNED_TWICE);
}

#[test]
fn a_parameter_captured_by_a_closure_is_left_alone() {
    left_alone(CAPTURED_BY_A_CLOSURE);
}

#[test]
fn an_assignment_that_is_not_a_direct_statement_of_the_body_is_left_alone() {
    left_alone(ASSIGNED_INSIDE_A_BRANCH);
}

#[test]
fn a_parameter_written_from_a_nested_function_is_left_alone() {
    left_alone(ASSIGNED_FROM_A_NESTED_FUNCTION);
}

#[test]
fn a_function_that_references_arguments_is_left_alone() {
    left_alone(READS_ARGUMENTS);
}

#[test]
fn a_program_with_a_scope_hazard_is_left_alone() {
    left_alone(A_SCOPE_HAZARD);
}

#[test]
fn a_parameter_read_by_its_own_assignment_is_left_alone() {
    left_alone(READS_ITSELF);
}

#[test]
fn a_destructured_parameter_is_left_alone() {
    left_alone(A_DESTRUCTURED_PARAMETER);
}

#[test]
fn a_rest_parameter_is_left_alone() {
    left_alone(A_REST_PARAMETER);
}

#[test]
fn a_block_function_named_like_the_parameter_is_left_alone() {
    left_alone(A_BLOCK_FUNCTION_WITH_THE_PARAMETER_NAME);
}

#[test]
fn a_program_that_reads_the_functions_arguments_property_is_left_alone() {
    left_alone(ARGUMENTS_READ_THROUGH_THE_FUNCTION);
}

const ARGUMENTS_READ_THROUGH_A_COMPUTED_KEY: &str =
    "function f(c) {\n  c = 2;\n  return g();\n}\nfunction g() {\n  return f[\"arguments\"][0];\n}";

#[test]
fn a_computed_arguments_key_is_the_same_hazard_and_is_noted() {
    left_alone(ARGUMENTS_READ_THROUGH_A_COMPUTED_KEY);
    let (_code, all) = common::run_all_stages_extended(ARGUMENTS_READ_THROUGH_A_COMPUTED_KEY);
    assert!(
        all.iter()
            .any(|note| note.contains(notes::VAR_ARGUMENTS_PROPERTY_ALIAS)),
        "{all:?}"
    );
}

#[test]
fn the_static_arguments_property_hazard_is_noted_as_well() {
    let (_code, all) = common::run_all_stages_extended(ARGUMENTS_READ_THROUGH_THE_FUNCTION);
    assert!(
        all.iter()
            .any(|note| note.contains(notes::VAR_ARGUMENTS_PROPERTY_ALIAS)),
        "{all:?}"
    );
}

const VAR_NOTES: &[&str] = &[
    notes::VAR_FUNCTION_READS_ARGUMENTS,
    notes::VAR_PARAMETER_WRITTEN_AGAIN,
    notes::VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT,
    notes::VAR_DECLARATION_NOT_REBUILT,
];

fn var_notes_of(source: &str) -> Vec<String> {
    let (_code, all) = common::run_all_stages_extended(source);
    all.into_iter()
        .filter(|note| VAR_NOTES.iter().any(|text| note.contains(text)))
        .collect()
}

#[test]
fn a_shape_that_does_not_match_is_refused_without_a_note() {
    assert_eq!(var_notes_of(A_DESTRUCTURED_PARAMETER), Vec::<String>::new());
    assert_eq!(var_notes_of(A_REST_PARAMETER), Vec::<String>::new());
    assert_eq!(var_notes_of(READS_ITSELF), Vec::<String>::new());
    assert_eq!(var_notes_of(ASSIGNED_INSIDE_A_BRANCH), Vec::<String>::new());
    assert_eq!(
        var_notes_of(A_BLOCK_FUNCTION_WITH_THE_PARAMETER_NAME),
        Vec::<String>::new()
    );
    assert_eq!(
        var_notes_of(ARGUMENTS_READ_THROUGH_THE_FUNCTION),
        Vec::<String>::new()
    );
}

#[test]
fn a_refusal_that_follows_a_matched_shape_is_noted() {
    assert_eq!(
        var_notes_of(READ_BEFORE_THE_ASSIGNMENT),
        vec![notes::VAR_PARAMETER_USED_BEFORE_THE_ASSIGNMENT.to_string()]
    );
}

#[test]
fn the_pass_is_named_and_declares_its_mode_and_safety() {
    let pass: Box<dyn Pass<'_>> = boxed();
    assert_eq!(pass.name(), "var-transformation");
    assert!(pass.requires_extended());
    assert_eq!(pass.safety(), Safety::Safe);
}
