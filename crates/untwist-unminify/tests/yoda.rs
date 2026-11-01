use untwist_testing::assert_transform;
use untwist_unminify::yoda;

#[test]
fn strict_equality() {
    assert_transform!(r#""red" === color"#, [yoda::boxed], @r#"color === "red";"#);
}

#[test]
fn loose_equality() {
    assert_transform!("null == x", [yoda::boxed], @"x == null;");
}

#[test]
fn strict_inequality() {
    assert_transform!(r#""red" !== color"#, [yoda::boxed], @r#"color !== "red";"#);
}

#[test]
fn loose_inequality() {
    assert_transform!("null != x", [yoda::boxed], @"x != null;");
}

#[test]
fn less_than() {
    assert_transform!("0 < x", [yoda::boxed], @"x > 0;");
}

#[test]
fn less_or_equal() {
    assert_transform!("0 <= x", [yoda::boxed], @"x >= 0;");
}

#[test]
fn greater_than() {
    assert_transform!("0 > x", [yoda::boxed], @"x < 0;");
}

#[test]
fn greater_or_equal() {
    assert_transform!("0 >= x", [yoda::boxed], @"x <= 0;");
}

#[test]
fn multiply() {
    assert_transform!("0 * x", [yoda::boxed], @"x * 0;");
}

#[test]
fn xor() {
    assert_transform!("0 ^ x", [yoda::boxed], @"x ^ 0;");
}

#[test]
fn and() {
    assert_transform!("0 & x", [yoda::boxed], @"x & 0;");
}

#[test]
fn or() {
    assert_transform!("0 | x", [yoda::boxed], @"x | 0;");
}

#[test]
fn string() {
    assert_transform!(r#""str" == x"#, [yoda::boxed], @r#"x == "str";"#);
}

#[test]
fn number() {
    assert_transform!("1 == x", [yoda::boxed], @"x == 1;");
}

#[test]
fn negative_number() {
    assert_transform!("-1 == x", [yoda::boxed], @"x == -1;");
}

#[test]
fn boolean() {
    assert_transform!("true == x", [yoda::boxed], @"x == true;");
}

#[test]
fn null() {
    assert_transform!("null == x", [yoda::boxed], @"x == null;");
}

#[test]
fn undefined() {
    assert_transform!("undefined == x", [yoda::boxed], @"x == undefined;");
}

#[test]
fn nan() {
    assert_transform!("NaN == x", [yoda::boxed], @"x == NaN;");
}

#[test]
fn infinity() {
    assert_transform!("Infinity == x", [yoda::boxed], @"x == Infinity;");
}

#[test]
fn negative_infinity() {
    assert_transform!("-Infinity == x", [yoda::boxed], @"x == -Infinity;");
}

#[test]
fn ignore_other_operators() {
    assert_transform!("2 + x", [yoda::boxed], @"2 + x;");
}

#[test]
fn ignore_when_right_side_is_a_literal() {
    assert_transform!("1 === 2", [yoda::boxed], @"1 === 2;");
}

#[test]
fn ignore_when_both_sides_are_pure_values() {
    assert_transform!("NaN == Infinity", [yoda::boxed], @"NaN == Infinity;");
}
