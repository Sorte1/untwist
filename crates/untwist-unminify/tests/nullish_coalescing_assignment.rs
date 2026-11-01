use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "a ?? (a = b);\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::nullish_coalescing_assignment::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn identifier_assignment_babel() {
    assert_transform!(
        "a ?? (a = b);",
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a ??= b;"
    );
}

#[test]
fn member_expression_assignment_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a).b ?? (_a.b = c);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a.b ??= c;"
    );
}

#[test]
fn computed_member_expression_assignment_babel() {
    assert_transform!(
        r#"var _a;
    (_a = a)[b] ?? (_a[b] = c);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a[b] ??= c;"
    );
}

#[test]
fn member_expression_assignment_ts() {
    assert_transform!(
        "a.b ?? (a.b = c);",
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"a.b ??= c;"
    );
}

#[test]
#[ignore = "upstream test.skip: nested object temp"]
fn nested_object_temp_ts() {
    assert_transform!(
        r#"var _c;
    (_c = c.foo).baz ?? (_c.baz = result.foo.baz);"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"c.foo.baz ??= result.foo.baz;"
    );
}

#[test]
#[ignore = "upstream test.skip: side-effecting computed key"]
fn side_effecting_computed_key_ts() {
    assert_transform!(
        r#"var _a;
    obj[_a = incr()] ?? (obj[_a] = incr());"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @"obj[incr()] ??= incr();"
    );
}

#[test]
#[ignore = "upstream test.skip: two temps with side-effecting computed key"]
fn two_temps_ts() {
    assert_transform!(
        r#"var _b;
var _c;
(_b = oobj.obj)[_c = incr()] ?? (_b[_c] = incr());"#,
        [untwist_unminify::nullish_coalescing_assignment::boxed],
        @r#"oobj["obj"][incr()] ??= incr();"#
    );
}
