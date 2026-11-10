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

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::nullish_coalescing::boxed],
        PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::nullish_coalescing::boxed],
    )
}

const CALL_LOOSE: &str = r#"var n = 0;
function f() { n++; return n > 1 ? null : 1; }
var r = f() != null ? f() : "g";
console.log(r, n);"#;

const CALL_STRICT: &str = r#"var n = 0;
function f() { n++; return n; }
var r = f() !== null && f() !== undefined ? f() : "g";
console.log(r, n);"#;

const GETTER_LOOSE: &str = r#"var reads = 0;
var o = { get p() { reads++; return reads; } };
var r = o.p != null ? o.p : "g";
console.log(r, reads);"#;

const GETTER_STRICT: &str = r#"var reads = 0;
var o = { get p() { reads++; return reads; } };
var r = o.p !== null && o.p !== undefined ? o.p : "g";
console.log(r, reads);"#;

const GLOBAL_ACCESSOR: &str = r#"var reads = 0;
Object.defineProperty(globalThis, "gx", { get() { return ++reads; }, configurable: true });
var r = gx != null ? gx : "g";
console.log(r, reads);"#;

const WITH_ACCESSOR: &str = r#"var reads = 0;
var scope = { get a() { return ++reads; } };
var a = 0;
var r;
with (scope) { r = a != null ? a : "g"; }
console.log(r, reads);"#;

const DIRECT_EVAL: &str = r#"function h(a, code) { eval(code); return a != null ? a : "g"; }
console.log(h(1, ""), h(null, ""));"#;

#[test]
fn a_repeated_operand_that_can_run_code_keeps_its_conditional_by_default() {
    for source in [
        CALL_LOOSE,
        CALL_STRICT,
        GETTER_LOOSE,
        GETTER_STRICT,
        GLOBAL_ACCESSOR,
        WITH_ACCESSOR,
        DIRECT_EVAL,
    ] {
        let output = default_lane(source);
        assert!(!output.contains("??"), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_repeated_call_keeps_its_conditional_under_unsafe() {
    for source in [CALL_LOOSE, CALL_STRICT] {
        let output = unsafe_lane(source);
        assert!(!output.contains("??"), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_call_free_member_chain_still_converts_under_unsafe() {
    assert!(unsafe_lane(GETTER_LOOSE).contains(r#"o.p ?? "g""#));
    assert!(unsafe_lane(GETTER_STRICT).contains(r#"o.p ?? "g""#));
    assert!(unsafe_lane(GLOBAL_ACCESSOR).contains(r#"gx ?? "g""#));
    assert!(
        unsafe_lane("var r = a.b[c + 1].d != null ? a.b[c + 1].d : 0;")
            .contains("a.b[c + 1].d ?? 0")
    );
}

#[test]
fn transpiler_output_still_converts_by_default() {
    let rows = [
        (
            "function h(a, b) { return a != null ? a : b; }",
            "return a ?? b;",
        ),
        (
            "function h(a, b) { return a !== null && a !== undefined ? a : b; }",
            "return a ?? b;",
        ),
        (
            "function h(b) { return this != null ? this : b; }",
            "return this ?? b;",
        ),
        (
            "function h(a, b) { var _a$x; return (_a$x = a.x) !== null && _a$x !== undefined ? _a$x : b; }",
            "return a.x ?? b;",
        ),
        (
            "function h(a, b) { var _a$x; return (_a$x = a.x) != null ? _a$x : b; }",
            "return a.x ?? b;",
        ),
        (
            "function h(f, b) { var _f; return (_f = f()) !== null && _f !== undefined ? _f : b; }",
            "return f() ?? b;",
        ),
        (
            r#"function h(foo, qux = (_foo$bar => (_foo$bar = foo.bar) !== null && _foo$bar !== undefined ? _foo$bar : "qux")()) {}"#,
            r#"qux = foo.bar ?? "qux""#,
        ),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
    }
}
