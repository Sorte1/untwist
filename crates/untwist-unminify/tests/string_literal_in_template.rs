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
