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
