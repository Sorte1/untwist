use untwist_wasm::report::run_report;

fn fixture(path: &str) -> String {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/libraries")
        .join(path);
    std::fs::read_to_string(full).unwrap()
}

fn libraries(source: &str) -> serde_json::Value {
    let report = run_report(source, "", false);
    let json: serde_json::Value = serde_json::from_str(&report.report_json()).unwrap();
    json["libraries"].clone()
}

#[test]
fn the_report_lists_a_library_whose_table_is_compiled_in() {
    let rows = libraries(&fixture("regenerator-runtime/rollup-cjs.js"));
    let rows = rows.as_array().expect("a libraries array");
    assert_eq!(rows[0]["library"], "regenerator-runtime", "{rows:?}");
    assert!(rows[0]["range"]
        .as_str()
        .is_some_and(|range| !range.is_empty()));
    assert!(rows[0]["modules"].as_u64().is_some_and(|count| count >= 1));
}

#[test]
fn a_program_without_library_code_has_no_libraries_key() {
    assert!(libraries("function add(a, b) { return a + b; }\nconsole.log(add(1, 2));\n").is_null());
}
