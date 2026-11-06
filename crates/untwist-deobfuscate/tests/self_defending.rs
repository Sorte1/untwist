mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};

const CONTROLLER: &str = "var e = (function () {\n  var f = !![];\n  return function (g, h) {\n    var i = f ? function () {\n      if (h) {\n        var j = h['apply'](g, arguments);\n        h = null;\n        return j;\n      }\n    } : function () {};\n    f = ![];\n    return i;\n  };\n})();\n";

fn run<'a>(alloc: &'a Allocator, source: &'a str) -> String {
    common::run_ctors(alloc, source, &untwist_deobfuscate::protections_ctors()).0
}

fn run_with_notes<'a>(alloc: &'a Allocator, source: &'a str) -> (String, Vec<String>) {
    common::run_ctors(alloc, source, &untwist_deobfuscate::protections_ctors())
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn the_controller_and_its_immediate_call_are_removed() {
    let alloc = Allocator::default();
    let source = format!(
        "function hi() {{\n{CONTROLLER}  (function () {{\n    e(this, function () {{ return 1; }})();\n  }})();\n  console.log('x');\n}}"
    );
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "function hi() {\n  console.log(\"x\");\n}");
}

#[test]
fn the_variable_form_is_removed_with_its_calls() {
    let alloc = Allocator::default();
    let source = format!(
        "function hi() {{\n{CONTROLLER}  var sd = e(this, function () {{ return 1; }});\n  sd();\n  console.log('x');\n}}"
    );
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "function hi() {\n  console.log(\"x\");\n}");
}

#[test]
fn an_unrelated_iife_is_kept() {
    let alloc = Allocator::default();
    let source = "(function () {\n  console.log(1);\n})();";
    let out = run(&alloc, source);
    assert!(out.contains("console.log(1)"), "{out}");
    assert_eq!(out, printed(source));
}

#[test]
fn an_empty_iife_in_a_file_with_no_controller_is_kept() {
    let alloc = Allocator::default();
    let source = "(function () {})();\nconsole.log(1);";
    let out = run(&alloc, source);
    assert_eq!(out, printed(source));
}

#[test]
fn an_outer_shell_whose_inner_function_is_not_the_template_is_kept() {
    let alloc = Allocator::default();
    let source = "var e = (function () {\n  var f = !![];\n  return function (g, h) {\n    return h;\n  };\n})();\ne(this, function () {})();\nconsole.log(1);";
    let out = run(&alloc, source);
    assert!(out.contains("var e ="), "{out}");
    assert_eq!(out, printed(source));
}

#[test]
fn an_empty_iife_beside_a_controller_is_swept_with_it() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}(function () {{}})();\nconsole.log(1);");
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "console.log(1);");
}

#[test]
fn a_shell_that_carries_other_work_keeps_the_shell() {
    let alloc = Allocator::default();
    let source = format!(
        "{CONTROLLER}(function () {{\n  e(this, function () {{ return 1; }})();\n  console.log(2);\n}})();\nconsole.log(1);"
    );
    let out = run(&alloc, &source);
    assert!(out.contains("console.log(2)"), "{out}");
    assert!(!out.contains("var e ="), "{out}");
}

#[test]
fn under_conservative_a_string_literal_left_at_the_head_of_the_program_becomes_a_directive_again() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}\"use strict\";\nconsole.log(1);");
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "\"use strict\";\nconsole.log(1);");
    assert!(!out.contains("(\"use strict\")"), "{out}");
    assert_eq!(common::directive_count(&out), 1);
}

#[test]
fn under_conservative_a_string_literal_left_at_the_head_of_a_function_body_becomes_a_directive_again(
) {
    let alloc = Allocator::default();
    let source =
        format!("function hi() {{\n{CONTROLLER}  \"use strict\";\n  console.log('x');\n}}");
    let out = run(&alloc, &source);
    assert_eq!(
        out.trim(),
        "function hi() {\n  \"use strict\";\n  console.log(\"x\");\n}"
    );
    assert!(!out.contains("(\"use strict\")"), "{out}");
}

#[test]
fn a_surviving_head_that_is_not_a_string_literal_is_left_alone() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}console.log(1);");
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "console.log(1);");
    assert_eq!(common::directive_count(&out), 0);
}

#[test]
fn a_string_literal_that_is_not_at_the_head_stays_an_expression() {
    let alloc = Allocator::default();
    let source = format!("console.log(0);\n{CONTROLLER}\"use strict\";\nconsole.log(1);");
    let out = run(&alloc, &source);
    assert!(out.contains("console.log(0)"), "{out}");
    assert!(out.contains("use strict"), "{out}");
    assert_eq!(common::directive_count(&out), 0);
}

const SHADOWING_LOCAL: &str = "function work() {\n  var e = function (a, b) {\n    return b;\n  };\n  var out = e(0, function () {\n    return 42;\n  });\n  return out();\n}\nconsole.log(work());";

const A_NESTED_SCOPE_THAT_REUSES_THE_NAME: &str = "function outer() {\n  var e = function (a, b) {\n    return b;\n  };\n  var sd = e(this, function () {\n    return 7;\n  });\n  return sd();\n}\nconsole.log(outer());";

#[test]
fn a_shadowing_local_with_the_controller_name_keeps_its_own_code() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}{SHADOWING_LOCAL}");
    let out = run(&alloc, &source);
    assert_eq!(out, printed(SHADOWING_LOCAL));
}

#[test]
fn a_nested_scope_that_reuses_the_controller_name_keeps_its_own_calls() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}{A_NESTED_SCOPE_THAT_REUSES_THE_NAME}");
    let out = run(&alloc, &source);
    assert_eq!(out, printed(A_NESTED_SCOPE_THAT_REUSES_THE_NAME));
}

#[test]
fn a_controller_declared_beside_a_sibling_declarator_is_kept() {
    let alloc = Allocator::default();
    let source = format!(
        "{}console.log(keep);",
        CONTROLLER.replacen("})();\n", "})(), keep = 42;\n", 1)
    );
    let (out, notes) = run_with_notes(&alloc, &source);
    assert_eq!(out, printed(&source));
    assert!(notes.is_empty(), "{notes:?}");
}

#[test]
fn a_controller_reference_the_template_cannot_account_for_refuses_the_removal() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}console.log(typeof e);");
    let (out, notes) = run_with_notes(&alloc, &source);
    assert_eq!(out, printed(&source));
    assert!(
        notes
            .iter()
            .any(|note| note == untwist_deobfuscate::notes::SELF_DEFENDING_REFERENCE_KEPT),
        "{notes:?}"
    );
}

#[test]
fn a_block_a_control_flow_fold_respanned_is_still_removed() {
    let alloc = Allocator::default();
    let source = format!(
        "function hi() {{\n{CONTROLLER}  var cff = {{ HLAuI: function (a, b, c) {{ return a(b, c); }} }};\n  var sd = cff.HLAuI(e, this, function () {{ return sd; }});\n  sd();\n  console.log('x');\n}}"
    );
    let groups = vec![
        vec![untwist_deobfuscate::control_flow_object::boxed as untwist_core::pass::PassCtor],
        untwist_deobfuscate::protections_ctors(),
    ];
    let out = common::run_ctor_groups(&alloc, &source, &groups);
    assert_eq!(
        out.trim(),
        "function hi() {\n  console.log(\"x\");\n}",
        "the fold gives every node it splices a fresh span, so the reference to sd inside the \
         removed statement no longer sits inside that statement's span"
    );
}

fn run_extended<'a>(alloc: &'a Allocator, source: &'a str) -> String {
    common::run_ctors_extended(alloc, source, &untwist_deobfuscate::protections_ctors())
}

#[test]
fn extended_a_string_literal_left_at_the_head_of_the_program_stays_inert() {
    let alloc = Allocator::default();
    let source = format!("{CONTROLLER}\"use strict\";\nconsole.log(1);");
    let out = run_extended(&alloc, &source);
    assert_eq!(out.trim(), "(\"use strict\");\nconsole.log(1);");
    assert_eq!(common::directive_count(&out), 0);
}

#[test]
fn extended_a_string_literal_left_at_the_head_of_a_function_body_stays_inert() {
    let alloc = Allocator::default();
    let source =
        format!("function hi() {{\n{CONTROLLER}  \"use strict\";\n  console.log('x');\n}}");
    let out = run_extended(&alloc, &source);
    assert_eq!(
        out.trim(),
        "function hi() {\n  (\"use strict\");\n  console.log(\"x\");\n}"
    );
}

#[test]
fn extended_a_directive_the_input_already_had_is_kept() {
    let alloc = Allocator::default();
    let source = format!("\"use strict\";\n{CONTROLLER}console.log(1);");
    let out = run_extended(&alloc, &source);
    assert_eq!(out.trim(), "\"use strict\";\nconsole.log(1);");
    assert_eq!(common::directive_count(&out), 1);
}

#[test]
fn bare_var_declarations_left_inside_the_controller_do_not_hide_it() {
    let alloc = Allocator::default();
    let controller = CONTROLLER
        .replace(
            "if (h) {\n",
            "if (h) {\n        var k;\n        var l, m;\n",
        )
        .replace("    f = ![];\n", "    var n;\n    f = ![];\n");
    let source = format!(
        "function hi() {{\n{controller}  var sd = e(this, function () {{ return sd.toString().search('(((.+)+)+)+$'); }});\n  sd();\n  console.log('x');\n}}"
    );
    let out = run(&alloc, &source);
    assert_eq!(out.trim(), "function hi() {\n  console.log(\"x\");\n}");
}

#[test]
fn a_bare_var_that_shadows_the_controllers_flag_keeps_it() {
    let alloc = Allocator::default();
    let controller = CONTROLLER.replace("    f = ![];\n", "    var f;\n    f = ![];\n");
    let source = format!(
        "function hi() {{\n{controller}  var sd = e(this, function () {{ return 1; }});\n  sd();\n  console.log('x');\n}}"
    );
    let out = run(&alloc, &source);
    assert!(out.contains("sd()"), "{out}");
}

#[test]
fn a_bare_var_that_shadows_the_guarded_function_keeps_it() {
    let alloc = Allocator::default();
    let controller = CONTROLLER.replace("if (h) {\n", "if (h) {\n        var h;\n");
    let source = format!(
        "function hi() {{\n{controller}  var sd = e(this, function () {{ return 1; }});\n  sd();\n  console.log('x');\n}}"
    );
    let out = run(&alloc, &source);
    assert!(out.contains("sd()"), "{out}");
}
