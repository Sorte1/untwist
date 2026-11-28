mod common;

use std::path::{Path, PathBuf};

use common::{stdin_output, SPLIT_STRINGS_BEHIND_A_DECODER, WEBPACK_4};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn validator() -> jsonschema::Validator {
    let path = repo().join("docs/report.schema.json");
    let text = std::fs::read_to_string(&path).expect("the report schema");
    let schema: serde_json::Value = serde_json::from_str(&text).expect("the schema is JSON");
    jsonschema::draft202012::new(&schema).expect("the schema compiles")
}

fn report(source: &str, args: &[&str]) -> serde_json::Value {
    let mut all = vec!["--report", "json"];
    all.extend_from_slice(args);
    let out = stdin_output(source, &all);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|error| {
        panic!(
            "{args:?}: {error}: {}",
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn assert_valid(report: &serde_json::Value, args: &[&str]) {
    let errors: Vec<String> = validator()
        .iter_errors(report)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{args:?}: {errors:#?}\n{report:#}");
    assert_eq!(report["schema_version"], 1, "{args:?}");
}

fn check(source: &str, args: &[&str]) -> serde_json::Value {
    let json = report(source, args);
    assert_valid(&json, args);
    json
}

#[test]
fn a_decoded_run_matches_the_report_schema() {
    let json = check(SPLIT_STRINGS_BEHIND_A_DECODER, &["--unsafe"]);
    assert!(json["code"].is_string(), "{json}");
    assert!(!json["stages"].as_array().expect("stages").is_empty());
}

#[test]
fn a_budgeted_run_with_types_matches_the_report_schema() {
    let json = check(
        "function add(a, b) { return a + b; }\nconsole.log(add(1, 2));\n",
        &["--types=ts", "--max-memory", "512MiB", "--timeout", "60"],
    );
    assert!(json["types"].is_object(), "{json}");
    assert!(json["budget"]["timeout"].is_number(), "{json}");
    assert!(json["budget"]["max_memory"].is_u64(), "{json}");
}

#[test]
fn a_run_with_parse_errors_matches_the_report_schema() {
    let json = check("function (", &[]);
    assert!(
        json["parse_errors"].as_u64().unwrap_or_default() > 0,
        "{json}"
    );
}

#[test]
fn an_unpacked_bundle_written_to_a_directory_matches_the_report_schema() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let out = dir.path().join("out");
    let out = out.display().to_string();
    let json = check(WEBPACK_4, &["--unsafe", "-o", &out, "--library-imports"]);
    assert!(json["bundle"]["modules"].is_array(), "{json}");
    assert!(json["output_path"].is_string(), "{json}");
    assert!(json.get("code").is_none(), "{json}");
}

#[test]
fn a_run_with_a_library_collection_matches_the_report_schema() {
    let collection = repo().join("fixtures/library-collection");
    let input =
        std::fs::read_to_string(collection.join("inputs/eventemitter3.rollup-cjs.terser.js"))
            .expect("the fixture input");
    let dir = collection.display().to_string();
    let json = check(&input, &["--unsafe", "--libraries", &dir]);
    assert!(json["collection"].is_object(), "{json}");
    assert!(json["libraries"].is_array(), "{json}");
}

#[test]
fn the_schema_names_the_version_the_cli_writes() {
    let text = std::fs::read_to_string(repo().join("docs/report.schema.json")).expect("schema");
    let schema: serde_json::Value = serde_json::from_str(&text).expect("JSON");
    assert_eq!(schema["properties"]["schema_version"]["const"], 1);
    let required = schema["required"].as_array().expect("required");
    assert!(required.iter().any(|name| name == "schema_version"));
}
