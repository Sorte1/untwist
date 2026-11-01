use untwist_testing::assert_transform;
use untwist_unminify::invert_boolean_logic::boxed;

#[test]
fn loose_equal() {
    assert_transform!("!(a == b);", [boxed], @"a != b;");
}

#[test]
fn strict_equal() {
    assert_transform!("!(a === b);", [boxed], @"a !== b;");
}

#[test]
fn not_equal() {
    assert_transform!("!(a != b);", [boxed], @"a == b;");
}

#[test]
fn not_strict_equal() {
    assert_transform!("!(a !== b);", [boxed], @"a === b;");
}

#[test]
fn logical_or() {
    assert_transform!("!(a || b || c);", [boxed], @"!a && !b && !c;");
}

#[test]
fn logical_and() {
    assert_transform!("!(a && b && c);", [boxed], @"!a || !b || !c;");
}

#[test]
fn mixed_logical() {
    assert_transform!("!((a ?? b) || c);", [boxed], @"!(a ?? b) && !c;");
}
