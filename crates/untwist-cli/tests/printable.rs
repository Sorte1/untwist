mod common;

use common::{bin, stdin_output};

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn assert_printable(text: &str) {
    let raw: Vec<char> = text
        .chars()
        .filter(|ch| *ch != '\n' && (ch.is_control() || *ch == '\u{2028}' || *ch == '\u{2029}'))
        .collect();
    assert!(raw.is_empty(), "unescaped {raw:?} in {text:?}");
}

const HOSTILE_IDS: &str = "(function(modules){var cache={};function __webpack_require__(id){\
    if(cache[id])return cache[id].exports;var module=cache[id]={exports:{}};\
    modules[id].call(module.exports,module,module.exports,__webpack_require__);\
    return module.exports;}return __webpack_require__(\"./i\\u001b[31mX\");})\
    ({\"./i\\u001b[31mX\":function(module,exports,__webpack_require__){\
    var a=__webpack_require__(\"./a\\nprocess.exit(9);//\");console.log(a);},\
    \"./a\\nprocess.exit(9);//\":function(module,exports){module.exports=4;}});";

#[test]
fn an_escape_character_in_the_entry_id_is_printed_escaped() {
    let out = stdin_output(HOSTILE_IDS, &["-v"]);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("entry ./i\\u{1b}[31mX)"), "{stderr}");
    assert_printable(&stderr);
}

#[test]
fn hostile_module_ids_reach_neither_stderr_nor_the_single_file_output_raw() {
    let out = stdin_output(HOSTILE_IDS, &["-v", "--single-file"]);
    assert_printable(&stderr_of(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains('\u{1b}'), "{stdout}");
    assert!(
        !stdout.lines().any(|line| line.starts_with("process.exit")),
        "{stdout}"
    );
}

#[test]
fn hostile_module_ids_in_refusals_and_notes_of_an_output_tree_are_printed_escaped() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.js");
    std::fs::write(&input, HOSTILE_IDS).unwrap();
    let out = bin()
        .arg("-v")
        .arg("-o")
        .arg(dir.path().join("out"))
        .arg(&input)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", stderr_of(&out));
    assert_printable(&stderr_of(&out));
}

const HOSTILE_WASM_URL: &str =
    "WebAssembly.instantiateStreaming(fetch('\\u001b]52;c;aGk=\\u0007/x\\u001b[2K.wasm'));";

#[test]
fn a_wasm_url_with_terminal_controls_is_printed_escaped() {
    let out = stdin_output(HOSTILE_WASM_URL, &["-v"]);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("\\u{1b}]52;c;aGk=\\u{7}"), "{stderr}");
    assert_printable(&stderr);
}

#[test]
fn a_wasm_url_that_is_not_fetched_is_printed_escaped() {
    let out = stdin_output(HOSTILE_WASM_URL, &["--fetch-wasm"]);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("not fetching"), "{stderr}");
    assert_printable(&stderr);
}

#[test]
fn a_parse_diagnostic_quoting_a_control_character_is_printed_escaped() {
    let out = stdin_output("var a = 1;\u{1b}[2J\n", &[]);
    let stderr = stderr_of(&out);
    assert!(stderr.contains("\\u{1b}"), "{stderr}");
    assert_printable(&stderr);
}

#[test]
fn an_error_naming_an_input_path_is_printed_escaped() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("in\u{1b}[31m.js");
    let out = bin().arg(&missing).output().unwrap();
    assert!(!out.status.success());
    let stderr = stderr_of(&out);
    assert!(stderr.contains("in\\u{1b}[31m.js"), "{stderr}");
    assert_printable(&stderr);
}
