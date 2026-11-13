use untwist_wasm::report::run_report;

const CORE_JS: &[u8] = include_bytes!("../../untwist-libraries-corejs/data/core-js.table.gz");

fn fixture(path: &str) -> String {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/libraries")
        .join(path);
    std::fs::read_to_string(full).unwrap()
}

#[test]
fn a_table_loaded_at_run_time_names_its_library_in_the_report() {
    assert!(!untwist_wasm::load_library_table(b"not a table"));
    assert!(untwist_wasm::load_library_table(CORE_JS));
    assert!(!untwist_wasm::load_library_table(CORE_JS));
    assert!(untwist_wasm::library_tables()
        .split(',')
        .any(|id| id == "core-js"));
    let report = run_report(&fixture("core-js/rollup-cjs.terser.js"), "", false);
    let json: serde_json::Value = serde_json::from_str(&report.report_json()).unwrap();
    let rows = json["libraries"].as_array().expect("a libraries array");
    assert_eq!(rows[0]["library"], "core-js", "{rows:?}");
    assert_eq!(rows[0]["range"], "3.39");
    assert!(report.stdout().contains("// core-js 3.39: "));
}
