use untwist::{Options, Pipeline};
use untwist_core::visible::is_hidden;
use untwist_testing::node::printed_by_node;

const HIDDEN: &[&str] = &[
    "\\x00", "\\x01", "\\x07", "\\b", "\\x0e", "\\x1b", "\\x1f", "\\x7f", "\\x80", "\\x9b",
    "\\x9c", "\\x9f", "\\u2028", "\\u2029", "\\u202a", "\\u202e", "\\u2066", "\\u2069", "\\r",
    "\\t", "\\v", "\\f",
];

fn forms(escape: &str) -> Vec<String> {
    let mut forms = vec![
        format!("\"a{escape}1\""),
        format!("`a${{\"{escape}\"}}1`"),
        format!("`a${{\"{escape}0\"}}${{x}}{escape}9`"),
        format!("`{escape}`"),
        format!("String.raw`{escape}`"),
        format!("Object.keys({{ \"{escape}k\": 1 }})[0]"),
        format!("\"b\" + \"{escape}\" + \"2\""),
        format!("/[{escape}]/.test(\"{escape}\")"),
    ];
    if escape != "\\b" {
        forms.push(format!("/{escape}+/.source"));
    }
    forms
}

fn program() -> String {
    let values: Vec<String> = HIDDEN.iter().flat_map(|escape| forms(escape)).collect();
    format!(
        "var x = 7;\nconsole.log(JSON.stringify([\n{}\n]));\n",
        values.join(",\n")
    )
}

#[test]
fn printed_code_holds_no_hidden_character_and_means_the_same() {
    let source = program();
    let output = Pipeline::new(Options::default()).run(&source);
    let code = output.code();
    let hidden: Vec<String> = code
        .lines()
        .filter(|line| line.chars().any(is_hidden))
        .map(|line| line.escape_default().to_string())
        .collect();
    assert!(hidden.is_empty(), "hidden characters printed: {hidden:#?}");
    let Some(expected) = printed_by_node(&source) else {
        return;
    };
    assert!(expected.len() > 100, "node printed {expected:?}");
    assert_eq!(printed_by_node(code), Some(expected), "{code}");
}

#[test]
fn the_review_probes_print_visible_code() {
    let probes = [
        "var t = `safe${\"\\x1b]0;owned\\x07\\x1b[1A\\x1b[2K\"}`;\nconsole.log(JSON.stringify(t));\n",
        "var c = \"\\u009b31mC1\\u202e\\u2028\";\nvar d = new RegExp(\"\\x1b\\\\[0m\");\nvar h = \"\\r\\n\\x00\\x7f\\x9c\";\nconsole.log(JSON.stringify([c, d.source, h]));\n",
    ];
    for source in probes {
        let output = Pipeline::new(Options::default()).run(source);
        let code = output.code();
        assert!(!code.chars().any(is_hidden), "{}", code.escape_default());
        if let Some(expected) = printed_by_node(source) {
            assert_eq!(printed_by_node(code), Some(expected), "{code}");
        }
    }
}
