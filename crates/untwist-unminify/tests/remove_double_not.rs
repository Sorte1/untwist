use untwist_testing::assert_transform;

#[test]
fn if_statement() {
    assert_transform!(
        "if (!!a) {}",
        [untwist_unminify::remove_double_not::boxed],
        @"if (a) {}"
    );
}

#[test]
fn ternary() {
    assert_transform!(
        "!!a ? b : c;",
        [untwist_unminify::remove_double_not::boxed],
        @"a ? b : c;"
    );
}

#[test]
fn triple_not() {
    assert_transform!(
        "function f() {\n  return !!!a;\n}",
        [untwist_unminify::remove_double_not::boxed],
        @r"
    function f() {
      return !a;
    }
    "
    );
}

#[test]
fn array_filter() {
    assert_transform!(
        "[].filter(a => !!a);",
        [untwist_unminify::remove_double_not::boxed],
        @"[].filter((a) => a);"
    );
}

#[test]
fn keeps_double_not_used_as_a_value() {
    assert_transform!(
        "x = !!a;",
        [untwist_unminify::remove_double_not::boxed],
        @"x = !!a;"
    );
}
