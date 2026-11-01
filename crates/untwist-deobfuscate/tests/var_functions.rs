mod common;

#[test]
fn the_whole_machinery_may_be_var_functions() {
    let (code, _notes) = common::run_passes(
        "\
var a = function () { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); };
var b = function (c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); };
var w = function (c, d) { return b(c - 0x0, d); };
console[w(0x0, 0x0)](w(0x1, 0x0));",
    );
    assert_eq!(code.trim(), "console[\"log\"](\"Hello World!\");");
}

const MACHINERY: &str = "KEYWORD a = function () { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); };\nvar b = function (c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); };\nconsole[b(0x0, 0x0)](b(0x1, 0x0));";

#[test]
fn a_let_or_const_function_is_not_treated_as_the_machinery() {
    let with_var = MACHINERY.replace("KEYWORD", "var");
    let (decoded, _notes) = common::run_passes(&with_var);
    assert_eq!(decoded.trim(), "console[\"log\"](\"Hello World!\");");

    for keyword in ["let", "const"] {
        let source = MACHINERY.replace("KEYWORD", keyword);
        let (code, _notes) = common::run_passes(&source);
        assert!(
            !code.contains("console[\"log\"](\"Hello World!\")"),
            "{keyword}: {code}"
        );
        assert!(!code.contains("__STRING_ARRAY__"), "{keyword}: {code}");
        assert!(!code.contains("__DECODE_"), "{keyword}: {code}");
    }
}

#[test]
fn merge_strings_alone_merges_the_split_debugger_the_fixture_carries() {
    let alloc = untwist_testing::Allocator::default();
    let code = untwist_testing::transform(
        &alloc,
        "console.log(\"debu\" + \"gger\");",
        &[untwist_unminify::merge_strings::boxed],
    );
    assert_eq!(code.trim(), "console.log(\"debugger\");");
}

#[test]
fn the_var_functions_fixture_whose_rotator_runs_before_its_array_is_bound_is_refused() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/reference/deobfuscate/obfuscator.io-var-functions.js"),
    )
    .expect("fixture");
    let out = untwist::Pipeline::new(untwist::Options {
        unsafe_transforms: true,
        ..untwist::Options::default()
    })
    .run(&source);
    assert!(
        !out.code().contains("console.log(\"Hello World!\")"),
        "{}",
        out.code()
    );
    assert!(out.code().contains("var c = function"), "{}", out.code());
    assert!(
        out.notes
            .iter()
            .any(|note| note.message == untwist_deobfuscate::notes::MACHINERY_NOT_INITIALISED),
        "{:?}",
        out.notes
    );
}
