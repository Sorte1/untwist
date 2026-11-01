use untwist_testing::assert_transform;

#[test]
fn extract_a_single_unused_var() {
    assert_transform!("for (var i = 0;;) {}", [untwist_unminify::split_for_loop_vars::boxed], @r"
var i = 0;
for (;;) {}
");
}

#[test]
fn extract_the_first_unused_vars_and_keep_the_rest() {
    assert_transform!("for (var i = 0, j = 1, k = 2; k < 5; k++) {}", [untwist_unminify::split_for_loop_vars::boxed], @r"
var i = 0;
var j = 1;
for (var k = 2; k < 5; k++) {}
");
}

#[test]
fn ignore_let_in_for_loop_initializer() {
    assert_transform!("let i;\nfor (let i = 0, j = 1;;) {}", [untwist_unminify::split_for_loop_vars::boxed], @r"
let i;
for (let i = 0, j = 1;;) {}
");
    assert_transform!("for (let i = 0, j = 1;;) { setTimeout(() => console.log(i)); if (++i > 3) break; }", [untwist_unminify::split_for_loop_vars::boxed], @r"
for (let i = 0, j = 1;;) {
  setTimeout(() => console.log(i));
  if (++i > 3) break;
}
");
}
