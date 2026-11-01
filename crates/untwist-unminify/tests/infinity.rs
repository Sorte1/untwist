use untwist_testing::assert_transform;
use untwist_unminify::infinity::boxed;

#[test]
fn infinity() {
    assert_transform!("1/0", [boxed], @"Infinity;");
}

#[test]
fn negative_infinity() {
    assert_transform!("-1/0", [boxed], @"-Infinity;");
}

#[test]
fn ignore_when_infinity_is_declared_in_scope() {
    assert_transform!("let Infinity = 1; 1/0", [boxed], @r"
    let Infinity = 1;
    1 / 0;
    ");
}
