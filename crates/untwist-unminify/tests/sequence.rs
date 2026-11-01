use untwist_testing::assert_transform;

#[test]
fn to_statements() {
    assert_transform!("if (a) b(), c();", [untwist_unminify::sequence::boxed], @r"
if (a) {
  b();
  c();
}
");
}

#[test]
fn rearrange_from_return() {
    assert_transform!("function f() { return a(), b(), c(); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  a();
  b();
  return c();
}
");
}

#[test]
fn rearrange_from_if() {
    assert_transform!("if (a(), b()) c();", [untwist_unminify::sequence::boxed], @r"
a();
if (b()) c();
");
}

#[test]
fn rearrange_from_switch() {
    assert_transform!("switch (a(), b()) {}", [untwist_unminify::sequence::boxed], @r"
a();
switch (b()) {}
");
}

#[test]
fn throw() {
    assert_transform!("throw a(), b();", [untwist_unminify::sequence::boxed], @r"
a();
throw b();
");
}

#[test]
fn rearrange_from_for_in() {
    assert_transform!("for (let key in a = 1, object) {}", [untwist_unminify::sequence::boxed], @r"
a = 1;
for (let key in object) {}
");
}

#[test]
fn rearrange_from_for_of() {
    assert_transform!("for (let value of (a = 1, array)) {}", [untwist_unminify::sequence::boxed], @r"
a = 1;
for (let value of array) {}
");
}

#[test]
fn rearrange_from_for_loop_init() {
    assert_transform!("for((a(), b());;);", [untwist_unminify::sequence::boxed], @r"
a();
b();
for (;;);
");
    assert_transform!("if (1) for ((a(), b());;) {}", [untwist_unminify::sequence::boxed], @r"
if (1) {
  a();
  b();
  for (;;) {}
}
");
}

#[test]
fn rearrange_from_for_loop_update() {
    assert_transform!("for(; i < 10; a(), b(), i++);", [untwist_unminify::sequence::boxed], @r"
for (; i < 10; i++) {
  a();
  b();
}
");
}

#[test]
fn dont_rearrange_from_while() {
    assert_transform!("while (a(), b()) c();", [untwist_unminify::sequence::boxed], @"while (a(), b()) c();");
}

#[test]
fn dont_rearrange_from_do_while() {
    assert_transform!("do {} while (a(), b());", [untwist_unminify::sequence::boxed], @"do {} while (a(), b());");
}

#[test]
fn rearrange_variable_declarator() {
    assert_transform!("var a = (b(), c());", [untwist_unminify::sequence::boxed], @r"
b();
var a = c();
");
    assert_transform!("for (let a = (b(), c());;) {}", [untwist_unminify::sequence::boxed], @r"
b();
for (let a = c();;) {}
");
}

#[test]
fn rearrange_assignment_to_an_uncaptured_local() {
    assert_transform!("function f() { var a = {}; a.x = (b(), c()); a[1] = (b(), c()); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  var a = {};
  b();
  a.x = c();
  b();
  a[1] = c();
}
");
}

#[test]
fn dont_rearrange_when_the_moved_part_can_touch_the_target() {
    assert_transform!("var a = 1; a += (a = 5, 1);", [untwist_unminify::sequence::boxed], @"var a = 1;\na += (a = 5, 1);");
    assert_transform!("a += (b(), 1);", [untwist_unminify::sequence::boxed], @"a += (b(), 1);");
    assert_transform!("function f() { var o = {}; o.k += (o.k = 5, 1); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  var o = {};
  o.k += (o.k = 5, 1);
}
");
    assert_transform!("function f() { var o = {}, i = 0; o[i] = (i++, 5); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  var o = {}, i = 0;
  o[i] = (i++, 5);
}
");
    assert_transform!("function f() { var o = {}; function g() { o = {}; } o.k = (g(), 5); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  var o = {};
  function g() {
    o = {};
  }
  o.k = (g(), 5);
}
");
}

#[test]
fn dont_rearrange_past_eval_or_with_around_a_local() {
    assert_transform!("function f() { var o = {}; eval(s); o.k = (g(), 5); }", [untwist_unminify::sequence::boxed], @r"
function f() {
  var o = {};
  eval(s);
  o.k = (g(), 5);
}
");
    assert_transform!("function f(w) { var o = {}; with (w) { o.k = (g(), 5); } }", [untwist_unminify::sequence::boxed], @r"
function f(w) {
  var o = {};
  with(w) {
    o.k = (g(), 5);
  }
}
");
}

#[test]
fn rearrange_assignment() {
    assert_transform!("a = (b(), c());", [untwist_unminify::sequence::boxed], @r"
b();
a = c();
");
    assert_transform!("a.x = (b(), c());", [untwist_unminify::sequence::boxed], @"a.x = (b(), c());");
    assert_transform!("a[1] = (b(), c());", [untwist_unminify::sequence::boxed], @"a[1] = (b(), c());");
    assert_transform!("a[x()] = (b(), c());", [untwist_unminify::sequence::boxed], @"a[x()] = (b(), c());");
    assert_transform!("console.log(a = (b(), c()));", [untwist_unminify::sequence::boxed], @"console.log((b(), a = c()));");
    assert_transform!("while (a = (b(), c()));", [untwist_unminify::sequence::boxed], @"while (b(), a = c());");
    assert_transform!("a ||= (b(), c());\na &&= (b(), c());\na ??= (b(), c());", [untwist_unminify::sequence::boxed], @r"
a ||= (b(), c());
a &&= (b(), c());
a ??= (b(), c());
");
    assert_transform!("for (;;) a = (b(), c());", [untwist_unminify::sequence::boxed], @r"
for (;;) {
  b();
  a = c();
}
");
}

#[test]
fn simplify_computed_property_with_only_literals() {
    assert_transform!(r#"Lr[("nnQB", "jcIgN")]();"#, [untwist_unminify::sequence::boxed], @r#"Lr["jcIgN"]();"#);
}

const LET_UPDATE_CLOSURE_FIRST: &str = "var fs = []; for (let i = 0; i < 3; fs.push(function(){ return i; }), i++); console.log(fs.map(function(f){ return f(); }).join());";

const LET_UPDATE_CLOSURE_LAST: &str = "var fs = []; for (let i = 0; i < 3; i++, fs.push(() => i)); console.log(fs.map(function(f){ return f(); }).join());";

fn sequence_output(source: &str, extended: bool) -> String {
    let alloc = untwist_testing::Allocator::default();
    untwist_testing::transform_with(
        &alloc,
        source,
        &[untwist_unminify::sequence::boxed],
        untwist_core::pass::PassOptions {
            extended,
            ..untwist_core::pass::PassOptions::default()
        },
    )
}

#[test]
fn a_let_loop_update_with_a_closure_stays_in_the_update_in_either_mode() {
    for source in [LET_UPDATE_CLOSURE_FIRST, LET_UPDATE_CLOSURE_LAST] {
        for extended in [true, false] {
            let output = sequence_output(source, extended);
            assert!(output.contains(");\nconsole.log"), "{output}");
            untwist_testing::node::assert_node_agrees(source, &output);
        }
    }
}

#[test]
fn a_var_loop_update_with_a_closure_still_moves_into_the_body() {
    let source = "var fs = []; for (var i = 0; i < 3; fs.push(function(){ return i; }), i++); console.log(fs.map(function(f){ return f(); }).join());";
    let output = sequence_output(source, true);
    assert!(output.contains("i++) {"), "{output}");
    untwist_testing::node::assert_node_agrees(source, &output);
}
