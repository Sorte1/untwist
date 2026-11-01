use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "var _a;\n((_a = a) !== null && _a !== undefined) ? _a : b;\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::nullish_coalescing::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn identifier_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a) !== null && _a !== undefined ? _a : b;"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @"a ?? b;"
    );
}

#[test]
fn identifier_swc_esbuild() {
    assert_transform!(
        "a != null ? a : b;",
        [untwist_unminify::nullish_coalescing::boxed],
        @"a ?? b;"
    );
}

#[test]
fn identifier_ts() {
    assert_transform!(
        "a !== null && a !== undefined ? a : b;",
        [untwist_unminify::nullish_coalescing::boxed],
        @"a ?? b;"
    );
}

#[test]
fn member_expression_babel() {
    assert_transform!(
        r#"var _a$b;
    (_a$b = a.b) !== null && _a$b !== undefined ? _a$b : c;"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @"a.b ?? c;"
    );
}

#[test]
fn member_expression_loose_babel() {
    assert_transform!(
        r#"var _opts$foo;
    var foo = (_opts$foo = opts.foo) != null ? _opts$foo : "default";"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @r#"var foo = opts.foo ?? "default";"#
    );
}

#[test]
#[ignore = "upstream test.todo: esbuild ((_a_b = a.b) !== null && _a_b !== undefined) || c"]
fn member_expression_esbuild() {
    assert_transform!(
        r#"var _a_b;
    ((_a_b = a.b) !== null && _a_b !== undefined) || c;"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @"a.b ?? c;"
    );
}

#[test]
fn default_param_babel() {
    assert_transform!(
        r#"function foo(foo, qux = (_foo$bar => (_foo$bar = foo.bar) !== null && _foo$bar !== undefined ? _foo$bar : "qux")()) {}
    function bar(bar, qux = bar !== null && bar !== undefined ? bar : "qux") {}"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @r#"
function foo(foo, qux = foo.bar ?? "qux") {}
function bar(bar, qux = bar ?? "qux") {}
"#
    );
}

#[test]
#[ignore = "upstream test.todo: flipped (interp = hello) == null ? \"\" : interp"]
fn flipped() {
    assert_transform!(
        r#"var interp;
    (interp = hello) == null ? "" : interp;"#,
        [untwist_unminify::nullish_coalescing::boxed],
        @r#"hello ?? "";"#
    );
}
