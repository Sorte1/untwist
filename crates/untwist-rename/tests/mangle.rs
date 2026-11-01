mod common;

use common::{assert_scope_safe, rename, render};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, Note, PassGroup, PassOptions, RunReport};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_rename::{rename_passes, RenameConfig, RenameMode};

fn config() -> RenameConfig {
    RenameConfig {
        mode: RenameMode::Mangle,
        rename_all: false,
        extended: true,
    }
}

fn conservative_config() -> RenameConfig {
    RenameConfig {
        extended: false,
        ..config()
    }
}

fn mangle(source: &str) -> String {
    let output = rename(source, config());
    assert_scope_safe(source, &output);
    output
}

fn mangle_for_conservative(source: &str) -> String {
    let output = rename(source, conservative_config());
    assert_scope_safe(source, &output);
    output
}

const NAMED_FUNCTION: &str = "var named = function inner() {\n  return 2;\n};";
const NAMED_CLASS: &str = "var Named = class Inner {};";
const IIFE: &str = "var result = function inner() {\n  return 2;\n}();";
const HOISTED_CALLEE: &str = "var user = getUser();\nfunction getUser() {\n  return 1;\n}";
const FORWARD_ALIAS: &str = "var a = b;\nvar b = 1;";
const CYCLE: &str = "var a = b;\nvar b = a;";
const PINNED_NAME: &str = "var named = function inner() {\n  ({ inner } = o);\n};";

fn run_report(source: &str) -> (String, RunReport) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::new(rename_passes(config()), PassOptions::default());
    let (_sem, report) = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
    (print(&parsed.program, PrintStyle::Pretty), report)
}

fn hazard_notes(notes: &[Note]) -> Vec<&Note> {
    notes
        .iter()
        .filter(|note| note.message.contains("may defeat scope analysis"))
        .collect()
}

#[test]
fn variable_initialisers_drive_the_name() {
    assert_eq!(mangle("let x = 1;"), "let vLN1 = 1;");
    assert_eq!(mangle("let x = exports;"), "let vExports = exports;");
    assert_eq!(mangle("let x = () => {};"), "let vF = () => {};");
    assert_eq!(mangle("let x = class {};"), "let vC = class {};");
    assert_eq!(mangle("let x = Array(100);"), "let vArray = Array(100);");
    assert_eq!(mangle("let x = [];"), "let vA = [];");
    assert_eq!(mangle("let x = {};"), "let vO = {};");
    assert_eq!(mangle("let [x] = 1;"), "let [v] = 1;");
    assert_eq!(mangle("let x = true;"), "let v = true;");
}

#[test]
fn require_calls_name_the_binding_after_the_module() {
    assert_eq!(
        mangle(r#"const x = require("fs");"#),
        r#"const fs = require("fs");"#
    );
    assert_eq!(
        mangle("const x = require(\"node:fs\");\nconst y = require(\"node:fs\");"),
        "const nodeFs = require(\"node:fs\");\nconst nodeFs2 = require(\"node:fs\");"
    );
}

#[test]
fn declaration_order_drives_the_numeric_suffix() {
    assert_eq!(
        mangle("let a = 100;\nlet b = 200;\nlet c = 300;"),
        "let vLN100 = 100;\nlet vLN200 = 200;\nlet vLN300 = 300;"
    );
}

#[test]
fn a_redeclared_binding_takes_the_last_declarations_name() {
    assert_eq!(
        mangle("var x = 1; var x = 2;"),
        "var vLN2 = 1;\nvar vLN2 = 2;"
    );
}

#[test]
fn a_parameter_redeclared_as_a_var_takes_the_variable_name() {
    assert_eq!(
        mangle("function f(p) { var p; return p; }"),
        "function f(v) {\n  var v;\n  return v;\n}"
    );
}

#[test]
fn every_declaration_site_of_a_redeclared_binding_costs_a_uid() {
    assert_eq!(
        mangle("var x = 3; var y = 3; var x = w.q();"),
        "var v = 3;\nvar vLN32 = 3;\nvar v = w.q();"
    );
    assert_eq!(
        mangle("function f(a, b) { var [a, b] = z; } function g(d) {}"),
        "function f(v, v2) {\n  var [v, v2] = z;\n}\nfunction f2(p3) {}"
    );
    assert_eq!(
        mangle("var x = 1; var x = 1; let y = 1;"),
        "var vLN12 = 1;\nvar vLN12 = 1;\nlet vLN13 = 1;"
    );
}

#[test]
fn a_redeclared_binding_is_named_at_its_last_declaration() {
    assert_eq!(
        mangle("var x = 3; var y = q.r(); var x = q.s();"),
        "var v2 = 3;\nvar v = q.r();\nvar v2 = q.s();"
    );
    assert_eq!(
        mangle("var x = 1; var y = x; var x = q.s(); var z = x;"),
        "var v = 1;\nvar vVLN1 = v;\nvar v = q.s();\nvar vV = v;"
    );
}

#[test]
fn a_require_call_through_a_renamed_callee_is_named_after_the_callee() {
    assert_eq!(
        mangle("(function(require) { var x = require(\"vscode\"); var y = x; })();"),
        "(function (p) {\n  var vP = p(\"vscode\");\n  var vVP = vP;\n})();"
    );
    assert_eq!(
        mangle("var x = require(\"a\"); function require() {}"),
        "var vF = f(\"a\");\nfunction f() {}"
    );
    assert_eq!(
        mangle_for_conservative("var x = require(\"a\"); function require() {}"),
        "var a = f(\"a\");\nfunction f() {}"
    );
}

#[test]
fn string_initialisers_are_title_cased_and_truncated() {
    assert_eq!(
        mangle("let a = \"hello world\";\nlet b = \"foo-bar-🗿-ä\";"),
        "let vLSHelloWorld = \"hello world\";\nlet vLSFoobar = \"foo-bar-🗿-ä\";"
    );
    assert_eq!(mangle(r#"let x = "";"#), r#"let vLS = "";"#);
    let long = "a".repeat(1000);
    let expected = format!("let vLSA{} = \"{long}\";", "a".repeat(19));
    assert_eq!(mangle(&format!("let x = \"{long}\";")), expected);
}

#[test]
fn exports_are_never_renamed() {
    assert_eq!(mangle("export const x = 1;"), "export const x = 1;");
    assert_eq!(mangle("export class X {}"), "export class X {}");
    assert_eq!(
        mangle("export default function x() {}"),
        "export default function f() {}"
    );
}

#[test]
fn declarations_parameters_and_catch_clauses() {
    assert_eq!(mangle("class abc {}"), "class C {}");
    assert_eq!(mangle("function abc() {}"), "function f() {}");
    assert_eq!(
        mangle("(x, y, z) => x + y + z;"),
        "(p, p2, p3) => p + p2 + p3;"
    );
    assert_eq!(mangle("(x = 1) => x;"), "(p = 1) => p;");
    assert_eq!(mangle("try {} catch (x) {}"), "try {} catch (e) {}");
}

#[test]
fn import_specifiers_and_pattern_properties_keep_their_names() {
    assert_eq!(
        mangle(r#"import { sin as k } from "m"; k();"#),
        "import { sin as k } from \"m\";\nk();"
    );
    assert_eq!(
        mangle("const { cos: v } = Math; v();"),
        "const { cos: v } = Math;\nv();"
    );
}

#[test]
fn a_member_callee_yields_no_suffix_and_a_nested_call_recurses() {
    assert_eq!(
        mangle("let x = s.concat(a).concat(b);"),
        "let v = s.concat(a).concat(b);"
    );
    assert_eq!(mangle("let x = f()();"), "let vF = f()();");
    assert_eq!(mangle("let x = new Thing();"), "let v = new Thing();");
}

#[test]
fn nested_pattern_bindings_keep_their_names() {
    assert_eq!(mangle("let [x = 1] = a;"), "let [x = 1] = a;");
    assert_eq!(mangle("let [...x] = a;"), "let [ ...x] = a;");
}

#[test]
fn mangling_an_already_mangled_program_changes_nothing() {
    let once = mangle("let x = 1; function abc(q) { return q; }");
    let twice = mangle(&once);
    assert_eq!(once, twice);

    let redeclared =
        mangle("var x = 1; var x = 1; let y = 1; var z = 3; var w = 3; var z = q.s();");
    assert_eq!(mangle(&redeclared), redeclared);
}

#[test]
fn the_driver_settles_after_a_single_renaming_iteration() {
    let (output, report) = run_report("let x = 1; function abc(q) { return q; eval('x'); }");
    assert_eq!(report.iterations, 2);
    assert!(report.converged(), "stop={:?}", report.stop);
    assert_eq!(report.per_pass, vec![("mangle", 3)]);
    assert_eq!(hazard_notes(&report.notes).len(), 1);
    assert!(output.starts_with("let vLN1 = 1;"));
}

#[test]
fn a_scope_hazard_is_reported_once_per_run() {
    let (_, notes) = render("let x = 1; eval('x');", config(), PrintStyle::Pretty, false);
    let hazards = hazard_notes(&notes);
    assert_eq!(hazards.len(), 1);
    assert_eq!(hazards[0].pass, "mangle");
    assert_eq!(
        hazards[0].message,
        "direct eval may defeat scope analysis; renaming anyway"
    );

    let (_, with_notes) = render(
        "with (o) { let y = 2; }",
        config(),
        PrintStyle::Pretty,
        false,
    );
    let with_hazards = hazard_notes(&with_notes);
    assert_eq!(with_hazards.len(), 1);
    assert_eq!(
        with_hazards[0].message,
        "with statement may defeat scope analysis; renaming anyway"
    );
}

#[test]
fn a_var_holding_a_named_function_expression_derives_from_the_mangled_name() {
    assert_eq!(
        mangle(NAMED_FUNCTION),
        "var vF = function f() {\n  return 2;\n};"
    );
}

#[test]
fn mangling_a_var_holding_a_named_function_expression_twice_changes_nothing() {
    let once = mangle(NAMED_FUNCTION);
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_var_holding_a_named_class_expression_derives_from_the_mangled_name() {
    let once = mangle(NAMED_CLASS);
    assert_eq!(once, "var vC = class C {};");
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_var_holding_a_call_of_a_named_function_expression_derives_from_the_mangled_callee() {
    let once = mangle(IIFE);
    assert_eq!(once, "var vF = function f() {\n  return 2;\n}();");
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_var_holding_a_call_of_a_hoisted_function_derives_from_the_mangled_callee() {
    let once = mangle(HOISTED_CALLEE);
    assert_eq!(once, "var vF = f();\nfunction f() {\n  return 1;\n}");
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_var_aliasing_a_later_declaration_derives_from_the_mangled_alias() {
    let once = mangle(FORWARD_ALIAS);
    assert_eq!(once, "var vVLN1 = vLN1;\nvar vLN1 = 1;");
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_cyclic_read_contributes_no_suffix() {
    let once = mangle(CYCLE);
    assert_eq!(once, "var vV = v;\nvar v = vV;");
    assert_eq!(mangle(&once), once);
}

#[test]
fn a_pinned_function_expression_name_still_names_the_var_from_its_original_name() {
    let once = mangle(PINNED_NAME);
    assert_eq!(
        once,
        "var vInner = function inner() {\n  ({inner} = o);\n};"
    );
    assert_eq!(mangle(&once), once);
}

#[test]
fn under_conservative_a_var_derives_from_the_name_its_source_had() {
    assert_eq!(
        mangle_for_conservative(NAMED_FUNCTION),
        "var vInner = function f() {\n  return 2;\n};"
    );
    assert_eq!(
        mangle_for_conservative(NAMED_CLASS),
        "var vInner = class C {};"
    );
    assert_eq!(
        mangle_for_conservative(IIFE),
        "var vInner = function f() {\n  return 2;\n}();"
    );
    assert_eq!(
        mangle_for_conservative(HOISTED_CALLEE),
        "var vGetUser = f();\nfunction f() {\n  return 1;\n}"
    );
    assert_eq!(
        mangle_for_conservative(FORWARD_ALIAS),
        "var vB = vLN1;\nvar vLN1 = 1;"
    );
}

fn mangle_twice(source: &str) -> (String, String) {
    let once = mangle(source);
    let twice = mangle(&once);
    (once, twice)
}

#[test]
fn a_name_released_by_a_renamed_variable_is_free_for_a_function_on_the_first_run() {
    let (once, twice) = mangle_twice("var f = 1; function g() { return f; } g();");
    assert_eq!(
        once,
        "var vLN1 = 1;\nfunction f() {\n  return vLN1;\n}\nf();"
    );
    assert_eq!(twice, once);
}

#[test]
fn under_conservative_a_renamed_variables_original_name_stays_refused() {
    assert_eq!(
        mangle_for_conservative("var f = 1; function g() { return f; } g();"),
        "var vLN1 = 1;\nfunction f2() {\n  return vLN1;\n}\nf2();"
    );
}

#[test]
fn a_function_already_named_f_keeps_it_and_the_next_one_takes_f2() {
    let (once, twice) = mangle_twice("function f() {}\nfunction g() {}");
    assert_eq!(once, "function f() {}\nfunction f2() {}");
    assert_eq!(twice, once);
}

#[test]
fn a_function_decided_first_takes_f_from_a_later_function_that_holds_it() {
    let (once, twice) = mangle_twice("function g() {}\nfunction f() {}");
    assert_eq!(once, "function f() {}\nfunction f2() {}");
    assert_eq!(twice, once);
}

#[test]
fn a_global_reference_keeps_its_name_off_the_table() {
    let (once, twice) = mangle_twice("function g() { return f; }");
    assert_eq!(once, "function f2() {\n  return f;\n}");
    assert_eq!(twice, once);
}

#[test]
fn a_pinned_holder_keeps_its_name_off_the_table() {
    let (once, twice) = mangle_twice("export function f() {}\nfunction g() {}");
    assert_eq!(once, "export function f() {}\nfunction f2() {}");
    assert_eq!(twice, once);
}

#[test]
fn a_holder_whose_role_infers_no_name_keeps_its_name_off_the_table() {
    let (once, twice) = mangle_twice("const { f } = o;\nfunction g() {}\nf();");
    assert_eq!(once, "const { f } = o;\nfunction f2() {}\nf();");
    assert_eq!(twice, once);
}

#[test]
fn a_binding_that_exhausts_its_candidates_leaves_the_program_unrenamed() {
    let mut source = String::new();
    for index in 2..1000 {
        source.push_str(&format!("f{index};"));
    }
    source.push_str("function g() {}\nfunction f() {}");
    for (extended, starved) in [(true, "f"), (false, "g")] {
        let config = RenameConfig {
            extended,
            ..config()
        };
        let (output, notes) = render(&source, config, PrintStyle::Compact, false);
        assert!(output.ends_with("function g(){}function f(){}"), "{output}");
        let aborts: Vec<&Note> = notes
            .iter()
            .filter(|note| note.message.contains("ran out of names"))
            .collect();
        assert_eq!(aborts.len(), 1, "{notes:?}");
        assert_eq!(
            aborts[0].message,
            format!("ran out of names for {starved}; leaving the program unrenamed")
        );
    }
}
