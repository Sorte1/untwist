use untwist_wasm::report::{run_report_with_wasm, WasmFiles};

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist/tests/wasm")
        .join(name)
}

fn wasm_json(report: &untwist_wasm::report::RunReport) -> serde_json::Value {
    serde_json::from_str(&report.wasm_json()).unwrap()
}

#[test]
fn an_embedded_module_comes_back_as_a_downloadable_file() {
    let source = std::fs::read_to_string(fixture("bindgen.js")).unwrap();
    let report = run_report_with_wasm(&source, "", false, "", &WasmFiles::new());
    let json = wasm_json(&report);
    assert_eq!(json["files"][0]["path"], "wasm/deobfuscated.wasm");
    assert_eq!(json["files"][0]["renamed"], 3);
    assert_eq!(
        report.wasm_bytes(0).unwrap(),
        std::fs::read(fixture("bindgen.wasm")).unwrap()
    );
    assert!(report.stdout().contains("\"./wasm/deobfuscated.wasm\""));
}

#[test]
fn a_dropped_module_is_matched_to_the_reference_by_name() {
    let source = std::fs::read_to_string(fixture("bindgen-remote.js")).unwrap();
    let unsupplied = run_report_with_wasm(&source, "", false, "", &WasmFiles::new());
    assert_eq!(
        wasm_json(&unsupplied)["unresolved"][0]["url"],
        "pkg/app_bg.3f2a91c0.wasm"
    );
    let mut files = WasmFiles::new();
    files.add(
        "app_bg.wasm".to_string(),
        std::fs::read(fixture("bindgen.wasm")).unwrap(),
        None,
    );
    let supplied = run_report_with_wasm(&source, "", false, "", &files);
    let json = wasm_json(&supplied);
    assert_eq!(json["files"][0]["origin"], "supplied");
    assert_eq!(json["unresolved"].as_array().unwrap().len(), 0);
    assert!(supplied
        .stdout()
        .contains("function __wbg_log_9a1b2c(e, t)"));
}
