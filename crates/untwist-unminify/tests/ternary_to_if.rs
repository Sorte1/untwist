use untwist_testing::assert_transform;

#[test]
fn statement() {
    assert_transform!("a ? b() : c();", [untwist_unminify::ternary_to_if::boxed], @r"
if (a) {
  b();
} else {
  c();
}
");
}

#[test]
fn returned() {
    assert_transform!("return a ? b() : c();", [untwist_unminify::ternary_to_if::boxed], @r"
if (a) {
  return b();
} else {
  return c();
}
");
}

#[test]
fn ignore_expression() {
    assert_transform!("const x = a ? b() : c();", [untwist_unminify::ternary_to_if::boxed], @"const x = a ? b() : c();");
}

#[test]
fn a_statement_chain_lifts_at_most_the_lift_depth_and_keeps_the_rest() {
    let max = untwist_core::depth::LIFT_DEPTH_MAX;
    let source = format!("{}c;", "a ? b : ".repeat(max + 6));
    let alloc = untwist_testing::Allocator::default();
    let output =
        untwist_testing::transform(&alloc, &source, &[untwist_unminify::ternary_to_if::boxed]);
    assert_eq!(output.matches("if (a)").count(), max);
    assert_eq!(output.matches("a ? b :").count(), 6);
}

#[test]
fn a_chain_under_the_lift_depth_lifts_whole() {
    let source = format!("{}c;", "a ? b : ".repeat(10));
    let alloc = untwist_testing::Allocator::default();
    let output =
        untwist_testing::transform(&alloc, &source, &[untwist_unminify::ternary_to_if::boxed]);
    assert_eq!(output.matches("if (a)").count(), 10);
    assert!(!output.contains('?'));
}
