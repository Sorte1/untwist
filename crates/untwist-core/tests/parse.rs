use oxc_allocator::Allocator;
use untwist_core::parse::{parse_program, strip_bookmarklet};

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
