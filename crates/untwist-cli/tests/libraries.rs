mod common;

use std::path::{Path, PathBuf};

use common::stdin_output;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/library-collection")
}

fn input() -> String {
    std::fs::read_to_string(fixture().join("inputs/eventemitter3.rollup-cjs.terser.js"))
        .expect("the fixture input")
}

fn report(args: &[&str]) -> (serde_json::Value, String) {
    let mut all = vec!["--unsafe", "--report", "json"];
    all.extend_from_slice(args);
    let out = stdin_output(&input(), &all);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        serde_json::from_slice(&out.stdout).expect("a json report"),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn libraries(json: &serde_json::Value) -> Vec<String> {
    json["libraries"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row["library"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_local_collection_names_its_package_and_reports_what_it_loaded() {
    let dir = fixture().display().to_string();
    let (json, stderr) = report(&["--libraries", &dir]);
    assert!(stderr.is_empty(), "{stderr}");
    assert!(
        libraries(&json).contains(&"eventemitter3".to_string()),
        "{json}"
    );
    assert_eq!(json["collection"]["version"], "2026.11.0-test");
    let loaded = json["collection"]["shards_loaded"]
        .as_array()
        .expect("loaded shards");
    assert!(loaded.iter().any(|id| id == "eventemitter3"), "{loaded:?}");
    assert!(json["code"]
        .as_str()
        .unwrap_or_default()
        .contains("// eventemitter3 "));
}

#[test]
fn builtin_is_the_default_and_changes_nothing() {
    let plain = stdin_output(&input(), &["--unsafe"]);
    let builtin = stdin_output(&input(), &["--unsafe", "--libraries", "builtin"]);
    assert_eq!(plain.stdout, builtin.stdout);
    assert!(!String::from_utf8_lossy(&plain.stdout).contains("// eventemitter3 "));
    let (json, _) = report(&[]);
    assert!(json.get("collection").is_none(), "{json}");
}

#[test]
fn none_drops_the_built_in_tables() {
    let source = "function _classCallCheck(a, n) { if (!(a instanceof n)) throw new TypeError(\"Cannot call a class as a function\"); }\nfunction A() { _classCallCheck(this, A); }\nnew A();\n";
    let none = stdin_output(source, &["--report", "json", "--libraries", "none"]);
    let parse = |out: &std::process::Output| -> serde_json::Value {
        serde_json::from_slice(&out.stdout).expect("a json report")
    };
    assert!(libraries(&parse(&none)).is_empty());
    assert!(none.status.success());
}

#[test]
fn a_collection_for_another_shape_epoch_falls_back_with_a_warning() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let index = std::fs::read_to_string(fixture().join("collection.json")).expect("the index");
    let epoch = untwist::libraries::SHAPE_EPOCH;
    let moved = index.replacen(
        &format!("\"shape_epoch\": {epoch}"),
        &format!("\"shape_epoch\": {}", epoch + 1),
        1,
    );
    std::fs::write(dir.path().join("collection.json"), moved).expect("writing the index");
    let path = dir.path().display().to_string();
    let (json, stderr) = report(&["--libraries", &path]);
    assert!(stderr.contains("shape epoch"), "{stderr}");
    assert!(stderr.contains("using the built-in tables"), "{stderr}");
    assert!(!libraries(&json).contains(&"eventemitter3".to_string()));
}

#[test]
fn a_missing_shard_is_a_warning_not_a_failure() {
    let dir = tempfile::tempdir().expect("a temp dir");
    std::fs::copy(
        fixture().join("collection.json"),
        dir.path().join("collection.json"),
    )
    .expect("copying the index");
    let path = dir.path().display().to_string();
    let (json, stderr) = report(&["--libraries", &path]);
    assert!(stderr.contains("every package is loaded"), "{stderr}");
    assert!(stderr.contains("that package is skipped"), "{stderr}");
    assert!(!libraries(&json).contains(&"eventemitter3".to_string()));
}

#[test]
fn a_url_is_refused_since_the_cli_reads_only_local_collections() {
    let out = stdin_output("a;", &["--libraries", "https://untwist.dev/libraries/"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("local path"));
}

fn core_js_table() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../untwist-libraries-corejs/data/core-js.table.gz")
}

fn curated_core_js(dir: &Path) {
    use untwist_rename::library::collection::{sha256_hex, Index, Package, Shard, SCHEMA};
    let bytes = std::fs::read(core_js_table()).expect("the core-js table");
    let sha256 = sha256_hex(&bytes);
    let file = format!("shards/{sha256}.utl");
    std::fs::create_dir_all(dir.join("shards")).expect("the shard dir");
    std::fs::write(dir.join(&file), &bytes).expect("writing the shard");
    let index = Index {
        schema: SCHEMA.to_string(),
        version: "2026.11.0-curated".to_string(),
        shape_epoch: untwist::libraries::SHAPE_EPOCH,
        untwist: String::new(),
        packages: vec![Package {
            id: "core-js".to_string(),
            label: "core-js".to_string(),
            versions: Vec::new(),
            shards: vec![Shard {
                file,
                bytes: bytes.len() as u64,
                sha256,
                versions: String::new(),
                rows: 0,
            }],
            keys: Vec::new(),
            curated: true,
        }],
        filters: None,
    };
    std::fs::write(
        dir.join("collection.json"),
        serde_json::to_string(&index).expect("the index"),
    )
    .expect("writing the index");
}

fn core_js_input() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/libraries/core-js/rollup-cjs.terser.js"),
    )
    .expect("the core-js fixture")
}

#[test]
fn a_curated_core_js_package_matches_like_the_built_in_table() {
    let dir = tempfile::tempdir().expect("a temp dir");
    curated_core_js(dir.path());
    let table = core_js_table().display().to_string();
    let collection = dir.path().display().to_string();
    for lane in [&[][..], &["--unsafe"][..]] {
        let run = |source: &str| {
            let mut args = lane.to_vec();
            args.extend(["--libraries", "none", "--libraries", source]);
            stdin_output(&core_js_input(), &args)
        };
        let built_in = run(&table);
        let curated = run(&collection);
        assert!(curated.status.success());
        assert!(String::from_utf8_lossy(&built_in.stdout).contains("// core-js 3."));
        assert_eq!(built_in.stdout, curated.stdout, "{lane:?}");
    }
}

#[test]
fn modern_without_core_js_fingerprints_stops_with_a_hint() {
    let out = stdin_output("a;", &["--modern", "chrome 120", "--libraries", "none"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("core-js fingerprints are not available"),
        "{stderr}"
    );
    let dir = tempfile::tempdir().expect("a temp dir");
    curated_core_js(dir.path());
    let collection = dir.path().display().to_string();
    let out = stdin_output(
        "a;",
        &[
            "--modern",
            "chrome 120",
            "--libraries",
            "none",
            "--libraries",
            &collection,
        ],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn library_imports_needs_an_output_directory() {
    let out = stdin_output("a;", &["--library-imports"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--library-imports needs --output"));
}
