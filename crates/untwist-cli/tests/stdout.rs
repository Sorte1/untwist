mod common;

use std::process::{Output, Stdio};

use common::{bin, stdin_output, WEBPACK_4};

const FIRST_SOURCE: &str = "var a=!0;x(),y();";
const SECOND_SOURCE: &str = "var b=void 0;";
const FIRST_CODE: &str = "var a = true;\nx();\ny();";
const SECOND_CODE: &str = "var b = undefined;";

fn stdout_of(args: &[&str]) -> Vec<u8> {
    let out = bin().args(args).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

struct Inputs {
    _dir: tempfile::TempDir,
    first: String,
    second: String,
    empty: String,
    broken: String,
}

fn inputs() -> Inputs {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, source: &str| {
        let path = dir.path().join(name);
        std::fs::write(&path, source).unwrap();
        path.to_string_lossy().into_owned()
    };
    let first = write("a.js", FIRST_SOURCE);
    let second = write("b.js", SECOND_SOURCE);
    let empty = write("e.js", "");
    let broken = write("x.js", "var = ;");
    Inputs {
        _dir: dir,
        first,
        second,
        empty,
        broken,
    }
}

fn joined(codes: &[&str]) -> Vec<u8> {
    format!("{}\n", codes.join("\n\n")).into_bytes()
}

#[test]
fn one_input_prints_its_code_and_one_newline() {
    let files = inputs();
    assert_eq!(
        stdout_of(&["--no-unpack", &files.first]),
        joined(&[FIRST_CODE])
    );
    assert_eq!(stdout_of(&["--no-unpack", &files.empty]), b"\n");
}

#[test]
fn several_inputs_print_their_outputs_joined_by_a_blank_line() {
    let files = inputs();
    let alone_first = stdout_of(&["--no-unpack", &files.first]);
    let alone_second = stdout_of(&["--no-unpack", &files.second]);
    let both = stdout_of(&["--no-unpack", &files.first, &files.second]);
    let mut expected = alone_first
        .strip_suffix(b"\n")
        .unwrap_or(&alone_first)
        .to_vec();
    expected.extend_from_slice(b"\n\n");
    expected.extend_from_slice(&alone_second);
    assert_eq!(both, expected);
    assert_eq!(both, joined(&[FIRST_CODE, SECOND_CODE]));
    assert!(both.ends_with(b"\n") && !both.ends_with(b"\n\n"));
}

#[test]
fn empty_outputs_keep_their_separators_among_several_inputs() {
    let files = inputs();
    let around = stdout_of(&["--no-unpack", &files.empty, &files.first, &files.empty]);
    assert_eq!(around, joined(&["", FIRST_CODE, ""]));
    assert_eq!(around, b"\n\nvar a = true;\nx();\ny();\n\n\n");
    let only_empty = stdout_of(&["--no-unpack", &files.empty, &files.empty]);
    assert_eq!(only_empty, b"\n\n\n");
}

#[test]
fn a_single_file_run_of_one_input_prints_that_input_once() {
    let files = inputs();
    let plain = stdout_of(&["--no-unpack", &files.first]);
    let single = stdout_of(&["--no-unpack", "--single-file", &files.first]);
    assert_eq!(single, plain);
    let empty = stdout_of(&["--no-unpack", "--single-file", &files.empty]);
    assert_eq!(empty, b"\n");
}

#[test]
fn a_single_file_run_of_several_inputs_prints_each_under_its_banner() {
    let files = inputs();
    let both = stdout_of(&["--no-unpack", "--single-file", &files.first, &files.second]);
    let expected =
        format!("// ==== input 1 ====\n{FIRST_CODE}\n\n// ==== input 2 ====\n{SECOND_CODE}\n\n");
    assert_eq!(both, expected.into_bytes());
    let around = stdout_of(&[
        "--no-unpack",
        "--single-file",
        &files.empty,
        &files.first,
        &files.empty,
    ]);
    let expected = format!(
        "// ==== input 1 ====\n\n\n// ==== input 2 ====\n{FIRST_CODE}\n\n// ==== input 3 ====\n\n\n"
    );
    assert_eq!(around, expected.into_bytes());
}

#[test]
fn a_bundle_prints_the_same_bytes_plain_and_as_a_single_file() {
    let plain = stdin_output(WEBPACK_4, &[]);
    assert!(plain.status.success());
    assert_eq!(
        String::from_utf8_lossy(&plain.stdout),
        "(function (e) {\n  var a = {};\n  function n(t) {\n    if (a[t]) {\n      return a[t].exports;\n    }\n    const r = a[t] = { exports: {} };\n    e[t].call(r.exports, r, r.exports, n);\n    return r.exports;\n  }\n  return n(n.s = 0);\n})([function (e, t, i) {\n  const lib = i(1);\n  console.log(lib);\n}, function (e, t, i) {\n  e.exports = 1;\n}]);\n"
    );
    let single = stdin_output(WEBPACK_4, &["--single-file"]);
    assert!(single.status.success());
    assert_eq!(
        String::from_utf8_lossy(&single.stdout),
        "// untwist: every module of this bundle, concatenated for reading. require() calls still name the paths in bundle.json; this file is not runnable.\n\n// ==== index.js ====\nconst lib = require(\"./1.js\");\nconsole.log(lib);\n\n// ==== 1.js ====\nmodule.exports = 1;\n\n"
    );
}

fn run_into(stdout: impl Into<Stdio>, args: &[&str]) -> Output {
    bin()
        .args(args)
        .stdout(stdout)
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn assert_write_error(out: &Output, os_error: &str) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(stderr.starts_with("error: write to stdout: "), "{stderr}");
    assert!(stderr.contains(os_error), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}

fn stdout_modes(files: &Inputs) -> Vec<Vec<String>> {
    let modes: [&[&str]; 5] = [
        &["--no-unpack", &files.first],
        &["--no-unpack", &files.first, &files.second],
        &["--no-unpack", "--single-file", &files.first],
        &["--no-unpack", "--single-file", &files.first, &files.second],
        &["--no-unpack", "--report", "json", &files.first],
    ];
    modes
        .iter()
        .map(|args| args.iter().map(|arg| arg.to_string()).collect())
        .collect()
}

#[test]
fn a_closed_stdout_pipe_fails_with_a_write_error_in_every_mode() {
    let files = inputs();
    for args in stdout_modes(&files) {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run_into(writer, &args);
        assert_write_error(&out, "Broken pipe");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_stdout_fails_with_a_write_error_in_every_mode() {
    let files = inputs();
    for args in stdout_modes(&files) {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run_into(full, &args);
        assert_write_error(&out, "No space left on device");
    }
}

fn stderr_modes(files: &Inputs) -> Vec<Vec<String>> {
    let missing = format!("{}.missing", files.first);
    let modes: [&[&str]; 4] = [
        &["--no-unpack", "--report", "text", &files.first],
        &["--no-unpack", "--verbose", &files.first, &files.second],
        &["--no-unpack", &files.broken],
        &["--no-unpack", &missing],
    ];
    modes
        .iter()
        .map(|args| args.iter().map(|arg| arg.to_string()).collect())
        .collect()
}

fn run_with_stderr(stderr: impl Into<Stdio>, args: &[&str]) -> Output {
    bin()
        .args(args)
        .stdout(Stdio::null())
        .stderr(stderr)
        .output()
        .unwrap()
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_stderr_exits_one_in_every_mode() {
    let files = inputs();
    for args in stderr_modes(&files) {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .unwrap();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run_with_stderr(full, &args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
    }
}

#[test]
fn a_closed_stderr_pipe_exits_one_in_every_mode() {
    let files = inputs();
    for args in stderr_modes(&files) {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run_with_stderr(writer, &args);
        assert_eq!(out.status.code(), Some(1), "{args:?}");
    }
}
