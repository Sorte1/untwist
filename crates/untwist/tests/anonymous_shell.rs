use untwist::{Options, Output, Pipeline, ANONYMOUS_SHELL_KEPT};
use untwist_core::pass::NoteKind;
use untwist_deobfuscate::directives::ShellRefusal;

const SHELL: &str = "(function anonymous(b475796ed633d5fd0485, f9e2c8d63a4b7128b031) {\n  (function () {\n    \"use strict\";\n    console.log(1);\n  })();\n})";

fn options(extended: bool, deobfuscate: bool) -> Options {
    Options {
        unpack: false,
        jsx: false,
        deobfuscate,
        extended,
        ..Options::default()
    }
}

fn run(source: &str) -> Output {
    Pipeline::new(options(true, true)).run(source)
}

fn strip_notes(out: &Output) -> Vec<&untwist::Note> {
    out.notes
        .iter()
        .filter(|note| note.pass == "strip-shell")
        .collect()
}

fn assert_kept(out: &Output, reason: ShellRefusal) {
    assert!(out.code().contains("function anonymous"), "{}", out.code());
    assert!(
        out.stages.iter().all(|stage| stage.stage != "strip-shell"),
        "{:?}",
        out.stages
    );
    let notes = strip_notes(out);
    assert_eq!(notes.len(), 1, "{:?}", out.notes);
    assert_eq!(notes[0].kind, NoteKind::Refusal);
    assert_eq!(
        notes[0].message,
        format!("{ANONYMOUS_SHELL_KEPT} {}", reason.reason())
    );
    assert!(notes[0].span.is_some(), "{:?}", notes[0]);
}

fn assert_not_a_shell(out: &Output) {
    assert!(
        out.stages.iter().all(|stage| stage.stage != "strip-shell"),
        "{:?}",
        out.stages
    );
    assert!(strip_notes(out).is_empty(), "{:?}", out.notes);
}

#[test]
fn the_shell_is_stripped_in_both_modes_and_without_the_deobfuscate_stage() {
    for (extended, deobfuscate) in [(true, true), (false, true), (true, false)] {
        let out = Pipeline::new(options(extended, deobfuscate)).run(SHELL);
        assert!(!out.code().contains("function anonymous"), "{}", out.code());
        assert!(out.code().contains("console.log(1)"), "{}", out.code());
        assert_eq!(out.parse_errors, 0, "{:?}", out.parse_diagnostics);
    }
}

#[test]
fn a_called_anonymous_function_is_left_alone() {
    let out = run("(function anonymous(a) {\n  use(a);\n})(1);");
    assert!(out.code().contains("function anonymous"), "{}", out.code());
    assert_not_a_shell(&out);
}

#[test]
fn a_shell_whose_parameters_are_read_is_stripped_and_its_parameters_hoisted() {
    let out = run("(function anonymous(a, b) {\n  use(a);\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
    assert_eq!(out.code(), "var a;\nvar b;\nuse(a);");
    assert_eq!(out.parse_errors, 0, "{:?}", out.parse_diagnostics);
}

#[test]
fn a_shell_whose_parameter_is_read_inside_a_nested_function_is_stripped_too() {
    let out = run("(function anonymous(a, b) {\n  function inner() {\n    use(a);\n  }\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
    assert!(out.code().starts_with("var a;\nvar b;\n"), "{}", out.code());
    assert!(out.code().contains("use(a)"), "{}", out.code());
}

#[test]
fn a_shell_whose_parameter_name_is_declared_at_the_body_top_level_is_left_alone() {
    for declaration in [
        "function b() {}",
        "let b = 1;",
        "const b = 1;",
        "class b {}",
        "let { b } = x;",
        "const [a, b] = x;",
    ] {
        let out = run(&format!(
            "(function anonymous(a, b) {{\n  {declaration}\n  use(b);\n}})"
        ));
        assert_kept(&out, ShellRefusal::ParameterDeclared);
    }
}

#[test]
fn a_body_level_var_of_a_parameter_name_does_not_block_the_strip() {
    let out = run("(function anonymous(a) {\n  var a = 1;\n  use(a);\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
    assert_eq!(out.code(), "var a;\nvar a = 1;\nuse(a);");
    assert_eq!(out.parse_errors, 0, "{:?}", out.parse_diagnostics);
}

#[test]
fn a_shell_with_a_defaulted_parameter_is_left_alone() {
    let out = run("(function anonymous(a = 1, b) {\n  y();\n})");
    assert!(out.code().contains("function anonymous"), "{}", out.code());
    assert_not_a_shell(&out);
}

#[test]
fn a_shell_whose_parameter_is_shadowed_and_never_read_is_stripped() {
    let out = run("(function anonymous(a, b) {\n  function inner(a) {\n    use(a);\n  }\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
}

#[test]
fn a_shell_that_reads_this_is_left_alone() {
    let out = run("(function anonymous(a, b) {\n  use(this);\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_that_reads_this_inside_an_arrow_is_left_alone() {
    let out = run("(function anonymous(a, b) {\n  const f = () => this;\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_that_reads_arguments_is_left_alone() {
    let out = run("(function anonymous(a, b) {\n  use(arguments);\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_that_returns_is_left_alone() {
    let out = run("(function anonymous(a, b) {\n  if (x) return;\n  y();\n})");
    assert_kept(&out, ShellRefusal::Returns);
    assert_eq!(out.parse_errors, 0, "{:?}", out.parse_diagnostics);
}

#[test]
fn a_return_inside_a_nested_function_does_not_stop_the_strip() {
    let out = run("(function anonymous(a, b) {\n  function f() {\n    return 1;\n  }\n  const g = () => {\n    return 2;\n  };\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
}

#[test]
fn a_program_with_other_statements_is_left_alone() {
    let out = run("x();\n(function anonymous(a) {\n  y();\n})");
    assert!(out.code().contains("function anonymous"), "{}", out.code());
    assert_not_a_shell(&out);
}

#[test]
fn a_differently_named_function_expression_is_left_alone() {
    let out = run("(function main(a, b) {\n  y();\n})");
    assert!(out.code().contains("function main"), "{}", out.code());
    assert_not_a_shell(&out);
}

#[test]
fn the_strip_says_it_happened_and_reports_its_own_stage() {
    let out = run(SHELL);
    assert!(
        out.notes.iter().any(|note| {
            note.pass == "strip-shell"
                && note.message.starts_with(untwist::ANONYMOUS_SHELL_STRIPPED)
                && note.message.ends_with(
                    "2 parameters hoisted as var: b475796ed633d5fd0485, f9e2c8d63a4b7128b031",
                )
        }),
        "{:?}",
        out.notes
    );
    let strip = out
        .stages
        .iter()
        .find(|stage| stage.stage == "strip-shell")
        .expect("a strip-shell stage row");
    assert_eq!((strip.iterations, strip.changes), (1, 1));
    assert_eq!(
        out.stages.first().map(|stage| stage.stage),
        Some("strip-shell")
    );
}

#[test]
fn a_program_without_a_shell_has_no_strip_stage_row() {
    let out = run("console.log(1);");
    assert_not_a_shell(&out);
}

#[test]
fn a_shell_whose_body_reads_its_own_name_is_left_alone() {
    let out = run("(function anonymous(a) {\n  use(anonymous);\n})");
    assert_kept(&out, ShellRefusal::ReferencesAnonymous);
}

#[test]
fn a_shell_whose_nested_function_reads_its_own_name_is_left_alone() {
    let out =
        run("(function anonymous(a) {\n  function f() {\n    return typeof anonymous;\n  }\n})");
    assert_kept(&out, ShellRefusal::ReferencesAnonymous);
}

#[test]
fn a_shell_with_a_block_level_function_named_like_a_parameter_is_left_alone() {
    let out = run("(function anonymous(a) {\n  {\n    function a() {}\n  }\n  use(a);\n})");
    assert_kept(&out, ShellRefusal::ParameterDeclared);
}

#[test]
fn a_shell_whose_own_level_uses_the_identifier_await_is_left_alone() {
    for (parameter, body) in [
        ("a", "await(a);"),
        ("a", "var await = a;"),
        ("await", "y();"),
    ] {
        let out = run(&format!(
            "(function anonymous({parameter}) {{\n  {body}\n}})"
        ));
        assert_kept(&out, ShellRefusal::UsesAwait);
    }
}

#[test]
fn a_shell_with_a_repeated_parameter_is_left_alone() {
    let out = run("(function anonymous(a, a) {\n  use(a);\n})");
    assert_kept(&out, ShellRefusal::RepeatedParameter);
}

#[test]
fn a_shell_that_reads_new_target_is_left_alone() {
    let out = run("(function anonymous(a) {\n  use(new.target);\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_that_reads_super_in_a_class_field_is_left_alone() {
    let out = run("(function anonymous(a) {\n  class C extends B {\n    x = super.y;\n  }\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_that_reads_import_meta_is_left_alone() {
    let out = run("(function anonymous(a) {\n  use(import.meta);\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_shell_with_a_class_field_initialiser_reading_this_is_left_alone() {
    let out = run("(function anonymous(a) {\n  class C {\n    x = this;\n  }\n})");
    assert_kept(&out, ShellRefusal::ReadsReceiver);
}

#[test]
fn a_this_inside_a_nested_function_expression_does_not_stop_the_strip() {
    let out =
        run("(function anonymous(a) {\n  const f = function () {\n    return this;\n  };\n})");
    assert!(!out.code().contains("function anonymous"), "{}", out.code());
    assert!(out.code().contains("return this"), "{}", out.code());
}

#[test]
fn a_shell_with_no_parameters_is_stripped_without_a_var_line() {
    let out = run("(function anonymous(\n) {\n  console.log(1);\n})");
    assert_eq!(out.code(), "console.log(1);");
    assert!(
        out.notes
            .iter()
            .any(|note| note.pass == "strip-shell"
                && note.message.ends_with("; it had no parameters")),
        "{:?}",
        out.notes
    );
}

#[test]
fn a_shell_with_one_parameter_is_reported_in_the_singular() {
    let out = run("(function anonymous(a) {\n  use(a);\n})");
    assert_eq!(out.code(), "var a;\nuse(a);");
    assert!(
        out.notes.iter().any(|note| note.pass == "strip-shell"
            && note.message.ends_with("; 1 parameter hoisted as var: a")),
        "{:?}",
        out.notes
    );
}

const DECLARATION_SHELL: &str = "function anonymous(b475796ed633d5fd0485,f9e2c8d63a4b7128b031\n) {\n(function () {\n  \"use strict\";\n  console.log(1);\n})();\n}";

#[test]
fn the_raw_to_string_declaration_form_is_stripped_in_both_modes() {
    for (extended, deobfuscate) in [(true, true), (false, true), (true, false)] {
        let out = Pipeline::new(options(extended, deobfuscate)).run(DECLARATION_SHELL);
        assert!(!out.code().contains("function anonymous"), "{}", out.code());
        assert_eq!(
            out.code(),
            "var b475796ed633d5fd0485;\nvar f9e2c8d63a4b7128b031;\n(function () {\n  \"use strict\";\n  console.log(1);\n})();"
        );
        assert_eq!(
            out.stages.first().map(|stage| stage.stage),
            Some("strip-shell")
        );
        assert_eq!(out.parse_errors, 0, "{:?}", out.parse_diagnostics);
    }
}

#[test]
fn a_declaration_shell_that_returns_is_left_alone_as_a_declaration() {
    let out = run("function anonymous(a) {\n  return a;\n}");
    assert_kept(&out, ShellRefusal::Returns);
    assert_eq!(out.code(), "function anonymous(a) {\n  return a;\n}");
}

#[test]
fn a_differently_named_function_declaration_is_left_alone() {
    let out = run("function main(a, b) {\n  y();\n}");
    assert!(out.code().contains("function main"), "{}", out.code());
    assert_not_a_shell(&out);
}
