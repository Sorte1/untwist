#[path = "common/wasm_module.rs"]
mod wasm_module;

use untwist::{Options, Output, Pipeline, WasmCheck, WasmSupply};
use wasm_module::{base64, sig, Module, Sig, F64, I32};

const VOID: Sig<'static> = sig(&[], &[]);
const MEMCPY: Sig<'static> = sig(&[I32, I32, I32], &[]);
const NOW: Sig<'static> = sig(&[], &[F64]);
const RESIZE: Sig<'static> = sig(&[I32], &[I32]);

const GLUE: &str = r#"var Module=typeof Module!="undefined"?Module:{};var wasmBinaryFile="data:application/octet-stream;base64,PAYLOAD";var Z,W,HEAPU8;function Pa(e,t,n){HEAPU8.copyWithin(e,t,t+n)}function Qa(){throw new Error("abort")}function Ra(){return Date.now()}function Ta(e){try{W.grow(e-W.buffer.byteLength+65535>>>16);HEAPU8=new Uint8Array(W.buffer);return 1}catch(t){}}var Sa=e=>{var t=HEAPU8.length;e>>>=0;var n=2147483648;if(e>n)return!1;for(var r=1;r<=4;r*=2){var o=t*(1+.2/r);o=Math.min(o,e+100663296);var i=Math.min(n,Math.ceil(Math.max(e,o)/65536)*65536);if(Ta(i))return!0}return!1};var Ya={a:Pa,b:Qa,c:Ra,d:Sa};function Hb(e){Z=e.exports;W=Z.e;HEAPU8=new Uint8Array(W.buffer);return Z}function Rb(e){var t=atob(e.slice(37)),n=new Uint8Array(t.length);for(var r=0;r<t.length;++r)n[r]=t.charCodeAt(r);return n}function Ib(){var e={a:Ya};return WebAssembly.instantiate(Rb(wasmBinaryFile),e).then(t=>Hb(t.instance))}var Xb=()=>(Xb=Z.f)();var Yb=Module._malloc=e=>(Yb=Module._malloc=Z.g)(e);Ib().then(()=>{Xb();Yb(Ra());Pa(1,2,3);Sa(5)});"#;

fn module_with(imports: Vec<(&'static str, &'static str, Sig<'static>)>) -> Vec<u8> {
    Module {
        imports,
        functions: vec![("f", VOID), ("g", RESIZE)],
        memory: Some("e"),
    }
    .bytes()
}

fn covering_imports() -> Vec<(&'static str, &'static str, Sig<'static>)> {
    vec![
        ("a", "a", MEMCPY),
        ("a", "b", VOID),
        ("a", "c", NOW),
        ("a", "d", RESIZE),
    ]
}

fn settled(glue: &str, bytes: &[u8]) -> Output {
    let source = glue.replace("PAYLOAD", &base64(bytes));
    let mut output = Pipeline::new(Options {
        wasm_check: WasmCheck::Auto,
        ..Options::default()
    })
    .run(&source);
    output.settle_wasm(&WasmSupply::default(), true);
    output
}

fn wasm_notes(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "wasm")
        .map(|note| note.message.as_str())
        .collect()
}

fn assert_unnamed(out: &Output) {
    let code = out.code();
    assert_eq!(out.wasm[0].renamed, 0, "{code}");
    for kept in ["function Pa(", "function Ra()", "Yb(Ra())", "Z.g"] {
        assert!(code.contains(kept), "{kept} missing\n{code}");
    }
    for absent in ["_emscripten_", "wasmExports", "wasmImports", "_malloc("] {
        assert!(!code.contains(absent), "{absent} present\n{code}");
    }
}

#[test]
fn emscripten_library_imports_are_named_from_their_shape_and_signature_without_a_name_section() {
    let out = settled(GLUE, &module_with(covering_imports()));
    assert!(!out.wasm[0].summary.has_function_names());
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    let code = out.code();
    for named in [
        "function _emscripten_memcpy_big(e, t, n)",
        "function _emscripten_date_now()",
        "var _emscripten_resize_heap = (e) =>",
        "function Qa()",
        "a: _emscripten_memcpy_big,",
        "d: _emscripten_resize_heap",
        "_malloc(_emscripten_date_now());",
        "_emscripten_memcpy_big(1, 2, 3);",
        "wasmMemory = wasmExports.e;",
    ] {
        assert!(code.contains(named), "{named} missing\n{code}");
    }
    assert_eq!(out.wasm[0].renamed, 7);
    insta::assert_snapshot!(code);
}

#[test]
fn a_library_shape_whose_module_import_has_another_signature_names_nothing() {
    let mut imports = covering_imports();
    imports[0] = ("a", "a", sig(&[I32, I32], &[]));
    let out = settled(GLUE, &module_with(imports));
    assert_unnamed(&out);
    assert!(
        wasm_notes(&out).iter().any(|note| note
            .contains("1 glue function(s) whose module import has another signature, e.g. a.a (_emscripten_memcpy_big)")),
        "{:?}",
        wasm_notes(&out)
    );
}

#[test]
fn a_nameless_module_with_an_import_the_glue_does_not_provide_names_nothing() {
    let mut imports = covering_imports();
    imports.push(("a", "h", VOID));
    let out = settled(GLUE, &module_with(imports));
    assert_unnamed(&out);
    assert!(
        wasm_notes(&out)
            .iter()
            .any(|note| note.contains("1 module import(s) the glue does not provide, e.g. a.h")),
        "{:?}",
        wasm_notes(&out)
    );
}

#[test]
fn a_nameless_module_missing_an_import_the_glue_provides_names_nothing() {
    let mut imports = covering_imports();
    imports.pop();
    let out = settled(GLUE, &module_with(imports));
    assert_unnamed(&out);
    assert!(
        wasm_notes(&out)
            .iter()
            .any(|note| note.contains("glue name(s) missing from the module, e.g. import a.d")),
        "{:?}",
        wasm_notes(&out)
    );
}

#[test]
fn a_nameless_module_missing_an_export_the_glue_reads_names_nothing() {
    let out = settled(
        &GLUE.replace("Z.g)(e)", "Z.q)(e)"),
        &module_with(covering_imports()),
    );
    let code = out.code();
    assert_eq!(out.wasm[0].renamed, 0, "{code}");
    assert!(code.contains("function Pa("), "{code}");
    assert!(
        wasm_notes(&out)
            .iter()
            .any(|note| note.contains("export q")),
        "{:?}",
        wasm_notes(&out)
    );
}

#[test]
fn a_function_that_only_resembles_a_library_shape_keeps_its_name() {
    let glue = GLUE
        .replace("copyWithin(e,t,t+n)", "copyWithin(t,e,t+n)")
        .replace("return Date.now()", "return Date.now()+1");
    let out = settled(&glue, &module_with(covering_imports()));
    assert!(wasm_notes(&out).is_empty(), "{:?}", wasm_notes(&out));
    let code = out.code();
    assert!(code.contains("function Pa("), "{code}");
    assert!(code.contains("function Ra()"), "{code}");
    assert!(code.contains("_emscripten_resize_heap"), "{code}");
    assert_eq!(out.wasm[0].renamed, 5);
}

#[test]
fn a_shadowed_date_is_not_the_global_clock() {
    let glue = GLUE.replace(
        "function Ra(){return Date.now()}",
        "var Date={now:()=>0};function Ra(){return Date.now()}",
    );
    let out = settled(&glue, &module_with(covering_imports()));
    let code = out.code();
    assert!(code.contains("function Ra()"), "{code}");
    assert!(!code.contains("_emscripten_date_now"), "{code}");
}
