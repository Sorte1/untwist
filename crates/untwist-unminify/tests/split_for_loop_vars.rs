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

#[test]
fn extract_a_read_only_binding_the_test_reads() {
    assert_transform!("for (var r = s.split(\",\"), t = [], n = 0; n < r.length; n++) t.push(r[n]);", [untwist_unminify::split_for_loop_vars::read_only_tests_boxed], @r#"
    var r = s.split(",");
    var t = [];
    for (var n = 0; n < r.length; n++) t.push(r[n]);
    "#);
}

#[test]
fn keep_a_test_binding_the_loop_writes() {
    assert_transform!("for (var r = s, n = 0; n < r.length; n++) r = r.slice(1);", [untwist_unminify::split_for_loop_vars::read_only_tests_boxed], @"for (var r = s, n = 0; n < r.length; n++) r = r.slice(1);");
}

#[test]
fn the_unminify_stage_keeps_a_binding_the_test_reads() {
    assert_transform!("for (var r = s, n = 0; n < r.length; n++) t(r[n]);", [untwist_unminify::split_for_loop_vars::boxed], @"for (var r = s, n = 0; n < r.length; n++) t(r[n]);");
}
