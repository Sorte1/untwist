mod common;

use common::{assert_scope_safe, rename_full};
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 4] = [
    RenameMode::Mangle,
    RenameMode::Simple,
    RenameMode::Unique,
    RenameMode::Smart,
];

const IN_A_BLOCK: &str = "function outer() { { function g() { return 7; } } return g(); }\n";

fn renamed(source: &str, mode: RenameMode) -> String {
    let config = RenameConfig {
        mode,
        rename_all: true,
        extended: true,
    };
    let output = rename_full(source, config);
    assert_scope_safe(source, &output);
    output
}

#[test]
fn a_sloppy_block_level_function_declaration_is_pinned_in_every_mode() {
    for mode in MODES {
        let output = renamed(IN_A_BLOCK, mode);
        assert!(
            output.contains("function g()"),
            "{mode:?}: Annex B.3.3 hoists the block-level `g` as a var in the enclosing \
             function, so renaming its declaration strands the call: {output}"
        );
        assert!(output.contains("return g()"), "{mode:?}: {output}");
    }
}

#[test]
fn the_function_around_a_pinned_block_declaration_is_still_renamed() {
    for mode in MODES {
        let output = renamed(IN_A_BLOCK, mode);
        assert!(
            !output.contains("function outer()"),
            "{mode:?}: only the block-level declaration is pinned: {output}"
        );
    }
}

#[test]
fn a_sloppy_case_body_function_declaration_is_pinned() {
    let source =
        "function f(x) { switch (x) { case 1: function g() { return 7; } } return g(); }\n";
    for mode in MODES {
        let output = renamed(source, mode);
        assert!(
            output.contains("function g()") && output.contains("return g()"),
            "{mode:?}: a case body is a block-like scope for Annex B: {output}"
        );
    }
}

#[test]
fn an_if_clause_function_declaration_keeps_its_call_site() {
    let source = "function f() { if (true) function g() { return 7; } return g(); }\n";
    for mode in MODES {
        let output = renamed(source, mode);
        assert_eq!(
            output.contains("function g()"),
            output.contains("return g()"),
            "{mode:?}: Annex B.3.4 hoists an if-clause declaration, so the declaration and \
             its call must keep the same name: {output}"
        );
    }
}

#[test]
fn a_strict_block_level_function_declaration_is_renamed() {
    let source = format!("\"use strict\";\n{IN_A_BLOCK}");
    for mode in MODES {
        let output = renamed(&source, mode);
        assert!(
            !output.contains("function g()"),
            "{mode:?}: Annex B does not apply to strict code, so nothing needs pinning: {output}"
        );
    }
}

#[test]
fn a_module_block_level_function_declaration_is_renamed() {
    let source = format!("export {{}};\n{IN_A_BLOCK}");
    for mode in MODES {
        let output = renamed(&source, mode);
        assert!(
            !output.contains("function g()"),
            "{mode:?}: module code is strict, so Annex B does not apply: {output}"
        );
    }
}

#[test]
fn the_hoisted_name_is_reserved_against_every_other_binding() {
    let source = "function outer(a1, a2, a3, a4, a5, a6, a7) { \
                  { function g() { return 7; } } \
                  return a1 + a2 + a3 + a4 + a5 + a6 + a7; }\n";
    let output = renamed(source, RenameMode::Simple);
    assert!(
        output.contains("function g()"),
        "the block declaration stays pinned: {output}"
    );
    assert!(
        output.contains(",h)"),
        "the sequence must skip `g`: Annex B assigns the block function to any \
         enclosing-scope binding of that name on block entry: {output}"
    );
}
