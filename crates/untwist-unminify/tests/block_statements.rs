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
