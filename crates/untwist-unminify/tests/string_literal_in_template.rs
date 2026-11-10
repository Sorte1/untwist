use untwist_testing::assert_transform;

#[test]
fn string_literal_in_template_literal() {
    assert_transform!(
        r#"const a = `Hello ${'World'}!`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @"const a = `Hello World!`;"
    );
    assert_transform!(
        r#"const a = `Hello ${'World'}${'!'}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @"const a = `Hello World!`;"
    );
    assert_transform!(
        r#"const a = `${"Hi!"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @"const a = `Hi!`;"
    );
    assert_transform!(
        r#"const a = `${"`"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"const a = `\``;"
    );
    assert_transform!(
        r#"const a = `${"${x}"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"const a = `\${x}`;"
    );
    assert_transform!(
        r#"const a = `${"\\"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"const a = `\\`;"
    );
}

#[test]
fn ignore_lone_surrogate_string_literal() {
    assert_transform!(
        r#"`a${"\ud800"}b`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r#"`a${"\ud800"}b`;"#
    );
}

#[test]
fn nul_before_a_digit_stays_a_nul() {
    assert_transform!(
        r#"var d = `2${"\0"}3`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"var d = `2\x003`;"
    );
    assert_transform!(
        r#"var n = `a${"\08"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"var n = `a\x008`;"
    );
    assert_transform!(
        r#"var p = `\0${"1"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"var p = `\x001`;"
    );
}

#[test]
fn dollar_before_a_merged_brace_stays_text() {
    assert_transform!(
        r#"var a = `$${"{1}"}`;"#,
        [untwist_unminify::string_literal_in_template::boxed],
        @r"var a = `\${1}`;"
    );
}
