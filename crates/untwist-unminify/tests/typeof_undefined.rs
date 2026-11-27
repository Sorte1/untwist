use untwist_testing::assert_transform;
use untwist_unminify::typeof_undefined;

#[test]
fn typeof_greater_than() {
    assert_transform!(
        r#"typeof a > "u""#,
        [typeof_undefined::boxed],
        @r#"typeof a === "undefined";"#
    );
}

#[test]
fn typeof_less_than() {
    assert_transform!(
        r#"typeof a < "u""#,
        [typeof_undefined::boxed],
        @r#"typeof a !== "undefined";"#
    );
}

#[test]
fn typeof_compared_loosely_with_a_string() {
    assert_transform!(
        r#"typeof a == "number"; "object" != typeof b; typeof a == typeof b; typeof a == c; a == "x";"#,
        [typeof_undefined::boxed],
        @r#"
    typeof a === "number";
    "object" !== typeof b;
    typeof a === typeof b;
    typeof a == c;
    a == "x";
    "#
    );
}
