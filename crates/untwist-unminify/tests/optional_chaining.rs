use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "(a === null || a === undefined) ? undefined : a.b;\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::optional_chaining::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn member_expression_ts() {
    assert_transform!(
        "a === null || a === undefined ? undefined : a.b;",
        [untwist_unminify::optional_chaining::boxed],
        @"a?.b;"
    );
}

#[test]
fn computed_member_expression_ts() {
    assert_transform!(
        "a === null || a === undefined ? undefined : a[b];",
        [untwist_unminify::optional_chaining::boxed],
        @"a?.[b];"
    );
}

#[test]
fn member_expression_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a) === null || _a === undefined ? undefined : _a.b;"#,
        [untwist_unminify::optional_chaining::boxed],
        @"a?.b;"
    );
}

#[test]
fn computed_member_expression_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a) === null || _a === undefined ? undefined : _a[0];"#,
        [untwist_unminify::optional_chaining::boxed],
        @"a?.[0];"
    );
}

#[test]
#[ignore = "upstream test.skip: optional call expression (Babel)"]
fn call_expression_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a) === null || _a === undefined ? undefined : _a();"#,
        [untwist_unminify::optional_chaining::boxed],
        @"a?.();"
    );
}

#[test]
#[ignore = "upstream test.skip: optional call expression (TS)"]
fn call_expression_ts() {
    assert_transform!(
        "a === null || a === undefined ? undefined : a();",
        [untwist_unminify::optional_chaining::boxed],
        @"a?.();"
    );
}

#[test]
fn nested_chain_two_levels_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a === null || a === undefined ? undefined : a.b) === null || _a === undefined ? undefined : _a.c;"#,
        [untwist_unminify::optional_chaining::boxed],
        @"a?.b?.c;"
    );
}

#[test]
fn nested_chain_three_levels_babel() {
    assert_transform!(
        r#"var _a;
var _b;
(_b = (_a = a === null || a === undefined ? undefined : a.b) === null || _a === undefined ? undefined : _a.c) === null || _b === undefined ? undefined : _b.d;"#,
        [untwist_unminify::optional_chaining::boxed],
        @"a?.b?.c?.d;"
    );
}

#[test]
fn removes_temp_declarator_nested_in_for_statement_init() {
    assert_transform!("for (var v, i = 0; i < 1; i++) {}\n(v = a) === null || v === undefined ? undefined : v.b;", [untwist_unminify::optional_chaining::boxed], @r"
for (var i = 0; i < 1; i++) {}
a?.b;
");
}

#[test]
fn a_conditional_callee_is_not_rewritten() {
    assert_transform!(
        "(a === null || a === undefined ? undefined : a.b)(x);",
        [untwist_unminify::optional_chaining::boxed],
        @"(a === null || a === undefined ? undefined : a.b)(x);"
    );
}

#[test]
fn a_conditional_callee_with_a_temp_is_not_rewritten() {
    let source = "var t;\n((t = a.q) === null || t === undefined ? undefined : t.b)(x);";
    let alloc = untwist_testing::Allocator::default();
    let output = untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::optional_chaining::boxed],
    );
    assert_eq!(output, source);
}

#[test]
fn a_conditional_delete_operand_is_not_rewritten() {
    assert_transform!(
        "delete (a === null || a === undefined ? undefined : a.b);",
        [untwist_unminify::optional_chaining::boxed],
        @"delete (a === null || a === undefined ? undefined : a.b);"
    );
}

#[test]
fn a_conditional_template_tag_is_not_rewritten() {
    assert_transform!(
        "(a === null || a === undefined ? undefined : a.b)`x`;",
        [untwist_unminify::optional_chaining::boxed],
        @"(a === null || a === undefined ? undefined : a.b)`x`;"
    );
}

#[test]
fn a_conditional_call_argument_is_still_rewritten() {
    assert_transform!(
        "f(a === null || a === undefined ? undefined : a.b);",
        [untwist_unminify::optional_chaining::boxed],
        @"f(a?.b);"
    );
}

fn default_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::optional_chaining::boxed],
        PassOptions::default(),
    )
}

fn unsafe_lane(source: &str) -> String {
    let alloc = Allocator::default();
    untwist_testing::transform(
        &alloc,
        source,
        &[untwist_unminify::optional_chaining::boxed],
    )
}

const REPEATED_CALL: &str = r#"var n = 0;
function f() { n++; return { c: n }; }
var r = f() === null || f() === undefined ? undefined : f().c;
console.log(r, n);"#;

const REPEATED_GETTER: &str = r#"var reads = 0;
var o = { get p() { reads++; return { c: reads }; } };
var r = o.p === null || o.p === undefined ? undefined : o.p.c;
console.log(r, reads);"#;

const REPEATED_GLOBAL_ACCESSOR: &str = r#"var reads = 0;
Object.defineProperty(globalThis, "gx", { get() { return { c: ++reads }; }, configurable: true });
var r = gx === null || gx === undefined ? undefined : gx.c;
console.log(r, reads);"#;

const REPEATED_WITH_ACCESSOR: &str = r#"var reads = 0;
var scope = { get a() { return { c: ++reads }; } };
var a = null;
var r;
with (scope) { r = a === null || a === undefined ? undefined : a.c; }
console.log(r, reads);"#;

#[test]
fn a_repeated_operand_that_can_run_code_keeps_its_conditional_by_default() {
    for source in [
        REPEATED_CALL,
        REPEATED_GETTER,
        REPEATED_GLOBAL_ACCESSOR,
        REPEATED_WITH_ACCESSOR,
    ] {
        let output = default_lane(source);
        assert!(!output.contains("?."), "{output}");
        untwist_testing::node::assert_node_agrees(source, &output);
    }
}

#[test]
fn a_repeated_call_keeps_its_conditional_under_unsafe() {
    let output = unsafe_lane(REPEATED_CALL);
    assert!(!output.contains("?."), "{output}");
    untwist_testing::node::assert_node_agrees(REPEATED_CALL, &output);
}

#[test]
fn a_call_free_member_chain_still_converts_under_unsafe() {
    assert!(unsafe_lane(REPEATED_GETTER).contains("o.p?.c"));
    assert!(unsafe_lane(REPEATED_GLOBAL_ACCESSOR).contains("gx?.c"));
}

#[test]
fn transpiler_output_still_converts_by_default() {
    let rows = [
        (
            "function h(a) { return a === null || a === undefined ? undefined : a.b; }",
            "return a?.b;",
        ),
        (
            "function h(a, b) { return a === null || a === undefined ? undefined : a[b]; }",
            "return a?.[b];",
        ),
        (
            "function h() { return this === null || this === undefined ? undefined : this.b; }",
            "return this?.b;",
        ),
        (
            "function h(a) { var _a; return (_a = a.x) === null || _a === undefined ? undefined : _a.b; }",
            "return a.x?.b;",
        ),
        (
            "function h(f) { var _f; return (_f = f()) === null || _f === undefined ? undefined : _f.b; }",
            "return f()?.b;",
        ),
        (
            "function h(a) { var _a; return (_a = a === null || a === undefined ? undefined : a.b) === null || _a === undefined ? undefined : _a.c; }",
            "return a?.b?.c;",
        ),
    ];
    for (source, expected) in rows {
        let output = default_lane(source);
        assert!(output.contains(expected), "{output}");
    }
}

#[test]
fn member_expression_esbuild_loose_test() {
    assert_transform!(
        r#"var _a, _b;
    console.log((_a = data.user) == null ? undefined : _a.name, (_b = data.list) == null ? undefined : _b[0]);"#,
        [untwist_unminify::optional_chaining::boxed],
        @"console.log(data.user?.name, data.list?.[0]);"
    );
}

#[test]
fn nested_chain_esbuild_loose_test() {
    assert_transform!(
        r#"var _a, _b;
    (_b = (_a = data.missing) == null ? undefined : _a.deep) == null ? undefined : _b.deeper;"#,
        [untwist_unminify::optional_chaining::boxed],
        @"data.missing?.deep?.deeper;"
    );
}

#[test]
fn a_loose_test_of_a_binding_converts() {
    assert_transform!(
        "function f(arr) { return arr == null ? undefined : arr[0]; }",
        [untwist_unminify::optional_chaining::boxed],
        @r"
    function f(arr) {
      return arr?.[0];
    }
    "
    );
}

#[test]
fn a_loose_temp_read_twice_keeps_its_conditional() {
    assert_transform!(
        r#"var _a;
    (_a = a.b) == null ? undefined : _a.c + _a.d;"#,
        [untwist_unminify::optional_chaining::boxed],
        @r"
    var _a;
    (_a = a.b) == null ? undefined : _a.c + _a.d;
    "
    );
}

#[test]
fn a_loose_test_against_undefined_keeps_its_conditional() {
    assert_transform!(
        r#"var _a;
    (_a = a.b) == undefined ? undefined : _a.c;"#,
        [untwist_unminify::optional_chaining::boxed],
        @r"
    var _a;
    (_a = a.b) == undefined ? undefined : _a.c;
    "
    );
}
