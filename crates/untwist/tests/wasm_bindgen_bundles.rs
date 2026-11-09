#[path = "common/wasm_module.rs"]
mod wasm_module;

use untwist::{Options, Output, Pipeline, SuppliedWasm, WasmOrigin, WasmSupply};
use wasm_module::{sig, Module, Sig, I32};

const WEBPACK5_HASH: &str = "5f0a1b2c3d4e5f607182.module.wasm";
const WEBPACK4_HASH: &str = "0a1b2c3d4e5f60718293.module.wasm";
const GLUE: &str = "./hello_wasm_bg.js";
const PAIR: Sig<'static> = sig(&[I32, I32], &[]);
const PAIR_RESULT: Sig<'static> = sig(&[I32, I32], &[I32]);
const QUAD_RESULT: Sig<'static> = sig(&[I32, I32, I32, I32], &[I32]);
const UNIT: Sig<'static> = sig(&[], &[]);

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("{}/tests/wasm/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn hello_module(exports: &[&'static str]) -> Vec<u8> {
    let all = [
        ("greet", PAIR),
        ("add", PAIR_RESULT),
        ("__wbindgen_malloc", PAIR_RESULT),
        ("__wbindgen_realloc", QUAD_RESULT),
        ("__webpack_init__", UNIT),
    ];
    Module {
        imports: vec![
            (GLUE, "__wbg_alert_e8a2a3b7c1d4f5e6", PAIR),
            (GLUE, "__wbindgen_throw", PAIR),
        ],
        functions: all
            .into_iter()
            .filter(|(name, _)| exports.contains(name))
            .collect(),
        memory: Some("memory"),
    }
    .bytes()
}

const FULL: &[&str] = &["greet", "add", "__wbindgen_malloc", "__wbindgen_realloc"];
const FULL_WEBPACK4: &[&str] = &[
    "greet",
    "add",
    "__wbindgen_malloc",
    "__wbindgen_realloc",
    "__webpack_init__",
];

fn supply(name: &str, bytes: Vec<u8>) -> WasmSupply {
    WasmSupply {
        files: vec![SuppliedWasm {
            name: name.to_string(),
            bytes,
            origin: WasmOrigin::Supplied,
            url: None,
        }],
    }
}

fn settled(source: &str, supply: &WasmSupply) -> Output {
    let mut out = Pipeline::new(Options::default()).run(source);
    out.settle_wasm(supply, true);
    out
}

fn module_code<'o>(out: &'o Output, path: &str) -> &'o str {
    out.bundle
        .as_ref()
        .expect("the bundle unpacks")
        .modules
        .iter()
        .find(|module| module.path == path)
        .map(|module| module.code.as_str())
        .unwrap_or_else(|| panic!("no module {path}"))
}

fn wasm_notes(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "wasm")
        .map(|note| note.message.as_str())
        .collect()
}

fn assert_named(text: &str, names: &[&str]) {
    for name in names {
        assert!(text.contains(name), "{name} missing\n{text}");
    }
}

const WRAPPERS: &[&str] = &[
    "function greet(e) {",
    "greet(t, n);",
    "function add(e, t) {",
    "return r.add(e, t) >>> 0;",
    "function __wbg_alert_e8a2a3b7c1d4f5e6(e, t) {",
    "function __wbindgen_throw(e, t) {",
    "Kz: () => greet",
    "Or: () => add",
];

#[test]
fn webpack5_bindgen_glue_handed_the_wasm_namespace_is_named_from_the_module() {
    let out = settled(
        &fixture("bindgen-webpack5.js"),
        &supply(WEBPACK5_HASH, hello_module(FULL)),
    );
    assert!(out.wasm_unresolved.is_empty());
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm[0].renamed, 5);
    let glue = module_code(&out, "311.js");
    assert_named(glue, WRAPPERS);
    assert_named(
        glue,
        &["function __wbg_set_wasm(e) {", "oT: () => __wbg_set_wasm"],
    );
    assert!(glue.contains("let r;"), "{glue}");
    let code = out.code();
    assert_named(code, WRAPPERS);
    assert_named(code, &["function __wbg_set_wasm(e) {"]);
}

#[test]
fn webpack5_bindgen_glue_that_imports_the_wasm_namespace_itself_is_named() {
    let out = settled(
        &fixture("bindgen-webpack5-direct.js"),
        &supply(WEBPACK5_HASH, hello_module(FULL)),
    );
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm[0].renamed, 4);
    assert_named(module_code(&out, "311.js"), WRAPPERS);
    assert_named(out.code(), WRAPPERS);
}

#[test]
fn webpack4_sync_bindgen_glue_with_numeric_module_ids_is_named() {
    let out = settled(
        &fixture("bindgen-webpack4.js"),
        &supply(WEBPACK4_HASH, hello_module(FULL_WEBPACK4)),
    );
    assert!(out.wasm_unresolved.is_empty());
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    assert_eq!(out.wasm[0].renamed, 4);
    let glue = module_code(&out, "311.js");
    assert_named(
        glue,
        &[
            "function greet(e) {",
            "function add(e, t) {",
            "function __wbg_alert_e8a2a3b7c1d4f5e6(e, t) {",
            "return greet;",
        ],
    );
    assert_named(out.code(), &["function greet(e) {", "function add(e, t) {"]);
}

#[test]
fn a_module_missing_an_export_the_glue_reads_through_the_namespace_names_nothing() {
    let without_add = &["greet", "__wbindgen_malloc", "__wbindgen_realloc"];
    for (source, name) in [
        ("bindgen-webpack5.js", WEBPACK5_HASH),
        ("bindgen-webpack5-direct.js", WEBPACK5_HASH),
    ] {
        let out = settled(&fixture(source), &supply(name, hello_module(without_add)));
        assert_eq!(out.wasm[0].renamed, 0, "{source}");
        let glue = module_code(&out, "311.js");
        assert!(glue.contains("function c(e) {"), "{glue}");
        assert!(glue.contains("function a(e, t) {"), "{glue}");
        let notes = wasm_notes(&out);
        assert!(
            notes.iter().any(
                |note| note.contains("does not match the glue around 718.js")
                    && note.contains("export add")
            ),
            "{source}: {notes:?}"
        );
    }
}

#[test]
fn without_the_module_the_glue_keeps_its_names_and_the_module_is_listed() {
    let out = settled(&fixture("bindgen-webpack5.js"), &WasmSupply::default());
    let names: Vec<&str> = out
        .wasm_unresolved
        .iter()
        .map(|reference| reference.name.as_str())
        .collect();
    assert_eq!(names, [WEBPACK5_HASH]);
    assert!(out.wasm.is_empty());
    let glue = module_code(&out, "311.js");
    assert!(glue.contains("function c(e) {"), "{glue}");
    assert!(glue.contains("function o(e) {"), "{glue}");
}

#[test]
fn a_setter_handed_something_other_than_the_wasm_namespace_names_no_wrappers() {
    let source = fixture("bindgen-webpack5.js").replace("(0,i.oT)(o)", "(0,i.oT)(i)");
    let out = settled(&source, &supply(WEBPACK5_HASH, hello_module(FULL)));
    assert_eq!(out.wasm[0].renamed, 2);
    let glue = module_code(&out, "311.js");
    assert_named(glue, &["function __wbg_alert_e8a2a3b7c1d4f5e6(e, t) {"]);
    assert!(glue.contains("function c(e) {"), "{glue}");
    assert!(glue.contains("function o(e) {"), "{glue}");
}

#[test]
fn a_setter_also_called_with_another_value_names_no_wrappers() {
    let source =
        fixture("bindgen-webpack5.js").replace("(0,i.oT)(o),", "(0,i.oT)(o),(0,i.oT)({}),");
    let out = settled(&source, &supply(WEBPACK5_HASH, hello_module(FULL)));
    assert_eq!(out.wasm[0].renamed, 2);
    assert!(module_code(&out, "311.js").contains("function c(e) {"));
}

#[test]
fn a_call_through_a_shadowing_binding_does_not_name_its_function() {
    let source = fixture("bindgen-webpack5.js").replace(
        "function l(e,t){return r.add(e,t)>>>0}",
        "function l(e,t){const r={add:Math.max};return r.add(e,t)>>>0}",
    );
    let out = settled(&source, &supply(WEBPACK5_HASH, hello_module(FULL)));
    let glue = module_code(&out, "311.js");
    assert!(glue.contains("function l(e, t) {"), "{glue}");
    assert!(glue.contains("function greet(e) {"), "{glue}");
}

#[test]
fn two_functions_calling_the_same_export_keep_their_names() {
    let source = fixture("bindgen-webpack5.js").replace(
        "function l(e,t){return r.add(e,t)>>>0}",
        "function l(e,t){return r.add(e,t)>>>0}function k(e){return r.add(e,e)}",
    );
    let out = settled(&source, &supply(WEBPACK5_HASH, hello_module(FULL)));
    let glue = module_code(&out, "311.js");
    assert!(glue.contains("function l(e, t) {"), "{glue}");
    assert!(glue.contains("function k(e) {"), "{glue}");
    assert!(glue.contains("function greet(e) {"), "{glue}");
}
