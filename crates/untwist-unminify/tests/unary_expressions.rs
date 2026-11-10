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

#[test]
fn typeof_of_an_unresolved_name_stays() {
    assert_transform!(
        "function u() { typeof notDeclaredAnywhere; return 1; }",
        [unary_expressions::boxed],
        @r"
        function u() {
          typeof notDeclaredAnywhere;
          return 1;
        }
        "
    );
}

#[test]
fn typeof_of_a_declared_name_is_dropped() {
    assert_transform!(
        "function u(declared) { typeof declared; return 1; }",
        [unary_expressions::boxed],
        @r"
        function u(declared) {
          declared;
          return 1;
        }
        "
    );
}
