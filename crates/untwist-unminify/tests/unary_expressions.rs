use untwist_testing::assert_transform;
use untwist_unminify::unary_expressions;

#[test]
fn void() {
    assert_transform!("void foo();", [unary_expressions::boxed], @"foo();");
}

#[test]
fn r#typeof() {
    assert_transform!("typeof foo();", [unary_expressions::boxed], @"foo();");
}

#[test]
fn logical_not() {
    assert_transform!("!foo();", [unary_expressions::boxed], @"foo();");
}

#[test]
fn return_void() {
    assert_transform!(
        "return void foo();",
        [unary_expressions::boxed],
        @r"
        foo();
        return;
        "
    );
}
