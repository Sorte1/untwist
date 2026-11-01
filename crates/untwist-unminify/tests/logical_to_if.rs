use untwist_testing::assert_transform;

#[test]
fn and() {
    assert_transform!(
        "x && y && z();",
        [untwist_unminify::logical_to_if::boxed],
        @r"
    if (x && y) {
      z();
    }
    "
    );
}

#[test]
fn or() {
    assert_transform!(
        "x || y || z();",
        [untwist_unminify::logical_to_if::boxed],
        @r"
    if (!(x || y)) {
      z();
    }
    "
    );
}

#[test]
fn nullish_coalescing_is_untouched() {
    assert_transform!(
        "x ?? y();",
        [untwist_unminify::logical_to_if::boxed],
        @"x ?? y();"
    );
}
