use std::borrow::Cow;

use untwist::{printable, BundleOutput, ModuleOutput, Options, Pipeline};

#[test]
fn plain_text_is_returned_unchanged_without_a_copy() {
    assert!(matches!(
        printable("module ./a.js ünïcode"),
        Cow::Borrowed(_)
    ));
}

#[test]
fn invisible_and_direction_changing_characters_are_escaped() {
    assert_eq!(
        printable("a\u{200b}b\u{202e}c\u{2066}d\u{feff}e\u{61c}f"),
        "a\\u{200b}b\\u{202e}c\\u{2066}d\\u{feff}e\\u{61c}f"
    );
}

#[test]
fn every_control_and_line_separator_character_is_escaped() {
    let raw = "a\u{0}b\u{7}c\u{1b}[31md\te\nf\rg\u{7f}h\u{85}i\u{9b}j\u{2028}k\u{2029}l";
    assert_eq!(
        printable(raw),
        "a\\u{0}b\\u{7}c\\u{1b}[31md\\u{9}e\\u{a}f\\u{d}g\\u{7f}h\\u{85}i\\u{9b}j\\u{2028}k\\u{2029}l"
    );
}

fn bundle_with_path(path: &str) -> BundleOutput {
    BundleOutput {
        kind: "webpack".to_string(),
        entry_id: Some("./index".to_string()),
        chunk_ids: Vec::new(),
        mirrored_inputs: Vec::new(),
        modules: vec![ModuleOutput {
            id: path.to_string(),
            path: path.to_string(),
            is_entry: false,
            code: "module.exports = 4;".to_string(),
            span: None,
            map: None,
        }],
    }
}

#[test]
fn a_line_break_in_a_module_path_cannot_end_the_single_file_banner() {
    for separator in ["\n", "\r", "\u{2028}", "\u{2029}"] {
        let mut out = Pipeline::new(Options::default()).run("x;");
        out.bundle = Some(bundle_with_path(&format!(
            "a{separator}process.exit(9);//.js"
        )));
        let text = out.concatenated().into_owned();
        let banner = text
            .lines()
            .find(|line| line.starts_with("// ===="))
            .unwrap();
        assert!(banner.ends_with("//.js ===="), "{text:?}");
        assert!(!text.contains(separator) || separator == "\n", "{text:?}");
        assert!(
            !text.lines().any(|line| line.starts_with("process.exit")),
            "{text:?}"
        );
    }
}

#[test]
fn an_escape_character_in_a_module_path_is_not_written_to_the_single_file_output() {
    let mut out = Pipeline::new(Options::default()).run("x;");
    out.bundle = Some(bundle_with_path("./i\u{1b}[31mX.js"));
    let text = out.concatenated().into_owned();
    assert!(!text.contains('\u{1b}'), "{text:?}");
    assert!(text.contains("// ==== ./i\\u{1b}[31mX.js ===="), "{text:?}");
}
