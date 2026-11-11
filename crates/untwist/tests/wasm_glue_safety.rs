#[path = "common/wasm_module.rs"]
mod wasm_module;

use std::time::{Duration, Instant};

use untwist::{Options, Output, Pipeline, SuppliedWasm, WasmCheck, WasmOrigin, WasmSupply};
use wasm_module::{base64, sig, Module, I32};

fn app_wasm() -> Vec<u8> {
    Module {
        imports: vec![("env", "log_value", sig(&[I32], &[]))],
        functions: vec![("run_main", sig(&[], &[]))],
        memory: Some("memory"),
    }
    .bytes()
}

fn app_supply() -> WasmSupply {
    WasmSupply {
        files: vec![SuppliedWasm {
            name: "app.wasm".to_string(),
            bytes: app_wasm(),
            origin: WasmOrigin::Supplied,
            url: None,
        }],
    }
}

fn pipeline(wasm_check: WasmCheck) -> Pipeline {
    Pipeline::new(Options {
        wasm_check,
        ..Options::default()
    })
}

fn supplied(sources: &[&str]) -> Output {
    let mut out = pipeline(WasmCheck::Off).run_many(sources);
    out.settle_wasm(&app_supply(), true);
    out
}

fn extracted(source: &str) -> Output {
    let mut out = pipeline(WasmCheck::Auto).run(source);
    out.settle_wasm(&WasmSupply::default(), true);
    out
}

const SCRIPT_GLUE: &str = r#"var wasmPath = "app.wasm";
function lv() {}
var imports = { env: { log_value: lv } };
var ins = new WebAssembly.Instance(new WebAssembly.Module(require("fs").readFileSync(wasmPath)), imports);
var ex = ins.exports;
var mem = ex.memory;
var run = ex.run_main;
console.log("names:", lv.name, typeof globalThis.ex, typeof globalThis.mem, typeof globalThis.lv);"#;

const SECOND_SCRIPT: &str =
    r#"console.log("second script sees", typeof ex, typeof mem, typeof run);"#;

#[test]
fn script_globals_of_classic_glue_keep_their_names() {
    let out = supplied(&[SCRIPT_GLUE, SECOND_SCRIPT]);
    let code = &out.codes[0];
    for kept in ["function lv()", "var ex =", "var mem =", "var run ="] {
        assert!(code.contains(kept), "{kept} missing\n{code}");
    }
    for absent in [
        "wasmExports",
        "function log_value",
        "var memory",
        "var run_main",
    ] {
        assert!(!code.contains(absent), "{absent} present\n{code}");
    }
    assert!(out.codes[1].contains("typeof ex, typeof mem, typeof run"));
}

#[test]
fn glue_inside_a_function_of_a_script_is_still_named() {
    let source = format!("(function () {{\n{SCRIPT_GLUE}\n}})();");
    let out = supplied(&[&source]);
    let code = out.code();
    assert!(code.contains("wasmExports = ins.exports"), "{code}");
    assert!(code.contains("function log_value()"), "{code}");
}

#[test]
fn glue_of_an_es_module_is_still_named() {
    let source = format!("{SCRIPT_GLUE}\nexport {{ run }};");
    let out = supplied(&[&source]);
    let code = out.code();
    assert!(code.contains("var wasmExports = ins.exports"), "{code}");
    assert!(code.contains("function log_value()"), "{code}");
}

const ANNEX_B_GLUE: &str = r#"var wasmPath = "app.wasm";
var imports = { env: { log_value: function () {} } };
function g(inst) {
  var ex = inst.exports;
  var run = ex.run_main;
  { function run() { return "block function"; } }
  return typeof run === "function" && run.length === 0 ? String(run()) : "other";
}
console.log(g(new WebAssembly.Instance(new WebAssembly.Module(require("fs").readFileSync(wasmPath)), imports)));"#;

#[test]
fn a_var_shared_with_a_sloppy_block_function_keeps_its_name() {
    let out = supplied(&[ANNEX_B_GLUE]);
    let code = out.code();
    assert!(!code.contains("run_main ="), "{code}");
    assert!(code.contains("var run = "), "{code}");
    assert!(code.contains("String(run())"), "{code}");
    assert!(code.contains("function run()"), "{code}");
}

const BLOCK_FUNCTION_READ_OUTSIDE: &str = r#"(function () {
  var lv = function () { return "outer"; };
  function g() {
    var imports;
    try { function lv() { return "inner"; } imports = { env: { log_value: lv } }; } catch (e) {}
    console.log(lv());
    var ins = new WebAssembly.Instance(new WebAssembly.Module(require("fs").readFileSync("app.wasm")), imports);
    ins.exports.run_main();
  }
  g();
})();"#;

#[test]
fn a_sloppy_block_function_read_outside_its_block_keeps_its_name() {
    let out = supplied(&[BLOCK_FUNCTION_READ_OUTSIDE]);
    let code = out.code();
    assert!(code.contains("function lv()"), "{code}");
    assert!(code.contains("console.log(lv())"), "{code}");
    assert!(!code.contains("log_value()"), "{code}");
}

fn byte_list(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

#[test]
fn an_embedded_module_read_as_data_keeps_its_literal_and_is_still_written() {
    let wasm = app_wasm();
    let source = format!(
        "var bytes = new Uint8Array([{}]);\nconsole.log(bytes.length, bytes[1]);\nvar text = \"{}\";\nconsole.log(text.length, text.slice(0, 6));\nvar instance = new WebAssembly.Instance(new WebAssembly.Module(bytes), {{ env: {{ log_value: function () {{}} }} }});\ninstance.exports.run_main();",
        byte_list(&wasm),
        base64(&wasm)
    );
    let out = extracted(&source);
    let code = out.code();
    assert_eq!(out.wasm.len(), 1, "{code}");
    assert_eq!(out.wasm[0].bytes, wasm);
    assert!(!code.contains("./wasm/"), "{code}");
    assert!(code.contains(&base64(&wasm)), "{code}");
    assert!(code.contains(&format!("[{}]", byte_list(&wasm))), "{code}");
}

#[test]
fn a_tagged_template_holding_a_module_is_left_as_it_was() {
    let wasm = app_wasm();
    let source = format!(
        "var text = String.raw`{}`;\nconsole.log(text.length);",
        base64(&wasm)
    );
    let out = extracted(&source);
    let code = out.code();
    assert!(
        code.contains(&format!("String.raw`{}`", base64(&wasm))),
        "{code}"
    );
    assert_eq!(out.parse_errors, 0);
}

#[test]
fn a_data_uri_fetched_directly_is_replaced_by_the_written_file() {
    let wasm = app_wasm();
    let source = format!(
        "WebAssembly.instantiateStreaming(fetch(\"data:application/wasm;base64,{}\"), {{ env: {{ log_value: function () {{}} }} }});",
        base64(&wasm)
    );
    let out = extracted(&source);
    let code = out.code();
    assert_eq!(out.wasm.len(), 1, "{code}");
    assert!(
        code.contains("fetch(\"./wasm/deobfuscated.wasm\")"),
        "{code}"
    );
}

#[test]
fn a_data_uri_fetched_through_a_local_fetch_keeps_its_literal() {
    let wasm = app_wasm();
    let source = format!(
        "function fetch(url) {{ return url.length; }}\nconsole.log(fetch(\"data:application/wasm;base64,{}\"));",
        base64(&wasm)
    );
    let out = extracted(&source);
    let code = out.code();
    assert!(!code.contains("./wasm/"), "{code}");
}

#[test]
fn many_distinct_wasm_names_are_listed_in_linear_time() {
    let count = 40_000;
    let source: String = (0..count)
        .map(|index| format!("fetch(\"m{index}.wasm\");\n"))
        .collect();
    let mut out = Pipeline::new(Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    })
    .run(&source);
    let started = Instant::now();
    let references = out.wasm_references();
    out.settle_wasm(&WasmSupply::default(), false);
    let took = started.elapsed();
    assert_eq!(references.len(), count);
    assert_eq!(references[0].name, "m0.wasm");
    assert_eq!(references[count - 1].name, format!("m{}.wasm", count - 1));
    assert_eq!(out.wasm_unresolved.len(), count);
    assert!(took < Duration::from_secs(4), "{took:?}");
}
