use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_unpack::{unpack_program_with, UnpackOptions};

const ON: UnpackOptions = UnpackOptions {
    split_hoisted: true,
    conservative: false,
    trust_free_loader: false,
};

const BODY: &str = "function a() { return 1; }
function b() { return a() + 1; }
const c = b();
function d() { return 2; }
function e() { return d() * 2; }
const f = e();
console.log(c, f);
";

fn split_count(source: &str) -> Option<usize> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    unpack_program_with(&alloc, &parsed.program, ON).map(|bundle| bundle.modules.len())
}

#[test]
fn an_es_module_with_static_imports_or_exports_is_split() {
    let source = format!("import path from \"node:path\";\n{BODY}export {{ c, f, path }};\n");
    assert!(
        split_count(&source).is_some_and(|count| count >= 2),
        "{source}"
    );
}

#[test]
fn a_program_without_module_syntax_is_not_split_as_a_module() {
    assert_eq!(split_count(BODY), None);
}

#[test]
fn shapes_that_change_evaluation_order_in_split_modules_are_refused() {
    let cases = [
        format!("{BODY}await Promise.resolve();\nexport {{ c }};\n"),
        format!("{BODY}export * from \"node:path\";\n"),
        format!("{BODY}import path from \"node:path\";\nexport {{ path }};\n"),
        format!("{BODY}const fs = require(\"node:fs\");\nexport {{ fs }};\n"),
        format!("{BODY}console.log(import.meta.url);\nexport {{ c }};\n"),
    ];
    for source in cases {
        assert_eq!(split_count(&source), None, "{source}");
    }
}
