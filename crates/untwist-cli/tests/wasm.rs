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
        .args(["--report", "json", "--wasm-check", "auto", "-o"])
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

fn report_of(out: &std::process::Output) -> serde_json::Value {
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn wasm_dir_with(dir: &std::path::Path, files: &[(&str, &str)]) -> std::path::PathBuf {
    let wasm_dir = dir.join("supplied");
    std::fs::create_dir_all(&wasm_dir).unwrap();
    for (name, fixture_name) in files {
        std::fs::copy(fixture(fixture_name), wasm_dir.join(name)).unwrap();
    }
    wasm_dir
}

#[test]
fn a_wasm_dir_file_matches_a_hashed_name_and_names_the_glue() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_dir = wasm_dir_with(dir.path(), &[("app_bg.wasm", "bindgen.wasm")]);
    let out = bin()
        .args(["--report", "json", "--wasm-dir"])
        .arg(&wasm_dir)
        .arg(fixture("bindgen-remote.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    assert_eq!(json["wasm"][0]["origin"], "supplied");
    assert_eq!(json["wasm"][0]["renamed"], 3);
    assert!(json.get("wasm_unresolved").is_none());
    assert!(json["code"]
        .as_str()
        .unwrap()
        .contains("function __wbg_log_9a1b2c(e, t)"));
}

#[test]
fn a_wasm_dir_file_matches_webpacks_hashed_module_name_exactly() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_dir = wasm_dir_with(
        dir.path(),
        &[("3f2a91c0d1e2b3a4.module.wasm", "webpack.wasm")],
    );
    let out_dir = dir.path().join("out");
    let out = bin()
        .args(["--report", "json", "--wasm-check", "auto", "--wasm-dir"])
        .arg(&wasm_dir)
        .arg("-o")
        .arg(&out_dir)
        .arg(fixture("webpack.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    assert!(json.get("wasm_unresolved").is_none(), "{json}");
    let paths: Vec<&str> = json["wasm"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        paths,
        ["wasm/index.wasm"],
        "the supplied file has the embedded bytes"
    );
}

#[test]
fn a_supplied_file_that_does_not_match_the_glue_warns_and_renames_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_dir = wasm_dir_with(dir.path(), &[("app_bg.wasm", "emscripten.wasm")]);
    let out = bin()
        .args(["--report", "json", "--wasm-dir"])
        .arg(&wasm_dir)
        .arg(fixture("bindgen-remote.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    assert_eq!(json["wasm"][0]["renamed"], 0);
    assert!(json["code"].as_str().unwrap().contains("function Q(e, t)"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning: wasm/app_bg.3f2a91c0.wasm does not match the glue"),
        "{stderr}"
    );
}

#[test]
fn a_wasm_override_wins_over_the_wasm_dir() {
    let dir = tempfile::tempdir().unwrap();
    let wasm_dir = wasm_dir_with(dir.path(), &[("app_bg.wasm", "emscripten.wasm")]);
    let out = bin()
        .args(["--report", "json", "--wasm-dir"])
        .arg(&wasm_dir)
        .arg("--wasm")
        .arg(format!(
            "app_bg.3f2a91c0.wasm={}",
            fixture("bindgen.wasm").display()
        ))
        .arg(fixture("bindgen-remote.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    assert_eq!(json["wasm"][0]["renamed"], 3);
}

struct Server {
    url: String,
    hits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

fn serve(body: Vec<u8>) -> Server {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/pkg/app_bg.3f2a91c0.wasm",
        listener.local_addr().unwrap()
    );
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counted = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/wasm\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    Server { url, hits }
}

fn absolute_glue(dir: &std::path::Path, url: &str) -> std::path::PathBuf {
    let glue = std::fs::read_to_string(fixture("bindgen-remote.js"))
        .unwrap()
        .replace("pkg/app_bg.3f2a91c0.wasm", url);
    let input = dir.join("glue.js");
    std::fs::write(&input, glue).unwrap();
    input
}

#[test]
fn nothing_is_fetched_without_the_flag() {
    let dir = tempfile::tempdir().unwrap();
    let server = serve(std::fs::read(fixture("bindgen.wasm")).unwrap());
    let input = absolute_glue(dir.path(), &server.url);
    let json = report_of(
        &bin()
            .args(["--report", "json"])
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(server.hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(json["wasm_unresolved"][0]["url"], server.url.as_str());
}

#[test]
fn fetch_wasm_downloads_an_absolute_url_and_names_the_glue() {
    let dir = tempfile::tempdir().unwrap();
    let server = serve(std::fs::read(fixture("bindgen.wasm")).unwrap());
    let input = absolute_glue(dir.path(), &server.url);
    let out = bin()
        .args(["--report", "json", "--fetch-wasm"])
        .arg(&input)
        .output()
        .unwrap();
    let json = report_of(&out);
    assert_eq!(server.hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(json["wasm"][0]["origin"], "fetched");
    assert_eq!(json["wasm"][0]["url"], server.url.as_str());
    assert_eq!(json["wasm"][0]["renamed"], 3);
}

#[test]
fn a_relative_url_is_not_fetched_without_a_base() {
    let out = bin()
        .args(["--report", "json", "--fetch-wasm"])
        .arg(fixture("bindgen-remote.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    assert_eq!(json["wasm_unresolved"][0]["name"], "app_bg.3f2a91c0.wasm");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("pass --wasm-base to fetch it"), "{stderr}");
}

#[test]
fn a_relative_url_is_fetched_against_the_wasm_base() {
    let dir = tempfile::tempdir().unwrap();
    let server = serve(std::fs::read(fixture("bindgen.wasm")).unwrap());
    let base = server.url.replace("pkg/app_bg.3f2a91c0.wasm", "index.html");
    let out = bin()
        .args(["--report", "json", "--fetch-wasm", "--wasm-base", &base])
        .arg(fixture("bindgen-remote.js"))
        .output()
        .unwrap();
    let json = report_of(&out);
    let _ = dir;
    assert_eq!(server.hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(json["wasm"][0]["url"], server.url.as_str());
}

#[test]
fn with_the_default_check_an_embedded_module_gets_a_hint_and_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("out");
    let out = bin()
        .arg("-o")
        .arg(&out_dir)
        .arg(fixture("bindgen.js"))
        .output()
        .unwrap();
    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("may hold embedded WebAssembly; pass --wasm-check auto or on"),
        "{stderr}"
    );
    assert!(!out_dir.join("wasm").exists());
}
