use untwist_testing::assert_transform;
use untwist_unminify::void_to_undefined;

#[test]
fn void_0() {
    assert_transform!("void 0", [void_to_undefined::boxed], @"undefined;");
}

#[test]
fn ignore_when_undefined_is_declared_in_scope() {
    assert_transform!(
        "let undefined = 1; { void 0; }",
        [void_to_undefined::boxed],
        @r"
        let undefined = 1;
        {
          void 0;
        }
        "
    );
}
