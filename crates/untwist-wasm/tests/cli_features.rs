use untwist_wasm::report::run_report;

const CORE_JS: &[u8] = include_bytes!("../../untwist-libraries-corejs/data/core-js.table.gz");

fn fixture(path: &str) -> String {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(path);
    std::fs::read_to_string(full).unwrap()
}

fn stage_names(report: &str) -> Vec<String> {
    let json: serde_json::Value = serde_json::from_str(report).unwrap();
    json["stages"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["stage"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn modern_removes_core_js_polyfills_in_the_wasm_build() {
    untwist_wasm::load_library_table(CORE_JS);
    let source = fixture("modern/babel-usage.core-js-3.39.0.rollup-cjs.js");
    let plain = run_report(&source, "", false);
    let modern = run_report(&source, "--modern es2022", false);
    assert_eq!(modern.code(), 0, "{}", modern.stderr());
    assert!(modern.stdout().len() < plain.stdout().len());
    assert!(stage_names(&modern.report_json()).contains(&"modern".to_string()));
    assert!(!stage_names(&plain.report_json()).contains(&"modern".to_string()));
}

#[test]
fn a_bad_baseline_is_a_usage_error() {
    let out = run_report("a;", "--modern es1999", false);
    assert_eq!(out.code(), 2);
    assert!(
        out.stderr().starts_with("error: --modern: "),
        "{}",
        out.stderr()
    );
}

#[test]
fn the_assumed_module_split_infers_rollup_boundaries_in_the_wasm_build() {
    let source = fixture("unpack-rollup-boundaries/synthetic-rollup3-wrapped/index.js");
    let split = run_report(&source, "--split-hoisted --split-assume-modules", false);
    assert_eq!(split.code(), 0, "{}", split.stderr());
    assert_eq!(split.module_count(), 10);
    assert_eq!(run_report(&source, "", false).module_count(), 0);
}

#[test]
fn an_inline_source_map_ends_the_output() {
    let out = run_report("var a = !0;", "--no-unpack --source-map=inline", false);
    let last = out.stdout().lines().last().unwrap_or_default().to_string();
    assert!(
        last.starts_with("//# sourceMappingURL=data:application/json;"),
        "{last}"
    );
    let plain = run_report("var a = !0;", "--no-unpack", false);
    assert!(out.stdout().starts_with(plain.stdout().trim_end()));
    assert_eq!(
        run_report("a;", "--source-map=file", false).code(),
        2,
        "file maps need a folder"
    );
}

fn assumed_split(report: &str) -> bool {
    let json: serde_json::Value = serde_json::from_str(report).unwrap();
    json["detection"]["assumedSplit"].as_bool().unwrap_or(false)
}

#[test]
fn the_report_says_when_only_the_assumption_would_split_the_bundle() {
    let source = fixture("unpack-rollup-boundaries/synthetic-rollup3-wrapped/index.js");
    assert!(assumed_split(&run_report(&source, "", false).report_json()));
    assert!(assumed_split(
        &run_report(&source, "--split-hoisted", false).report_json()
    ));
    let split = run_report(&source, "--split-hoisted --split-assume-modules", false);
    assert!(!assumed_split(&split.report_json()));
    assert!(!assumed_split(
        &run_report(&source, "--no-unpack", false).report_json()
    ));
    assert!(!assumed_split(
        &run_report("var a = !0;", "", false).report_json()
    ));
}

#[test]
fn an_input_too_deep_to_parse_gets_no_split_hint_and_keeps_the_stack() {
    let depth = 100_000;
    let source = format!("var x = {}1{};", "[".repeat(depth), "]".repeat(depth));
    let report = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || run_report(&source, "", false).report_json())
        .unwrap()
        .join()
        .unwrap();
    assert!(!assumed_split(&report));
}
