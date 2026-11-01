use untwist_testing::assert_transform;
use untwist_unminify::for_to_while::boxed;

#[test]
fn empty_for_loop_to_while_true() {
    assert_transform!("for (;;) b()", [boxed], @"while (true) b();");
}

#[test]
fn for_loop_with_only_test_to_while() {
    assert_transform!("for (; a(); ) b();", [boxed], @"while (a()) b();");
}

#[test]
fn ignore_for_loop_with_init_or_update() {
    assert_transform!("for (let i = 0;;) {}\nfor (;; i++) {}", [boxed], @r"
    for (let i = 0;;) {}
    for (;; i++) {}
    ");
}
