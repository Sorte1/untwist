use oxc_allocator::Allocator;
use untwist_core::parse::{parse_program, strip_bookmarklet};
use untwist_core::semantic::build_scoping;

#[test]
fn parses_module_syntax() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "import a from 'a'; export default a;");
    assert!(p.diagnostics.is_empty());
    assert!(p.source_type.is_module());
    assert_eq!(p.program.body.len(), 2);
}

#[test]
fn falls_back_to_commonjs_for_module_only_reserved_word() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var await = 1; return 1;");
    assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    assert!(p.source_type.is_commonjs());
}

#[test]
fn parses_jsx() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "const x = <div className=\"a\">{y}</div>;");
    assert!(p.diagnostics.is_empty());
}

#[test]
fn recovers_from_syntax_errors_with_diagnostics() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "if (true) const a = 1;\nlet b = 2;");
    assert!(!p.diagnostics.is_empty());
    assert!(!p.program.body.is_empty());
}

#[test]
fn strips_bookmarklet_prefix_and_decodes() {
    assert_eq!(
        strip_bookmarklet("javascript:alert(%22hi%22)"),
        "alert(\"hi\")"
    );
    assert_eq!(strip_bookmarklet("javascript:v%F"), "v%F");
    assert_eq!(strip_bookmarklet("plain()"), "plain()");
}

#[test]
fn a_file_without_module_syntax_is_a_classic_script() {
    let alloc = Allocator::default();
    let p = parse_program(&alloc, "var a = 1; function f() { return this; }");
    assert!(p.diagnostics.is_empty());
    assert!(!p.source_type.is_module());
    assert!(!p.program.source_type.is_module());
}

#[test]
fn import_meta_and_top_level_await_make_a_module() {
    for source in [
        "console.log(import.meta.url);",
        "function f() { return import.meta; }",
        "await Promise.resolve(1);",
        "for await (const x of xs) {}",
        "export {};",
    ] {
        let alloc = Allocator::default();
        let p = parse_program(&alloc, source);
        assert!(p.diagnostics.is_empty(), "{source}: {:?}", p.diagnostics);
        assert!(p.source_type.is_module(), "{source}");
        assert!(p.program.source_type.is_module(), "{source}");
    }
}

#[test]
fn await_inside_an_async_function_does_not_make_a_module() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "async function f() { await g(); for await (const x of y) {} }",
    );
    assert!(!p.source_type.is_module());
}

#[test]
fn a_sloppy_script_hoists_a_block_function_to_its_function() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "function outer() { { function atob(s) { return s; } } return atob(\"x\"); }",
    );
    let sem = build_scoping(&p.program);
    assert!(!sem
        .scoping
        .root_unresolved_references()
        .contains_key("atob"));
}

#[test]
fn a_strict_script_keeps_a_block_function_in_its_block() {
    let alloc = Allocator::default();
    let p = parse_program(
        &alloc,
        "\"use strict\"; function outer() { { function atob(s) { return s; } } return atob(\"x\"); }",
    );
    let sem = build_scoping(&p.program);
    assert!(sem
        .scoping
        .root_unresolved_references()
        .contains_key("atob"));
}
