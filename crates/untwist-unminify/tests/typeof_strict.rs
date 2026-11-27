use untwist_testing::assert_transform;
use untwist_unminify::typeof_strict;

#[test]
fn typeof_compared_loosely_with_a_string() {
    assert_transform!(
        r#"typeof a == "number"; "object" != typeof b; typeof a == typeof b; typeof a == c; a == "x";"#,
        [typeof_strict::boxed],
        @r#"
    typeof a === "number";
    "object" !== typeof b;
    typeof a === typeof b;
    typeof a == c;
    a == "x";
    "#
    );
}
