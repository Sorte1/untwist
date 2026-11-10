use oxc_allocator::Allocator;
use oxc_syntax::symbol::{SymbolFlags, SymbolId};
use untwist_core::parse::parse_program;
use untwist_core::semantic::{
    block_function_symbols, build_scoping, is_constant_binding_in, AnnexBWrites, Semantics,
};

fn outer_binding(sem: &Semantics, name: &str) -> SymbolId {
    sem.scoping
        .symbol_ids()
        .filter(|symbol| sem.scoping.symbol_name(*symbol) == name)
        .find(|symbol| {
            let scope = sem.scoping.symbol_scope_id(*symbol);
            let flags = sem.scoping.scope_flags(scope);
            flags.is_function() || flags.is_top()
        })
        .expect("outer binding")
}

fn written(source: &str, name: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let symbol = outer_binding(&sem, name);
    AnnexBWrites::of(&parsed.program, &sem.scoping).may_write(symbol)
}

fn constant(source: &str, name: &str) -> bool {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let symbol = outer_binding(&sem, name);
    let writes = AnnexBWrites::of(&parsed.program, &sem.scoping);
    is_constant_binding_in(&sem.scoping, &writes, symbol)
}

#[test]
fn sloppy_block_function_writes_the_var_of_its_function() {
    assert!(written(
        "function f() { var x; { function x() {} } x = 5; return typeof x; }",
        "x"
    ));
}

#[test]
fn block_function_in_an_if_or_switch_writes_the_var() {
    assert!(written(
        "function f(c) { var x; if (c) { function x() {} } return x; }",
        "x"
    ));
    assert!(written(
        "function f(c) { var x; switch (c) { case 1: function x() {} } return x; }",
        "x"
    ));
}

#[test]
fn block_function_in_a_loop_body_or_catch_writes_the_var() {
    assert!(written(
        "function f() { var x; for (var i = 0; i < 2; i++) { function x() {} } return x; }",
        "x"
    ));
    assert!(written(
        "function f() { var x; try { throw 1; } catch (e) { function x() {} } return x; }",
        "x"
    ));
}

#[test]
fn block_function_writes_a_function_level_function_of_the_same_name() {
    let source =
        "function f() { function x() { return 1; } { function x() { return 2; } } return x(); }";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let writes = AnnexBWrites::of(&parsed.program, &sem.scoping);
    let outer = outer_binding(&sem, "x");
    assert!(writes.may_write(outer));
    assert_eq!(writes.shared_bindings().count(), 1);
    assert!(writes.shared_bindings().all(|(_, target)| target == outer));
}

#[test]
fn top_level_block_function_writes_the_script_var() {
    assert!(written(
        "var x = 1; { function x() {} } console.log(typeof x);",
        "x"
    ));
}

#[test]
fn object_literal_var_is_not_constant_when_a_block_function_shares_its_name() {
    assert!(!constant(
        "function f() { var o = { a: 1 }; { function o() {} } return typeof o; }",
        "o"
    ));
    assert!(constant(
        "function f() { var o = { a: 1 }; { function p() {} } return typeof o; }",
        "o"
    ));
}

#[test]
fn parameters_are_not_written_by_a_block_function() {
    assert!(!written(
        "function f(x) { { function x() {} } return typeof x; }",
        "x"
    ));
    assert!(!written(
        "function f(x) { var x; { function x() {} } return typeof x; }",
        "x"
    ));
}

#[test]
fn strict_code_has_no_annex_b_writes() {
    assert!(!written(
        "function f() { 'use strict'; var x; { function x() {} } return x; }",
        "x"
    ));
    assert!(!written(
        "'use strict'; function f() { var x; { function x() {} } return x; }",
        "x"
    ));
    assert!(!written(
        "export {}; function f() { var x; { function x() {} } return x; }",
        "x"
    ));
    assert!(!written(
        "class C { m() { var x; { function x() {} } return x; } }",
        "x"
    ));
}

#[test]
fn async_and_generator_block_functions_do_not_write() {
    assert!(!written(
        "function f() { var x; { async function x() {} } return x; }",
        "x"
    ));
    assert!(!written(
        "function f() { var x; { function* x() {} } return x; }",
        "x"
    ));
}

#[test]
fn function_level_declarations_are_not_annex_b() {
    let source = "function f() { var x; function x() {} return x; }";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let writes = AnnexBWrites::of(&parsed.program, &sem.scoping);
    assert!(writes.is_empty());
    let flags = sem.scoping.symbol_flags(outer_binding(&sem, "x"));
    assert!(flags.contains(SymbolFlags::Function));
}

fn block_function_names(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let mut names: Vec<String> = block_function_symbols(&sem.scoping)
        .into_iter()
        .map(|symbol| sem.scoping.symbol_name(symbol).to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn a_hoisted_sloppy_block_function_is_still_a_block_function() {
    assert_eq!(
        block_function_names(
            "function o(c) { if (c) { function p() {} } function q() {} var r = function s() {}; return p; }"
        ),
        ["p"]
    );
}

#[test]
fn block_functions_of_strict_code_and_of_the_top_level_are_found() {
    assert_eq!(
        block_function_names("'use strict'; { function a() {} } function b() {}"),
        ["a"]
    );
    assert_eq!(
        block_function_names("{ function a() {} } function b() {}"),
        ["a"]
    );
}

#[test]
fn a_hoisted_block_function_is_written_where_its_block_runs() {
    let source = "function o(c) { if (c) { function p() {} } return typeof p; }";
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let writes = AnnexBWrites::of(&parsed.program, &sem.scoping);
    let p = outer_binding(&sem, "p");
    assert!(writes.may_write(p));
    assert!(!is_constant_binding_in(&sem.scoping, &writes, p));
}
