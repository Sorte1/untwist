use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn identifier_or_assignment() {
    assert_transform!("x || (x = y)", [untwist_unminify::logical_assignments::boxed], @"x ||= y;");
}

#[test]
fn identifier_and_assignment() {
    assert_transform!("x && (x = y)", [untwist_unminify::logical_assignments::boxed], @"x &&= y;");
}

#[test]
fn member_expression_or_assignment_babel() {
    assert_transform!(
        r#"var _x$y;
    (_x$y = x.y()).z || (_x$y.z = b);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x.y().z ||= b;"
    );
}

#[test]
fn computed_member_expression_or_assignment_babel() {
    assert_transform!(
        r#"var _x$y;
    (_x$y = x.y())[z] || (_x$y[z] = b);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x.y()[z] ||= b;"
    );
}

#[test]
fn member_expression_or_assignment_ts() {
    assert_transform!("x.y || (x.y = z)", [untwist_unminify::logical_assignments::boxed], @"x.y ||= z;");
}

#[test]
fn computed_member_expression_or_assignment_multi_temp() {
    assert_transform!(
        r#"var _x, _y;
    (_x = x)[_y = y] || (_x[_y] = z);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn computed_member_expression_or_assignment_ts() {
    assert_transform!(
        "x[y] || (x[y] = z);",
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn computed_member_expression_or_assignment_single_temp() {
    assert_transform!(
        r#"var _y;
    x[_y = y] || (x[_y] = z);"#,
        [untwist_unminify::logical_assignments::boxed],
        @"x[y] ||= z;"
    );
}

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "var _a;\n((_a = o).x) || (_a.x = 1);\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::logical_assignments::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}
