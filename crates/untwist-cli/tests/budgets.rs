mod common;

use common::{stdin_output, SPLIT_STRINGS_BEHIND_A_DECODER};

fn wide_array(elements: usize) -> String {
    let zeros = vec!["0"; elements].join(",");
    format!("var a = [{zeros}];\nconsole.log(a.length);\n")
}

#[test]
fn an_input_whose_parse_goes_past_the_memory_cap_is_passed_through_with_the_reason() {
    let source = wide_array(1 << 20);
    let out = stdin_output(&source, &["--max-memory", "48MiB"]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("budget-memory"), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), source.trim());
}

#[test]
fn the_reports_carry_the_budget_and_the_memory_peak() {
    let out = stdin_output(
        SPLIT_STRINGS_BEHIND_A_DECODER,
        &[
            "--report",
            "json",
            "--max-memory",
            "1GiB",
            "--timeout",
            "600",
        ],
    );
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["budget"]["max_memory"], 1u64 << 30);
    assert_eq!(v["budget"]["timeout"], 600.0);
    assert!(v["budget"]["memory_peak"].is_u64());
    assert_eq!(v["budget"]["hits"], serde_json::json!([]));
    let text = stdin_output(SPLIT_STRINGS_BEHIND_A_DECODER, &["--report", "text"]);
    let report = String::from_utf8_lossy(&text.stderr).into_owned();
    assert!(
        report.contains("budget: max-memory 4.0 GiB, timeout none, memory peak"),
        "{report}"
    );
}

#[test]
fn a_bad_budget_value_is_a_usage_error() {
    for args in [["--max-memory", "lots"], ["--timeout", "0"]] {
        let out = stdin_output("var a;", &args);
        assert_eq!(out.status.code(), Some(2));
    }
}

fn long_strings(count: usize, bytes: usize) -> String {
    (0..count)
        .map(|index| format!("var s{index} = \"{}\";\n", "x".repeat(bytes)))
        .collect()
}

fn read_slowly(child: &mut std::process::Child, pause: std::time::Duration) -> Vec<u8> {
    use std::io::Read;
    std::thread::sleep(pause);
    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_end(&mut stdout)
        .unwrap();
    stdout
}

#[test]
fn a_slow_reader_gets_the_whole_output_under_a_short_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("long.js");
    std::fs::write(&input, long_strings(8, 64 << 10)).unwrap();
    let quick = common::bin().arg(&input).output().unwrap();
    assert!(quick.status.success());
    let mut child = common::bin()
        .args(["--timeout", "1"])
        .arg(&input)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = read_slowly(&mut child, std::time::Duration::from_secs(4));
    let out = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert_eq!(stdout.len(), quick.stdout.len(), "{stderr}");
    assert!(stdout == quick.stdout);
}

struct Stalling {
    origin: String,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Stalling {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = std::thread::spawn(move || {
            let mut held = Vec::new();
            for stream in listener.incoming() {
                if stopped.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                held.extend(stream.ok());
            }
        });
        Self {
            origin,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Stalling {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let address = self.origin.trim_start_matches("http://");
        let _ = std::net::TcpStream::connect(address);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[test]
fn a_stalled_wasm_fetch_fails_and_the_code_is_still_written() {
    let server = Stalling::start();
    let source = format!(
        "WebAssembly.instantiateStreaming(fetch(\"{}/slow.wasm\"), {{}}).then(function (r) {{ console.log(r); }});\n",
        server.origin
    );
    let started = std::time::Instant::now();
    let out = common::bin()
        .env("UNTWIST_TEST_FETCH_REQUEST_MS", "4000")
        .args(["--timeout", "1", "--fetch-wasm", "--fetch-wasm-local"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(source.as_bytes())?;
            child.wait_with_output()
        })
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(stdout.contains("instantiateStreaming"), "{stdout}");
    assert!(stderr.contains("slow.wasm failed"), "{stderr}");
    assert!(started.elapsed() >= std::time::Duration::from_secs(4));
    drop(server);
}

#[test]
fn an_input_larger_than_the_memory_cap_is_passed_through_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("big.js");
    let source = long_strings(12, 1 << 20);
    std::fs::write(&input, &source).unwrap();
    let out = common::bin()
        .args(["--max-memory", "16MiB"])
        .arg(&input)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("was not processed"), "{stderr}");
    assert!(out.stdout == format!("{source}\n").into_bytes(), "{stderr}");
}

#[test]
fn a_large_input_does_not_take_a_small_one_down_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let small = dir.path().join("s.js");
    std::fs::write(&small, "var a = !0;\nconsole.log(a);\n").unwrap();
    let big = dir.path().join("big.js");
    let source = long_strings(12, 1 << 20);
    std::fs::write(&big, &source).unwrap();
    let written = dir.path().join("out");
    let out = common::bin()
        .args(["--max-memory", "8MiB"])
        .arg(&small)
        .arg(&big)
        .arg("-o")
        .arg(&written)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("input 2 was not processed"), "{stderr}");
    assert!(!stderr.contains("input 1 was not processed"), "{stderr}");
    let files: Vec<String> = std::fs::read_dir(&written)
        .unwrap()
        .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
        .collect();
    assert!(
        files.iter().any(|code| code.contains("var a = true;")),
        "{files:?}"
    );
    assert!(
        files.iter().any(|code| code.trim() == source.trim()),
        "{stderr}"
    );
}

#[test]
fn many_inputs_that_each_fit_the_memory_cap_all_go_through() {
    let dir = tempfile::tempdir().unwrap();
    let source = long_strings(4, 256 << 10);
    let inputs: Vec<std::path::PathBuf> = (0..40)
        .map(|index| {
            let path = dir.path().join(format!("in{index}.js"));
            std::fs::write(&path, &source).unwrap();
            path
        })
        .collect();
    let out = common::bin()
        .args(["-j", "1", "--max-memory", "24MiB"])
        .args(&inputs)
        .arg("-o")
        .arg(dir.path().join("out"))
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("not processed"), "{stderr}");
    let written = std::fs::read_dir(dir.path().join("out")).unwrap().count();
    assert!(written >= 40, "{written} files; {stderr}");
}

fn many_hoisted_segments(pairs: usize) -> String {
    let body: String = (0..pairs)
        .map(|k| format!("var a{k} = {k}; var b{k} = a{k} + 1;\n"))
        .collect();
    format!("(function () {{\n\"use strict\";\n{body}console.log(b0);\n}})();\n")
}

#[test]
fn a_budget_stop_while_unpacking_is_a_clean_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("hoisted.js");
    std::fs::write(&input, many_hoisted_segments(8000)).unwrap();
    for cap in ["12MiB", "20MiB"] {
        let out_dir = dir.path().join(format!("out-{cap}"));
        let out = common::bin()
            .arg(&input)
            .args(["--split-hoisted", "--no-deobfuscate", "--no-unminify"])
            .args(["--max-memory", cap, "-o"])
            .arg(&out_dir)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        assert_eq!(out.status.code(), Some(3), "{cap}: {stderr}");
        assert!(!stderr.contains("internal error"), "{cap}: {stderr}");
        assert!(stderr.contains("budget-memory"), "{cap}: {stderr}");
        assert!(!out_dir.join("bundle.json").exists(), "{cap}");
    }
}

#[test]
fn many_hoisted_segments_split_within_a_small_budget() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("hoisted.js");
    std::fs::write(&input, many_hoisted_segments(8000)).unwrap();
    let out_dir = dir.path().join("out");
    let out = common::bin()
        .arg(&input)
        .args(["--split-hoisted", "--no-deobfuscate", "--no-unminify"])
        .args(["--max-memory", "256MiB", "-o"])
        .arg(&out_dir)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{stderr}");
    assert!(out_dir.join("bundle.json").exists(), "{stderr}");
}
