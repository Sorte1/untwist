use std::path::{Path, PathBuf};

use untwist_wasm::report::run_report;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/library-collection")
}

fn read(path: &str) -> Vec<u8> {
    std::fs::read(fixture().join(path)).expect("a fixture file")
}

#[test]
fn the_page_loads_a_collection_asks_what_it_wants_and_passes_the_shards_in() {
    let index = String::from_utf8(read("collection.json")).expect("utf-8");
    let json: serde_json::Value = serde_json::from_str(&index).expect("json");
    let filters = json["filters"]["file"].as_str().expect("a filter file");
    assert!(untwist_wasm::load_library_collection("{", &[]).contains("does not parse"));
    assert_eq!(
        untwist_wasm::load_library_collection(&index, &read(filters)),
        ""
    );
    let source =
        String::from_utf8(read("inputs/eventemitter3.rollup-cjs.terser.js")).expect("utf-8");
    let first = run_report(&source, "--unsafe", false);
    let wanted: Vec<String> =
        serde_json::from_str(&untwist_wasm::library_wanted(&first.stdout())).expect("a json list");
    assert!(wanted.contains(&"eventemitter3".to_string()), "{wanted:?}");
    assert_eq!(untwist_wasm::load_library_shard(b"not a shard"), "");
    for package in json["packages"].as_array().expect("packages") {
        if wanted.iter().any(|id| package["id"] == id.as_str()) {
            let file = package["shards"][0]["file"].as_str().expect("a shard");
            assert_eq!(
                untwist_wasm::load_library_shard(&read(file)),
                package["id"].as_str().unwrap()
            );
        }
    }
    assert!(untwist_wasm::library_tables()
        .split(',')
        .any(|id| id == "eventemitter3"));
    let second = run_report(&source, "--unsafe", false);
    let report: serde_json::Value = serde_json::from_str(&second.report_json()).expect("json");
    let rows = report["libraries"].as_array().expect("libraries");
    assert!(
        rows.iter().any(|row| row["library"] == "eventemitter3"),
        "{rows:?}"
    );
}
