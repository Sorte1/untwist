use untwist_testing::assert_transform;

#[test]
fn split_variable_declarations() {
    assert_transform!("const a = 1, b = 2, c = 3;", [untwist_unminify::split_variable_declarations::boxed], @r"
const a = 1;
const b = 2;
const c = 3;
");
}

#[test]
fn split_exported_variable_declarations() {
    assert_transform!("export const a = 1, b = 2, c = 3;", [untwist_unminify::split_variable_declarations::boxed], @r"
export const a = 1;
export const b = 2;
export const c = 3;
");
}

#[test]
fn split_var_in_for_loop_initializer() {
    assert_transform!("for (var i = 0, j = 1;;) {}", [untwist_unminify::split_variable_declarations::boxed], @r"
var i = 0;
var j = 1;
for (;;) {}
");
}

#[test]
fn ignore_let_in_for_loop_initializer() {
    assert_transform!("let i;\nfor (let i = 0, j = 1;;) {}", [untwist_unminify::split_variable_declarations::boxed], @r"
let i;
for (let i = 0, j = 1;;) {}
");
    assert_transform!("for (let i = 0, j = 1;;) { setTimeout(() => console.log(i)); if (++i > 3) break; }", [untwist_unminify::split_variable_declarations::boxed], @r"
for (let i = 0, j = 1;;) {
  setTimeout(() => console.log(i));
  if (++i > 3) break;
}
");
}

#[test]
fn ignore_for_loop_with_test_or_update() {
    assert_transform!("for (var i = 0, j = 1; i < 10; i++, j++) var a, b;", [untwist_unminify::split_variable_declarations::boxed], @r"
for (var i = 0, j = 1; i < 10; i++, j++) {
  var a;
  var b;
}
");
}
