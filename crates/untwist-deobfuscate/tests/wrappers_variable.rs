use untwist_core::pass::PassOptions;
use untwist_testing::assert_transform;

const HEAD: &str = "function __STRING_ARRAY__() {\n  var h = ['log'];\n  __STRING_ARRAY__ = function () { return h; };\n  return __STRING_ARRAY__();\n}\nfunction __DECODE_0__(c, d) {\n  var e = __STRING_ARRAY__();\n  return e[c];\n}\n";

const DECODE_1: &str =
    "function __DECODE_1__(c, d) {\n  var e = __STRING_ARRAY__();\n  return e[c + 1];\n}\n";

fn wrapped(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    let code = untwist_testing::transform(&alloc, source, &[untwist_deobfuscate::wrappers::boxed]);
    let check = untwist_testing::Allocator::default();
    let reparsed = untwist_core::parse::parse_program(&check, &code);
    assert!(
        reparsed.diagnostics.is_empty(),
        "output does not re-parse: {:?}\n{code}",
        reparsed.diagnostics
    );
    code
}

fn printed(source: &str) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform(&alloc, source, &[])
}

fn notes(source: &str) -> Vec<(&'static str, String)> {
    let alloc = untwist_testing::Allocator::default();
    let (_, report) = untwist_testing::transform_report(
        &alloc,
        source,
        &[untwist_deobfuscate::wrappers::boxed],
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    );
    report
        .notes
        .into_iter()
        .map(|note| (note.pass, note.message))
        .collect()
}

fn rejected(source: &str) {
    assert_eq!(wrapped(source), printed(source), "the shape was rewritten");
    let notes = notes(source);
    assert_eq!(notes.len(), 1, "{notes:?}");
    assert_eq!(notes[0].0, "inline-decoder-wrappers", "{notes:?}");
}

#[test]
fn declarator_aliases_and_alias_of_alias_are_flattened() {
    let out = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nconst alias3 = alias;\nalias(2);\nalias3(3);"
    ));
    assert!(out.contains("__DECODE_0__(2);"), "{out}");
    assert!(out.contains("__DECODE_0__(3);"), "{out}");
    assert!(!out.contains("alias"), "{out}");
}

#[test]
fn an_assignment_in_expression_position_becomes_the_decoder() {
    let out = wrapped(&format!("{HEAD}let alias2;\n(alias2 = __DECODE_0__)(4);"));
    assert!(out.contains("__DECODE_0__(4);"), "{out}");
    assert!(!out.contains("let __DECODE_0__;"), "{out}");
    assert!(!out.contains("__DECODE_0__ = __DECODE_0__;"), "{out}");
}

#[test]
fn an_assignment_statement_alias_is_removed() {
    let out = wrapped(&format!(
        "{HEAD}let alias4;\nalias4 = __DECODE_0__;\nalias4(5);"
    ));
    assert!(out.contains("__DECODE_0__(5);"), "{out}");
    assert!(!out.contains("alias4 = "), "{out}");
    assert!(!out.contains("let __DECODE_0__;"), "{out}");
    assert!(!out.contains("__DECODE_0__ = __DECODE_0__;"), "{out}");
}

#[test]
fn a_self_assignment_does_not_loop() {
    let out = wrapped(&format!(
        "{HEAD}let alias;\nalias = alias;\nalias = __DECODE_0__;\nalias(6);"
    ));
    assert!(out.contains("__DECODE_0__(6);"), "{out}");
    assert!(!out.contains("let __DECODE_0__;"), "{out}");
    assert!(!out.contains("__DECODE_0__ = __DECODE_0__;"), "{out}");
}

#[test]
fn an_unrelated_alias_is_untouched() {
    assert_transform!(
        "const x = y;\nx(1);",
        [untwist_deobfuscate::wrappers::boxed],
        @"
    const x = y;
    x(1);
    "
    );
}

#[test]
fn an_alias_inside_an_arrow_body_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}const z = () => {{ const alias = __DECODE_0__; return alias(1); }};"
    ));
    assert!(code.contains("__DECODE_0__(1)"), "{code}");
    assert!(!code.contains("alias"), "{code}");
}

#[test]
fn an_optional_call_through_an_alias_is_inlined() {
    let code = wrapped(&format!("{HEAD}const alias = __DECODE_0__;\nalias?.(1);"));
    assert!(code.contains("__DECODE_0__?.(1)"), "{code}");
}

#[test]
fn an_alias_inside_a_template_expression_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nf(`x${{alias(1)}}y`);"
    ));
    assert!(code.contains("__DECODE_0__(1)"), "{code}");
}

#[test]
fn an_alias_inside_an_object_property_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nf({{ k: alias(1) }});"
    ));
    assert!(code.contains("__DECODE_0__(1)"), "{code}");
}

#[test]
fn an_alias_inside_a_for_of_body_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nfor (const q of list) {{ f(alias(1)); }}"
    ));
    assert!(code.contains("__DECODE_0__(1)"), "{code}");
}

#[test]
fn an_alias_inside_a_for_of_head_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nfor (const q of alias(1)) {{ f(q); }}"
    ));
    assert!(code.contains("of __DECODE_0__(1)"), "{code}");
}

#[test]
fn an_alias_inside_a_yield_operand_is_inlined() {
    let code = wrapped(&format!(
        "{HEAD}function* g() {{ const alias = __DECODE_0__; yield alias(1); }}"
    ));
    assert!(code.contains("yield __DECODE_0__(1)"), "{code}");
}

#[test]
fn sibling_scopes_that_rebind_the_same_alias_name_do_not_cross() {
    let code = wrapped(&format!(
        "{HEAD}function p() {{ const i = __DECODE_0__; return i(1); }}\nfunction q() {{ const i = other; return i(2); }}"
    ));
    assert!(code.contains("__DECODE_0__(1)"), "{code}");
    assert!(code.contains("i(2)"), "{code}");
}

#[test]
fn an_alias_declared_in_a_for_head_is_left_untouched() {
    rejected(&format!(
        "{HEAD}for (var a = __DECODE_0__; a; a = null) {{ f(a(1)); }}"
    ));
}

#[test]
fn a_multi_declarator_alias_is_left_untouched() {
    rejected(&format!("{HEAD}var a = __DECODE_0__, b = 1;\nf(a(1), b);"));
}

#[test]
fn an_assignment_alias_without_a_declaration_is_left_untouched() {
    rejected(&format!("{HEAD}function f(a) {{ (a = __DECODE_0__)(1); }}"));
}

#[test]
fn an_alias_with_a_second_write_is_left_untouched() {
    rejected(&format!("{HEAD}var a = __DECODE_0__;\na(1);\na = null;"));
}

#[test]
fn an_initialised_declaration_blocks_the_assignment_form() {
    rejected(&format!("{HEAD}var a = f();\na = __DECODE_0__;\na(1);"));
}

#[test]
fn a_declaration_outside_a_statement_list_is_left_untouched() {
    rejected(&format!("{HEAD}if (c) var a = __DECODE_0__;\na(1);"));
}

#[test]
fn an_alias_that_redeclares_a_parameter_is_left_untouched() {
    rejected(&format!(
        "{HEAD}function f(a) {{ var a = __DECODE_0__; a(1); }}"
    ));
}

#[test]
fn duplicate_definitions_of_an_alias_are_left_untouched() {
    rejected(&format!(
        "{HEAD}var a = __DECODE_0__;\na(1);\nvar a = __DECODE_0__;"
    ));
}

#[test]
fn duplicate_assignments_of_an_alias_are_left_untouched() {
    rejected(&format!(
        "{HEAD}let a;\na = __DECODE_0__;\na = __DECODE_0__;\na(1);"
    ));
}

#[test]
fn an_alias_chain_resolves_on_a_later_sweep() {
    let out = wrapped(&format!(
        "{HEAD}var a = __DECODE_0__;\nfunction g() {{ var c = b; return c(1); }}\nvar b = a;\ng();"
    ));
    assert!(out.contains("return __DECODE_0__(1);"), "{out}");
    assert!(out.contains("g();"), "{out}");
    assert!(!out.contains("var a"), "{out}");
    assert!(!out.contains("var b"), "{out}");
    assert!(!out.contains("var c"), "{out}");
}

#[test]
fn sibling_functions_alias_different_decoders() {
    let out = wrapped(&format!(
        "{HEAD}{DECODE_1}function p() {{ var i = __DECODE_0__; return i(1); }}\nfunction q() {{ var i = __DECODE_1__; return i(2); }}"
    ));
    assert!(out.contains("return __DECODE_0__(1);"), "{out}");
    assert!(out.contains("return __DECODE_1__(2);"), "{out}");
}

#[test]
fn an_inner_alias_shadows_an_outer_alias() {
    let out = wrapped(&format!(
        "{HEAD}{DECODE_1}var a = __DECODE_0__;\nfunction f() {{ var a = __DECODE_1__; a(1); }}\na(2);"
    ));
    assert!(out.contains("__DECODE_1__(1);"), "{out}");
    assert!(out.contains("__DECODE_0__(2);"), "{out}");
}

#[test]
fn a_second_run_over_the_output_changes_nothing() {
    let first = wrapped(&format!(
        "{HEAD}const alias = __DECODE_0__;\nconst alias3 = alias;\nlet alias2;\nalias(2);\nalias3(3);\n(alias2 = __DECODE_0__)(4);"
    ));
    let second = wrapped(&first);
    assert_eq!(first, second, "second run changed the output");
}

#[test]
fn an_alias_read_inside_a_hoisted_function_that_can_run_first_is_kept() {
    let source = format!(
        "{HEAD}function g() {{ return alias(0x0, 0x1); }}\nconsole[g()]('t');\nvar alias = __DECODE_0__;"
    );
    assert_eq!(wrapped(&source), printed(&source));
    assert!(
        notes(&source)
            .iter()
            .any(|note| note.1.contains("may be read before its declaration runs")),
        "{:?}",
        notes(&source)
    );
}

#[test]
fn an_alias_declared_before_the_function_that_reads_it_is_still_resolved() {
    let source = format!(
        "{HEAD}var alias = __DECODE_0__;\nfunction g() {{ return alias(0x0, 0x1); }}\nconsole[g()]('t');"
    );
    let out = wrapped(&source);
    assert!(!out.contains("alias"), "{out}");
    assert!(out.contains("__DECODE_0__(0, 1)"), "{out}");
}
