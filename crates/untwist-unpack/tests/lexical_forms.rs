use std::path::Path;

use oxc_allocator::Allocator;
use untwist_unpack::detect::unpack_program;

fn fixture(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-bundlers");
    std::fs::read_to_string(root.join(path)).expect("read fixture")
}

fn split(source: &str) -> Option<(&'static str, usize)> {
    let alloc = Allocator::default();
    let parsed = untwist_core::parse::parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    unpack_program(&alloc, &parsed.program)
        .map(|bundle| (bundle.kind.as_str(), bundle.modules.len()))
}

fn detected(source: &str) -> Option<&'static str> {
    split(source).map(|(kind, _)| kind)
}

fn rekeyed(path: &str, from: &str, keyword: &str) -> String {
    let source = fixture(path);
    let to = from.replacen("var ", &format!("{keyword} "), 1);
    assert!(source.contains(from), "{path} lacks {from:?}");
    source.replace(from, &to)
}

fn runtime_rekeyed(path: &str, keyword: &str) -> String {
    let source = fixture(path);
    let body = source.find("var globalObject").expect("runtime body");
    let mut out = source[..body].to_string();
    out.push_str(&source[body..].replacen(
        "var globalObject",
        &format!("{keyword} globalObject"),
        1,
    ));
    out
}

const ESBUILD: &str = "esbuild/esbuild-iife.js";

#[test]
fn an_esbuild_helper_declared_const_is_still_a_helper() {
    let source = rekeyed(ESBUILD, "var __commonJS =", "const");
    assert_eq!(split(&source), Some(("esbuild", 5)));
}

#[test]
fn an_esbuild_helper_declared_let_is_not_a_helper() {
    let source = rekeyed(ESBUILD, "var __commonJS =", "let");
    assert_eq!(split(&source), Some(("esbuild", 1)));
}

#[test]
fn an_esbuild_lazy_module_declared_const_is_still_split() {
    let source = rekeyed(ESBUILD, "var require_data = __commonJS(", "const");
    assert_eq!(split(&source), Some(("esbuild", 5)));
}

#[test]
fn an_esbuild_lazy_module_declared_let_is_not_split() {
    let source = rekeyed(ESBUILD, "var require_data = __commonJS(", "let");
    assert_eq!(split(&source), Some(("esbuild", 4)));
}

#[test]
fn a_closure_window_alias_declared_const_is_still_the_alias() {
    let source = rekeyed("closure/min-closure.js", "var window = this;", "const");
    assert_eq!(detected(&source), Some("closure"));
}

#[test]
fn a_closure_window_alias_declared_let_is_refused() {
    let source = rekeyed("closure/min-closure.js", "var window = this;", "let");
    assert_eq!(detected(&source), None);
}

#[test]
fn a_parcel_runtime_in_a_recorded_form_is_proven() {
    assert_eq!(
        detected(&fixture("parcel/single-nohoist.js")),
        Some("parcel")
    );
}

#[test]
fn a_parcel_runtime_outside_the_recorded_forms_is_refused() {
    let source = runtime_rekeyed("parcel/single-nohoist.js", "let");
    assert_eq!(detected(&source), None);
}
