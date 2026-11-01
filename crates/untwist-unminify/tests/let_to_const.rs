use untwist_core::pass::PassOptions;
use untwist_testing::{transform_report, transform_with, Allocator};

fn run(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[untwist_unminify::let_to_const::boxed],
        PassOptions {
            unsafe_transforms: false,
            extended,
            max_iterations: None,
            fuel: None,
            ..PassOptions::default()
        },
    )
}

fn in_function(body: &str) -> String {
    format!("function f() {{\n  {body}\n}}")
}

#[test]
fn a_never_written_let_becomes_const() {
    assert_eq!(
        run(&in_function("let a = 1;\n  use(a);"), true),
        in_function("const a = 1;\n  use(a);")
    );
}

#[test]
fn conservative_leaves_every_let_alone() {
    let source = in_function("let a = 1;\n  use(a);");
    assert_eq!(run(&source, false), source);
}

#[test]
fn a_written_let_stays_a_let() {
    let source = in_function("let a = 1;\n  a = 2;\n  use(a);");
    assert_eq!(run(&source, true), source);
}

#[test]
fn a_let_without_an_initialiser_stays_a_let() {
    let source = in_function("let a;\n  use(a);");
    assert_eq!(run(&source, true), source);
}

#[test]
fn a_for_head_let_stays_a_let() {
    assert_eq!(
        run("for (let i = 0; i < 3; i++) use(i);", true),
        "for (let i = 0; i < 3; i++) use(i);"
    );
}

#[test]
fn a_for_of_head_let_stays_a_let() {
    assert_eq!(
        run("for (let x of xs) use(x);", true),
        "for (let x of xs) use(x);"
    );
}

#[test]
fn a_mixed_declaration_is_split() {
    assert_eq!(
        run(
            &in_function("let a = 1, b = 2;\n  b = 3;\n  use(a, b);"),
            true
        ),
        in_function("const a = 1;\n  let b = 2;\n  b = 3;\n  use(a, b);")
    );
}

#[test]
fn a_destructuring_let_is_left_alone() {
    let source = in_function("let { a, b } = o;\n  use(a, b);");
    assert_eq!(run(&source, true), source);
}

#[test]
fn a_var_is_never_touched() {
    assert_eq!(run("var a = 1;\nuse(a);", true), "var a = 1;\nuse(a);");
}

#[test]
fn a_program_with_direct_eval_keeps_every_let() {
    let source = in_function("let a = 1;\n  eval(\"a = 2\");\n  use(a);");
    for extended in [true, false] {
        let alloc = Allocator::default();
        let (code, report) = transform_report(
            &alloc,
            &source,
            &[untwist_unminify::let_to_const::boxed],
            PassOptions {
                unsafe_transforms: false,
                extended,
                max_iterations: None,
                fuel: None,
                ..PassOptions::default()
            },
        );
        assert_eq!(code, source);
        assert_eq!(
            report
                .notes
                .iter()
                .any(|note| note.message == untwist_unminify::let_to_const::SCOPE_HAZARD),
            extended,
            "{:?}",
            report.notes
        );
    }
}

#[test]
fn a_for_in_head_let_stays_a_let() {
    assert_eq!(
        run("for (let k in o) use(k);", true),
        "for (let k in o) use(k);"
    );
}

#[test]
fn an_unwritten_for_head_let_stays_a_let() {
    assert_eq!(
        run("for (let i = 0; i < 3;) {\n  use(i);\n  break;\n}", true),
        "for (let i = 0; i < 3;) {\n  use(i);\n  break;\n}"
    );
}

#[test]
fn a_scripts_top_level_let_stays_a_let() {
    let source = "let x = 1;\n(0, eval)(\"x = 2\");\nuse(x);";
    assert_eq!(run(source, true), source);
}

#[test]
fn a_strict_mode_clean_script_is_still_a_script() {
    let source = "\"use strict\";\nlet x = 1;\nuse(x);";
    assert_eq!(run(source, true), source);
}

#[test]
fn a_modules_top_level_let_becomes_const() {
    assert_eq!(
        run(
            "let x = 1;\n(0, eval)(\"x = 2\");\nuse(x);\nexport {};",
            true
        ),
        "const x = 1;\n(0, eval)(\"x = 2\");\nuse(x);\nexport {};"
    );
}

#[test]
fn an_import_makes_the_program_a_module_too() {
    assert_eq!(
        run("import m from \"m\";\nlet x = m;\nuse(x);", true),
        "import m from \"m\";\nconst x = m;\nuse(x);"
    );
}

#[test]
fn a_scripts_function_level_let_still_becomes_const() {
    assert_eq!(
        run(
            "function f() {\n  let y = 1;\n  return y;\n}\nuse(f());",
            true
        ),
        "function f() {\n  const y = 1;\n  return y;\n}\nuse(f());"
    );
}
