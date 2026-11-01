use oxc_allocator::Allocator;
use std::path::PathBuf;
use untwist_core::parse::parse_program;
use untwist_core::print::{pretty_codegen, print, PrintStyle};
use untwist_core::reflow::reflow;

#[test]
fn pretty_uses_two_space_indent_and_keeps_comments() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "/* keep */ function f(a){if(a){return 1}}");
    let out = print(&p.program, PrintStyle::Pretty);
    assert!(out.contains("/* keep */"), "{out}");
    assert!(out.contains("\n  if (a) {\n    return 1;\n  }\n"), "{out}");
    assert!(!out.contains('\t'));
}

#[test]
fn compact_is_single_line_without_comments() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "/* drop */ function f(a){if(a){return 1}}");
    let out = print(&p.program, PrintStyle::Compact);
    assert_eq!(out, "function f(a){if(a){return 1}}");
}

fn pretty(source: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    print(&parsed.program, PrintStyle::Pretty)
}

#[test]
fn a_literal_array_of_any_length_stays_on_one_line() {
    assert_eq!(
        pretty("var a = [1, 2, 3, 4, 5];"),
        "var a = [1, 2, 3, 4, 5];"
    );
    assert_eq!(
        pretty("var a = [\"x\", \"y\", \"z\", \"w\"];"),
        "var a = [\"x\", \"y\", \"z\", \"w\"];"
    );
    assert_eq!(pretty("var a = [1, -2, 3.5];"), "var a = [1, -2, 3.5];");
}

#[test]
fn an_array_holding_a_multi_line_element_keeps_its_breaks() {
    let out = pretty("var a = [1, 2, function () {\n  x();\n}];");
    assert!(out.contains("function () {\n"), "{out}");
    assert!(out.lines().count() > 1, "{out}");
}

#[test]
fn a_bracket_inside_a_string_is_never_treated_as_an_array() {
    assert_eq!(
        pretty("var a = \"[1,\\n2,\\n3]\";"),
        "var a = \"[1,\\n2,\\n3]\";"
    );
}

#[test]
fn an_anonymous_function_keyword_carries_a_space() {
    assert_eq!(pretty("var f = function () {};"), "var f = function () {};");
    assert_eq!(pretty("var f = function() {};"), "var f = function () {};");
}

#[test]
fn a_named_function_is_unchanged() {
    assert_eq!(pretty("function f() {}"), "function f() {}");
    assert_eq!(
        pretty("var f = function g() {};"),
        "var f = function g() {};"
    );
}

#[test]
fn the_word_function_inside_a_string_is_untouched() {
    assert_eq!(
        pretty("var s = \"function(a)\";"),
        "var s = \"function(a)\";"
    );
}

#[test]
fn an_anonymous_async_function_gains_the_space_and_a_generator_already_has_it() {
    assert_eq!(
        pretty("var f = async function () {};"),
        "var f = async function () {};"
    );
    assert_eq!(
        pretty("var f = async function() {};"),
        "var f = async function () {};"
    );
    assert_eq!(
        pretty("var f = function* () {};"),
        "var f = function* () {};"
    );
    assert_eq!(
        pretty("var f = async function* () {};"),
        "var f = async function* () {};"
    );
}

#[test]
fn a_method_whose_parameter_default_holds_a_paren_is_untouched() {
    let out = pretty("var o = { m(abc = \"(\") {} };");
    assert!(out.contains("m(abc = \"(\")"), "{out}");
}

#[test]
fn a_class_method_and_a_getter_are_untouched() {
    let out = pretty("class A {\n  m(abc = \"(\") {}\n  get x() {\n    return 1;\n  }\n}");
    assert!(out.contains("m(abc = \"(\")"), "{out}");
    assert!(out.contains("get x() {"), "{out}");
}

#[test]
fn a_nested_array_literal_collapses_outer_last() {
    assert_eq!(
        pretty("var a = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];"),
        "var a = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];"
    );
}

#[test]
fn a_parenthesised_sequence_element_keeps_its_parentheses() {
    assert_eq!(
        pretty("var a = [(x, y), c, d];"),
        "var a = [\n  (x, y),\n  c,\n  d\n];"
    );
}

#[test]
fn a_sparse_array_keeps_its_lines() {
    let out = pretty("var a = [1, , 3, , 5];");
    assert_eq!(out, "var a = [\n  1,\n  ,\n  3,\n  ,\n  5\n];");
    assert_eq!(pretty(&out), out);
}

#[test]
fn the_compact_style_is_untouched() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "var a = [1, 2, 3];\nvar f = function () {};");
    let out = print(&parsed.program, PrintStyle::Compact);
    assert_eq!(out, "var a=[1,2,3];var f=function(){};");
}

#[test]
fn reflowing_is_idempotent() {
    let source = "var a = [1, 2, 3, 4];\nvar f = function () {};";
    let once = pretty(source);
    assert_eq!(pretty(&once), once);
}

fn unminify_fixtures() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unminify");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("{}: {err}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
            (path.display().to_string(), text)
        })
        .collect()
}

fn assert_pretty_is_the_codegen_reflowed(label: &str, source: &str) {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let pretty = print(&parsed.program, PrintStyle::Pretty);
    assert_eq!(reflow(pretty_codegen(&parsed.program)), pretty, "{label}");
    assert_eq!(
        reflow(pretty.clone()),
        pretty,
        "{label}: the printed text is not at the reflow's fixpoint"
    );
}

#[test]
fn pretty_is_the_pretty_codegen_reflowed_on_every_unminify_fixture() {
    let fixtures = unminify_fixtures();
    assert!(fixtures.len() >= 15, "{}", fixtures.len());
    for (label, source) in fixtures {
        assert_pretty_is_the_codegen_reflowed(&label, &source);
    }
}

#[test]
fn the_nested_literal_that_needs_a_second_pass_reaches_the_fixpoint() {
    assert_pretty_is_the_codegen_reflowed(
        "nested",
        "var nested = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];\nvar f = function() { return [1, 2, 3]; };",
    );
}

#[test]
fn a_program_with_nothing_to_reflow_is_untouched() {
    assert_pretty_is_the_codegen_reflowed("plain", "var a = 1;\nconsole.log(a);");
    let text = "var a = 1;\nconsole.log(a);".to_string();
    assert_eq!(reflow(text.clone()), text);
}

#[test]
fn reflow_on_the_raw_text_collapses_the_array_and_spaces_the_keyword() {
    let text = "var a = [\n  1,\n  2,\n  3\n];\nvar f = function() {};\n".to_string();
    assert_eq!(
        reflow(text),
        "var a = [1, 2, 3];\nvar f = function () {};\n"
    );
}
