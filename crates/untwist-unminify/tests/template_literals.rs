use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::build_scoping;
use untwist_testing::assert_transform;

#[test]
fn preserve_parens_does_not_panic() {
    let alloc = Allocator::default();
    let source = "(`a`) + b;\n(\"x\".concat)(y);\n";
    let mut parsed = Parser::new(&alloc, source, SourceType::mjs()).parse();
    let sem = build_scoping(&parsed.program);
    let mut group = PassGroup::from_ctors(
        &[untwist_unminify::template_literals::boxed],
        PassOptions::default(),
    );
    let _ = run_to_fixpoint(&alloc, &mut parsed.program, sem, &mut group);
}

#[test]
fn escape_sequences() {
    assert_transform!(
        "\"'\".concat(foo, \"' \\\"\").concat(bar, \"\\\"\").concat(\"$\\0\\b\\f\\n\\r\\t\\v\");",
        [untwist_unminify::template_literals::boxed],
        @r#"
`'${foo}' "${bar}"\$\0\b\f
\r\t\v`;
"#
    );
}

#[test]
fn expressions() {
    assert_transform!(
        r#""".concat(1);
1 + "".concat(foo).concat(bar).concat(baz);
1 + "".concat(foo, "bar").concat(baz);
"".concat(1, f, "oo", true).concat(b, "ar", 0).concat(baz);"#,
        [untwist_unminify::template_literals::boxed],
        @r#"
`${1}`;
`${1}${foo}${bar}${baz}`;
`${1}${foo}bar${baz}`;
`${1}${f}oo${true}${b}ar${0}${baz}`;
"#
    );
}

#[test]
fn merge_concatenations() {
    assert_transform!(
        r#"`a${1}b` + 'c';
`a${1}b` + `c${2}d`;
`a${1}b` + 2;
'c' + `a${1}b`;
2 + `a${1}b`;"#,
        [untwist_unminify::template_literals::boxed],
        @r#"
`a${1}bc`;
`a${1}bc${2}d`;
`a${1}b${2}`;
`ca${1}b`;
`${2}a${1}b`;
"#
    );
}

#[test]
fn ignore_lone_surrogate_receiver() {
    assert_transform!(
        "\"\\ud800\".concat(b);",
        [untwist_unminify::template_literals::boxed],
        @r#""\ud800".concat(b);"#
    );
}
