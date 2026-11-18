use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_core::pass::NoteKind;
use untwist_testing::node_rows::{node_or_skip, printed};

const ROWS: &[(&str, &str)] = &[
    ("outside-binding", "greeting.js"),
    ("external-constant", "console-test"),
    ("stateful-unproven", "console-test"),
];

const MACHINERY_PASSES: &[&str] = &[
    "find-string-array",
    "find-decoders",
    "inline-decoder-wrappers",
    "self-defending",
    "debug-protection",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn layered(label: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/layer_fixpoint")
        .join(format!("{label}.js"));
    std::fs::read_to_string(path).unwrap()
}

fn original(name: &str) -> String {
    match name {
        "console-test" => "console.log(\"test\");\n".to_string(),
        file => {
            std::fs::read_to_string(repo().join("tools/obfuscated/sources").join(file)).unwrap()
        }
    }
}

fn unsafe_run(source: &str) -> untwist::Output {
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

fn machinery_found(output: &untwist::Output) -> Vec<String> {
    output
        .notes
        .iter()
        .filter(|note| MACHINERY_PASSES.contains(&note.pass))
        .filter(|note| note.kind == NoteKind::Progress)
        .filter(|note| !note.message.contains(": none"))
        .map(|note| format!("{}: {}", note.pass, note.message))
        .collect()
}

#[test]
fn an_inner_layer_refused_under_the_outer_one_decodes_once_the_outer_layer_is_gone() {
    for (label, _) in ROWS {
        let code = unsafe_run(&layered(label)).code().to_string();
        let again = unsafe_run(&code);
        let left = machinery_found(&again);
        assert!(left.is_empty(), "{label}: {left:?}\n{code}");
        assert_eq!(
            again.code(),
            code,
            "{label}: a second run still changes the output"
        );
    }
}

#[test]
fn every_row_prints_what_its_source_prints_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in ROWS {
        let code = unsafe_run(&layered(label)).code().to_string();
        let expected = printed(&node, dir.path(), &original(source));
        let actual = printed(&node, dir.path(), &code);
        assert_eq!(actual, expected, "{label}\n{code}");
    }
}
