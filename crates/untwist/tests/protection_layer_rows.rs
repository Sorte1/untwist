use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{node_or_skip, printed};

const READ_ONLY_CONSOLE: &str = "Object.freeze(console);\n";

const LAYERED_CONSOLE_ROWS: &[&str] = &[
    "console-timing",
    "console-in-storage",
    "console-beside-lock",
];

const LOCK_ROWS: &[&str] = &["lock", "lock-string-array", "console-beside-lock"];

const REACHED_ARRAY_ROWS: &[&str] = &["nested-array-reached", "self-defending-array-reached"];

const KEPT_ARRAY_ROWS: &[&str] = &["self-defending-array-reached"];

const REACH_REASON: &str = "code that may hold the global object, or code from text, may reach";

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/protection_layers")
}

fn source(label: &str) -> String {
    std::fs::read_to_string(data().join(format!("{label}.js"))).unwrap()
}

fn default_lane(source: &str) -> (String, Vec<String>) {
    let result = Pipeline::new(Options::default()).run(source);
    let notes = result
        .notes
        .iter()
        .map(|note| note.message.clone())
        .collect();
    (result.code().to_string(), notes)
}

#[test]
fn the_default_lane_prints_what_the_input_prints_with_or_without_its_silencing() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    let rows = LAYERED_CONSOLE_ROWS
        .iter()
        .chain(LOCK_ROWS)
        .chain(REACHED_ARRAY_ROWS);
    for label in rows {
        let input = source(label);
        let silenced = printed(&node, dir.path(), &input);
        let heard = printed(&node, dir.path(), &format!("{READ_ONLY_CONSOLE}{input}"));
        let (code, _) = default_lane(&input);
        let after = printed(&node, dir.path(), &code);
        if after.is_err() || (after != silenced && after != heard) {
            wrong.push(format!(
                "{label}: node printed {silenced:?} (read-only console {heard:?}), the default lane's output printed {after:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_domain_lock_is_kept_with_a_note_that_names_its_premise() {
    for label in LOCK_ROWS {
        let (_, notes) = default_lane(&source(label));
        assert!(
            notes.iter().any(|note| note == notes::DOMAIN_LOCK_KEPT),
            "{label}: {notes:#?}"
        );
    }
}

#[test]
fn the_unsafe_lane_still_removes_the_domain_lock() {
    for label in LOCK_ROWS {
        let options = Options {
            unsafe_transforms: true,
            ..Options::default()
        };
        let code = Pipeline::new(options)
            .run(&source(label))
            .code()
            .to_string();
        assert!(!code.contains("for (const"), "{label}:\n{code}");
    }
}

#[test]
fn a_string_array_kept_after_the_verified_rerun_names_the_global_reach() {
    for label in KEPT_ARRAY_ROWS {
        let (_, notes) = default_lane(&source(label));
        let kept = notes
            .iter()
            .find(|note| note.starts_with("decoder machinery kept: "));
        assert!(
            kept.is_some_and(|note| note.contains(REACH_REASON)),
            "{label}: {notes:#?}"
        );
    }
}

#[test]
fn a_one_entry_outer_array_under_a_reached_inner_array_decodes() {
    let (code, notes) = default_lane(&source("nested-array-reached"));
    assert_eq!(code.trim(), "console.log(\"\");", "{notes:#?}");
}
