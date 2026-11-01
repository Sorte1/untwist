use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking, BundleKind, UnpackResult};

const NESTED: [&str; 3] = [
    "esbuild/min-esbuild-in-umd.js",
    "esbuild/min-esbuild-in-amd.js",
    "esbuild/min-esbuild-in-system.js",
];

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/unpack-bundlers")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

fn files(result: Option<UnpackResult<'_>>) -> Option<(BundleKind, Vec<(String, String)>)> {
    let result = result?;
    let mut files: Vec<(String, String)> = result
        .bundle
        .modules
        .values()
        .map(|module| {
            (
                module.path.as_str().to_string(),
                pretty_codegen(module.program()),
            )
        })
        .collect();
    files.sort();
    Some((result.bundle.kind, files))
}

fn split(source: &str) -> Option<(BundleKind, Vec<(String, String)>)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    files(unpack(&alloc, &parsed.program, &[]))
}

fn split_taken(source: &str) -> Option<(BundleKind, Vec<(String, String)>)> {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    files(unpack_taking(&alloc, &mut parsed.program, &[]))
}

fn kind_of(source: &str) -> Option<BundleKind> {
    split(source).map(|(kind, _)| kind)
}

#[test]
fn an_esbuild_bundle_inside_a_wrapper_is_split_by_esbuild() {
    for name in NESTED {
        let (kind, files) = split(&fixture(name)).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(kind, BundleKind::Esbuild, "{name}");
        let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
        assert!(paths.contains(&"src/a.js"), "{name}: {paths:?}");
        assert!(paths.contains(&"src/b.js"), "{name}: {paths:?}");
        assert!(paths.contains(&"index.js"), "{name}: {paths:?}");
    }
}

#[test]
fn taking_and_cloning_split_alike() {
    for name in NESTED {
        let source = fixture(name);
        assert_eq!(split(&source), split_taken(&source), "{name}");
    }
}

#[test]
fn a_factory_with_a_dependency_keeps_the_outer_module() {
    let source = fixture("esbuild/min-esbuild-in-umd.js")
        .replace("define([], factory)", "define([\"exports\"], factory)")
        .replace(
            "factory();\n  else factory();",
            "factory(exports);\n  else factory({});",
        )
        .replace("})(this, function () {", "})(this, function (exports) {");
    assert_eq!(kind_of(&source), Some(BundleKind::Umd));
    let source =
        fixture("esbuild/min-esbuild-in-amd.js").replace("define([],", "define([\"path\"],");
    assert_eq!(kind_of(&source), Some(BundleKind::Amd));
}

#[test]
fn a_wrapper_holding_no_bundle_is_unchanged() {
    for (name, kind) in [
        ("umd/min-umd.js", BundleKind::Umd),
        ("amd/min-amd.js", BundleKind::Amd),
        ("systemjs/min-system.js", BundleKind::SystemJs),
    ] {
        assert_eq!(kind_of(&fixture(name)), Some(kind), "{name}");
    }
}

#[test]
fn amd_with_several_defines_is_not_refed() {
    let inner = fixture("esbuild/min-esbuild-in-amd.js");
    let source = format!(
        "{}{}",
        inner.replace("define([],", "define(\"lib\", [],"),
        "define(\"main\", [], function () { console.log(\"main\"); });\n"
    );
    assert_eq!(kind_of(&source), Some(BundleKind::Amd));
}

#[test]
fn a_nested_wrapper_is_not_refed_again() {
    let inner = fixture("umd/min-umd.js").replace("})(this,", "})(globalThis,");
    let source = format!(
        "(function (root, factory) {{\n  if (typeof define === \"function\" && define.amd) define([], factory);\n  else if (typeof exports === \"object\" && typeof module !== \"undefined\") factory();\n  else factory();\n}})(this, function () {{\n{inner}}});\n"
    );
    assert!(matches!(kind_of(&source), None | Some(BundleKind::Umd)));
}
