mod common;

use common::bin;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist/tests/wasm")
        .join(name)
}

#[test]
fn an_embedded_wasm_payload_is_written_as_its_own_file_and_reported() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("out");
    let out = bin()
        .args(["--report", "json", "-o"])
        .arg(&out_dir)
        .arg(fixture("bindgen.js"))
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let written = std::fs::read(out_dir.join("wasm/deobfuscated.wasm")).unwrap();
    assert_eq!(written, std::fs::read(fixture("bindgen.wasm")).unwrap());
    let code = std::fs::read_to_string(out_dir.join("deobfuscated.js")).unwrap();
    assert!(code.contains("B(\"./wasm/deobfuscated.wasm\")"));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["wasm"][0]["path"], "wasm/deobfuscated.wasm");
    assert_eq!(json["wasm"][0]["origin"], "embedded");
    assert_eq!(json["wasm"][0]["renamed"], 3);
}

#[test]
fn an_unresolved_remote_wasm_is_named_on_stderr_and_in_the_report() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(
        &input,
        "WebAssembly.instantiateStreaming(fetch('pkg/app_bg.wasm'));",
    )
    .unwrap();
    let out = bin()
        .args(["--report", "json"])
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("app_bg.wasm was not resolved"), "{stderr}");
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["wasm_unresolved"][0]["name"], "app_bg.wasm");
    assert_eq!(json["wasm_unresolved"][0]["url"], "pkg/app_bg.wasm");
}
