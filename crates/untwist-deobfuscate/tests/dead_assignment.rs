mod common;

use common::{conservative_run, default_run};
use oxc_allocator::Allocator;
use untwist_core::pass::{Pass, Safety};
use untwist_deobfuscate::dead_assignment::boxed;
use untwist_deobfuscate::notes;

const HOISTED: &str = "var x;\nx = \"y\";\nFunc(x);";

const READ_BEFORE_THE_ASSIGNMENT: &str = "var y;\nFunc(y);\ny = \"z\";";

const A_NON_CONSTANT_RIGHT_HAND_SIDE: &str = "var x;\nx = compute();\nFunc(x);";

const A_SECOND_ASSIGNMENT: &str = "var x;\nx = 1;\nx = 2;\nFunc(x);";

const A_DECLARATION_WITH_AN_INIT: &str = "var x = 0;\nx = 1;\nFunc(x);";

const MORE_THAN_ONE_DECLARATOR: &str = "var x, y;\nx = 1;\nFunc(x, y);";

const A_SAME_NAMED_INNER_BINDING: &str = "var x;\n{\n  let x;\n  x = 1;\n}\nFunc(x);";

const A_SCOPE_HAZARD: &str = "var x;\nwith(o) {}\nx = 1;\nFunc(x);";

const TWO_EMPTY_DECLARATIONS: &str = "var x;\nvar x;\nx = 1;\nFunc(x);";

const A_MEMBER_READ_RIGHT_HAND_SIDE: &str = "var x;\no = null;\nx = o.p;\nFunc(x);";

const AN_ASSIGNMENT_A_CLOSURE_READS: &str =
    "var x;\ng();\nx = 1;\nfunction g() {\n  console.log(x);\n}";

const A_ROOT_VAR_WITH_A_CALL_BETWEEN: &str = "var x;\nf();\nx = 1;\nFunc(x);";

const A_ROOT_VAR_ASSIGNED_IMMEDIATELY: &str = "var x;\nx = 1;\nf();\nFunc(x);";

const A_FUNCTION_SCOPED_VAR_WITH_A_CALL_BETWEEN: &str =
    "function g() {\n  var x;\n  f();\n  x = 1;\n  return x;\n}";

const A_FUNCTION_SCOPED_READ_BEFORE_THE_ASSIGNMENT: &str =
    "function h() {\n  var y;\n  Func(y);\n  y = \"z\";\n  return y;\n}";

const A_THROW_INTO_A_CATCH_THAT_READS_IT: &str =
    "function f(g) {\n  var out;\n  try {\n    var x;\n    g();\n    x = 1;\n    out = \"t\" + x;\n  } catch (e) {\n    out = \"c\" + x;\n  }\n  return out;\n}";

const A_THROW_WITH_THE_READ_AFTER_THE_TRY: &str =
    "function f(g) {\n  var r;\n  try {\n    var x;\n    g();\n    x = 1;\n  } catch (e) {\n    r = 0;\n  }\n  return \"r\" + x;\n}";

const A_BREAK_THAT_SKIPS_THE_WRITE: &str =
    "function f(n) {\n  while (n) {\n    var x;\n    if (n === 1) break;\n    x = 1;\n  }\n  return x;\n}";

const A_CONTINUE_THAT_SKIPS_THE_WRITE: &str =
    "function f() {\n  for (var i = 0; i < 1; i++) {\n    var x;\n    if (i === 0) continue;\n    x = 1;\n  }\n  return \"b\" + x;\n}";

const A_NESTED_INITIALISED_REDECLARATION: &str =
    "function f(c) {\n  var x;\n  if (c) {\n    var x = 2;\n  }\n  x = 1;\n  return x;\n}";

const A_FOR_OF_REDECLARATION: &str =
    "function f(list) {\n  var x;\n  for (var x of list) {}\n  x = 1;\n  return x;\n}";

const A_FOR_IN_REDECLARATION: &str =
    "function f(o) {\n  var x;\n  for (var x in o) {}\n  x = 1;\n  return x;\n}";

const A_VAR_REDECLARING_A_PARAMETER: &str =
    "function f(x) {\n  var x;\n  g(arguments[0]);\n  x = 1;\n  return x;\n}";

const A_FOLDED_CALL_AFTER_A_STATEMENT: &str =
    "function f() {\n  var x;\n  Math.max = null;\n  x = Math.max(1, 2);\n  return x;\n}";

const A_FOLDED_CALL_BESIDE_THE_DECLARATION: &str =
    "function f() {\n  var x;\n  x = Math.max(1, 2);\n  return x;\n}";

const A_LITERAL_AFTER_A_STATEMENT: &str =
    "function f() {\n  var x;\n  g();\n  x = \"lit\";\n  return x;\n}";

const A_FUNCTION_SCOPED_MEMBER_READ_RIGHT_HAND_SIDE: &str =
    "function f(o) {\n  var x;\n  o = null;\n  x = o.p;\n  return x;\n}";

const AN_ASSIGNMENT_A_NESTED_CLOSURE_READS: &str =
    "function h() {\n  function g() {\n    console.log(x);\n  }\n  var x;\n  g();\n  x = 1;\n  return x;\n}";

const A_VAR_IN_A_CLASS_STATIC_BLOCK: &str =
    "class C {\n  static {\n    var x;\n    x = \"y\";\n    Func(x);\n  }\n}";

fn left_alone(source: &str) {
    let alloc = Allocator::default();
    let code = default_run(&alloc, source, boxed);
    assert_eq!(code.trim(), source.trim());
}

#[test]
fn issue_226_a_hoisted_var_assigned_once_collapses_by_default() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, HOISTED, boxed);
    assert_eq!(code.trim(), "var x = \"y\";\nFunc(x);");
}

#[test]
fn the_pass_is_inert_under_conservative() {
    let alloc = Allocator::default();
    assert_eq!(
        conservative_run(&alloc, HOISTED, boxed).trim(),
        HOISTED.trim()
    );
}

#[test]
fn issue_226_a_var_read_before_the_assignment_is_left_alone() {
    left_alone(READ_BEFORE_THE_ASSIGNMENT);
}

#[test]
fn a_non_constant_right_hand_side_is_left_alone() {
    left_alone(A_NON_CONSTANT_RIGHT_HAND_SIDE);
}

#[test]
fn a_second_assignment_is_left_alone() {
    left_alone(A_SECOND_ASSIGNMENT);
}

#[test]
fn a_declaration_with_an_init_is_left_alone() {
    left_alone(A_DECLARATION_WITH_AN_INIT);
}

#[test]
fn a_declaration_with_more_than_one_declarator_is_left_alone() {
    left_alone(MORE_THAN_ONE_DECLARATOR);
}

#[test]
fn an_assignment_to_a_same_named_inner_binding_is_left_alone() {
    left_alone(A_SAME_NAMED_INNER_BINDING);
}

#[test]
fn a_var_declared_in_a_class_static_block_is_left_alone() {
    left_alone(A_VAR_IN_A_CLASS_STATIC_BLOCK);
}

#[test]
fn a_program_with_a_scope_hazard_is_left_alone() {
    left_alone(A_SCOPE_HAZARD);
}

#[test]
fn a_symbol_with_two_empty_declarations_is_left_alone() {
    left_alone(TWO_EMPTY_DECLARATIONS);
}

#[test]
fn a_right_hand_side_whose_evaluation_would_move_backwards_is_left_alone() {
    left_alone(A_MEMBER_READ_RIGHT_HAND_SIDE);
}

#[test]
fn an_assignment_a_closure_reads_before_it_runs_is_left_alone() {
    left_alone(AN_ASSIGNMENT_A_CLOSURE_READS);
}

#[test]
fn an_assignment_a_nested_closure_reads_before_it_runs_is_left_alone() {
    left_alone(AN_ASSIGNMENT_A_NESTED_CLOSURE_READS);
}

#[test]
fn a_function_scoped_read_before_the_assignment_is_left_alone() {
    left_alone(A_FUNCTION_SCOPED_READ_BEFORE_THE_ASSIGNMENT);
}

#[test]
fn a_root_var_whose_assignment_is_not_the_next_statement_is_left_alone() {
    left_alone(A_ROOT_VAR_WITH_A_CALL_BETWEEN);
}

#[test]
fn a_root_var_assigned_by_the_next_statement_collapses() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_ROOT_VAR_ASSIGNED_IMMEDIATELY, boxed);
    assert_eq!(code.trim(), "var x = 1;\nf();\nFunc(x);");
}

#[test]
fn a_write_a_throw_can_skip_is_left_alone() {
    left_alone(A_THROW_INTO_A_CATCH_THAT_READS_IT);
    left_alone(A_THROW_WITH_THE_READ_AFTER_THE_TRY);
}

#[test]
fn a_write_a_break_or_continue_can_skip_is_left_alone() {
    left_alone(A_BREAK_THAT_SKIPS_THE_WRITE);
    left_alone(A_CONTINUE_THAT_SKIPS_THE_WRITE);
}

#[test]
fn a_redeclaration_that_also_writes_is_left_alone() {
    left_alone(A_NESTED_INITIALISED_REDECLARATION);
    left_alone(A_FOR_OF_REDECLARATION);
    left_alone(A_FOR_IN_REDECLARATION);
}

#[test]
fn a_var_redeclaring_a_parameter_keeps_the_var_transformation_refusal() {
    left_alone(A_VAR_REDECLARING_A_PARAMETER);
    let (_code, all) = common::run_all_stages_extended(A_VAR_REDECLARING_A_PARAMETER);
    assert!(
        all.iter()
            .any(|note| note.contains(notes::VAR_FUNCTION_READS_ARGUMENTS)),
        "{all:?}"
    );
}

#[test]
fn a_folded_right_hand_side_must_sit_beside_the_declaration() {
    left_alone(A_FOLDED_CALL_AFTER_A_STATEMENT);
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_FOLDED_CALL_BESIDE_THE_DECLARATION, boxed);
    assert_eq!(
        code.trim(),
        "function f() {\n  var x = Math.max(1, 2);\n  return x;\n}"
    );
}

#[test]
fn a_literal_right_hand_side_still_crosses_a_statement() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_LITERAL_AFTER_A_STATEMENT, boxed);
    assert_eq!(
        code.trim(),
        "function f() {\n  var x = \"lit\";\n  g();\n  return x;\n}"
    );
}

#[test]
fn a_function_scoped_member_read_right_hand_side_is_left_alone() {
    left_alone(A_FUNCTION_SCOPED_MEMBER_READ_RIGHT_HAND_SIDE);
}

#[test]
fn a_function_scoped_var_keeps_the_same_list_rule() {
    let alloc = Allocator::default();
    let code = default_run(&alloc, A_FUNCTION_SCOPED_VAR_WITH_A_CALL_BETWEEN, boxed);
    assert_eq!(
        code.trim(),
        "function g() {\n  var x = 1;\n  f();\n  return x;\n}"
    );
}

const DEAD_ASSIGNMENT_NOTES: &[&str] = &[notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE];

fn dead_assignment_notes_of(source: &str) -> Vec<String> {
    let (_code, all) = common::run_all_stages_extended(source);
    all.into_iter()
        .filter(|note| DEAD_ASSIGNMENT_NOTES.iter().any(|text| note.contains(text)))
        .collect()
}

#[test]
fn a_shape_that_does_not_match_is_refused_without_a_note() {
    for source in [
        A_DECLARATION_WITH_AN_INIT,
        MORE_THAN_ONE_DECLARATOR,
        A_SECOND_ASSIGNMENT,
        A_SAME_NAMED_INNER_BINDING,
        TWO_EMPTY_DECLARATIONS,
        A_NON_CONSTANT_RIGHT_HAND_SIDE,
        A_MEMBER_READ_RIGHT_HAND_SIDE,
        A_ROOT_VAR_WITH_A_CALL_BETWEEN,
        READ_BEFORE_THE_ASSIGNMENT,
        AN_ASSIGNMENT_A_CLOSURE_READS,
        A_THROW_INTO_A_CATCH_THAT_READS_IT,
        A_THROW_WITH_THE_READ_AFTER_THE_TRY,
        A_BREAK_THAT_SKIPS_THE_WRITE,
        A_CONTINUE_THAT_SKIPS_THE_WRITE,
        A_NESTED_INITIALISED_REDECLARATION,
        A_FOR_OF_REDECLARATION,
        A_FOR_IN_REDECLARATION,
        A_VAR_REDECLARING_A_PARAMETER,
        A_FOLDED_CALL_AFTER_A_STATEMENT,
        A_FUNCTION_SCOPED_MEMBER_READ_RIGHT_HAND_SIDE,
    ] {
        assert_eq!(
            dead_assignment_notes_of(source),
            Vec::<String>::new(),
            "{source}"
        );
    }
}

#[test]
fn a_refusal_that_follows_a_matched_shape_is_noted() {
    assert_eq!(
        dead_assignment_notes_of(A_FUNCTION_SCOPED_READ_BEFORE_THE_ASSIGNMENT),
        vec![notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE.to_string()]
    );
    assert_eq!(
        dead_assignment_notes_of(AN_ASSIGNMENT_A_NESTED_CLOSURE_READS),
        vec![notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE.to_string()]
    );
}

#[test]
fn the_pass_is_named_and_declares_its_mode_and_safety() {
    let pass: Box<dyn Pass<'_>> = boxed();
    assert_eq!(pass.name(), "dead-assignment-elimination");
    assert!(pass.requires_extended());
    assert_eq!(pass.safety(), Safety::Safe);
}

const A_READ_THE_FLATTENER_MOVES_AHEAD_OF_THE_ASSIGNMENT: &str = "function f(g) {\n  var d = \"2|1|0\".split(\"|\");\n  var e = 0;\n  while (true) {\n    switch (d[e++]) {\n      case \"0\":\n        x = 5;\n        continue;\n      case \"1\":\n        g(x);\n        continue;\n      case \"2\":\n        var x;\n        continue;\n    }\n    break;\n  }\n  return x;\n}\nf(console.log);";

const A_READ_BESIDE_THE_DECLARATION_THE_FLATTENER_MOVES_AHEAD: &str = "function f(g) {\n  var d = \"1|0\".split(\"|\");\n  var e = 0;\n  while (true) {\n    switch (d[e++]) {\n      case \"0\":\n        x = \"late\";\n        continue;\n      case \"1\":\n        var x;\n        g(x);\n        continue;\n    }\n    break;\n  }\n}\nf(console.log);";

fn every_lane(source: &str) -> [(String, Vec<String>); 2] {
    [
        common::run_all_stages_extended(source),
        common::run_all_stages_safe_lane(source),
    ]
}

#[test]
fn a_read_the_flattener_moved_ahead_of_the_assignment_keeps_the_declaration_empty() {
    for (code, all) in every_lane(A_READ_THE_FLATTENER_MOVES_AHEAD_OF_THE_ASSIGNMENT) {
        assert!(code.contains("g(x);\n  x = 5;"), "{code}");
        assert!(code.contains("var x;"), "{code}");
        assert!(
            all.iter()
                .any(|note| note.contains(notes::DEAD_ASSIGNMENT_READ_BEFORE_THE_WRITE)),
            "{all:?}"
        );
    }
}

#[test]
fn a_read_beside_the_declaration_moved_ahead_of_the_assignment_keeps_it_empty() {
    for (code, _all) in every_lane(A_READ_BESIDE_THE_DECLARATION_THE_FLATTENER_MOVES_AHEAD) {
        assert!(code.contains("var x;\n  g(x);\n  x = \"late\";"), "{code}");
    }
}
