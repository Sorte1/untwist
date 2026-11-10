use untwist::{Options, Pipeline};
use untwist_wasm::stderr_of;

#[test]
fn a_parse_diagnostic_quoting_a_control_character_is_escaped_in_the_stderr_text() {
    let result = Pipeline::new(Options::default()).run("var a = 1;\u{1b}[2J\n");
    let stderr = stderr_of(&result);
    assert!(stderr.contains("\\u{1b}"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "{stderr}");
}
