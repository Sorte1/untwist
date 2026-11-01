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
