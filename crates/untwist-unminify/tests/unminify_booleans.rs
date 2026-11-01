use untwist_testing::assert_transform;
use untwist_unminify::unminify_booleans;

#[test]
fn r#true() {
    assert_transform!("!0", [unminify_booleans::boxed], @"true;");
    assert_transform!("!!1", [unminify_booleans::boxed], @"true;");
    assert_transform!("!![]", [unminify_booleans::boxed], @"true;");
}

#[test]
fn r#false() {
    assert_transform!("!1", [unminify_booleans::boxed], @"false;");
    assert_transform!("![]", [unminify_booleans::boxed], @"false;");
}
