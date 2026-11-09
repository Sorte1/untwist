use untwist_testing::assert_transform;
use untwist_unminify::block_statements::boxed;

#[test]
fn if_statement() {
    assert_transform!("if (a) b();", [boxed], @r"
    if (a) {
      b();
    }
    ");
}

#[test]
fn while_statement() {
    assert_transform!("while (a) b();", [boxed], @r"
    while (a) {
      b();
    }
    ");
}

#[test]
fn for_statement() {
    assert_transform!("for (;;) b();", [boxed], @r"
    for (;;) {
      b();
    }
    ");
}

#[test]
fn for_in_statement() {
    assert_transform!("for (const key in object) b();", [boxed], @r"
    for (const key in object) {
      b();
    }
    ");
}

#[test]
fn for_of_statement() {
    assert_transform!("for (const item of array) b();", [boxed], @r"
    for (const item of array) {
      b();
    }
    ");
}

#[test]
fn arrow_function() {
    assert_transform!("const x = () => (a(), b());", [boxed], @r"
    const x = () => {
      return a(), b();
    };
    ");
}

#[test]
fn ignore_empty_statement() {
    assert_transform!("while (arr.pop());", [boxed], @"while (arr.pop());");
}

#[test]
fn else_if_chain_alternate_stays_unwrapped() {
    assert_transform!("if (a) b(); else if (c) d(); else e();", [boxed], @r"
    if (a) {
      b();
    } else if (c) {
      d();
    } else {
      e();
    }
    ");
}

#[test]
fn empty_statements_in_statement_lists() {
    assert_transform!("a();;\nfunction f() { ; b(); ; }\nswitch (x) { case 1: ; }\n{ ; }", [boxed], @r"
    a();
    function f() {
      b();
    }
    switch (x) {
      case 1:
    }
    {}
    ");
}

#[test]
fn empty_statement_that_keeps_a_string_out_of_the_directive_prologue() {
    assert_transform!("function f() { ; ; \"use strict\"; return this; }", [boxed], @r#"
    function f() {
      ;
      "use strict";
      return this;
    }
    "#);
}

#[test]
fn empty_statement_bodies_are_kept() {
    assert_transform!("if (a); else;\nfor (;;);\nlbl:;", [boxed], @r"
    if (a);
    else {}
    for (;;);
    lbl:;
    ");
}
