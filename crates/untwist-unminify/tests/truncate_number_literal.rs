use untwist_testing::assert_transform;

#[test]
fn truncate_float() {
    assert_transform!(
        "!(a | 12.34)",
        [untwist_unminify::truncate_number_literal::boxed],
        @"!(a | 12);"
    );
}

#[test]
fn truncate_overflow() {
    assert_transform!(
        "!(a ^ 0xfffffffff)",
        [untwist_unminify::truncate_number_literal::boxed],
        @"!(a ^ -1);"
    );
}

#[test]
fn truncate_shift_overflow() {
    assert_transform!(
        "!(a << 64)",
        [untwist_unminify::truncate_number_literal::boxed],
        @"!(a << 0);"
    );
    assert_transform!(
        "!(1000 << a)",
        [untwist_unminify::truncate_number_literal::boxed],
        @"!(1e3 << a);"
    );
}
