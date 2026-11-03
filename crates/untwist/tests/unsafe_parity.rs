use std::path::Path;

use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::js_files_in;
use untwist_testing::node::run as run_node;
use untwist_testing::node_rows::{node_or_skip, printed};

const REDUCED: &str = "fixtures/unsafe-parity";
const OBFUSCATED: &str = "fixtures/obfuscated";
const MACHINERY_KEPT: &str = "decoder machinery kept";

fn lane(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        ..Options::default()
    }
}

fn runnable(unsafe_transforms: bool) -> Options {
    Options {
        unpack: false,
        ..lane(unsafe_transforms)
    }
}

fn beside(label: &str, dir: &str, name: &str) -> String {
    let path = Path::new(label);
    let file = path
        .parent()
        .expect("fixture directory")
        .join(dir)
        .join(name);
    std::fs::read_to_string(&file).unwrap_or_else(|err| panic!("read {}: {err}", file.display()))
}

fn file_name(label: &str) -> &str {
    Path::new(label)
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture name")
}

fn refused(out: &Output) -> bool {
    out.notes.iter().any(|note| {
        note.message.starts_with(notes::BUILTIN_PATCHED) || note.message.contains(MACHINERY_KEPT)
    })
}

#[test]
fn the_unsafe_lane_decodes_every_reduced_fixture_and_removes_its_machinery() {
    let fixtures = js_files_in(REDUCED);
    assert!(!fixtures.is_empty(), "no fixtures under {REDUCED}");
    for (label, source) in fixtures {
        let out = Pipeline::new(lane(true)).run(&source);
        assert!(!refused(&out), "{label}: {:?}", out.notes);
        assert!(!out.code().contains("_0x"), "{label}:\n{}", out.code());
    }
}

#[test]
fn the_unsafe_output_of_a_reduced_fixture_prints_what_its_source_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in js_files_in(REDUCED) {
        let out = Pipeline::new(lane(true)).run(&source);
        let expected = printed(
            &node,
            dir.path(),
            &beside(&label, "sources", file_name(&label)),
        );
        let actual = printed(&node, dir.path(), out.code());
        assert!(
            expected.is_ok() && expected == actual,
            "{label}: the source printed {expected:?}, the output printed {actual:?}"
        );
    }
}

#[test]
fn the_default_lane_still_keeps_every_reduced_string_array() {
    for (label, source) in js_files_in(REDUCED) {
        let out = Pipeline::new(lane(false)).run(&source);
        assert!(refused(&out), "{label}: {:?}", out.notes);
        assert!(out.code().contains("_0x"), "{label}:\n{}", out.code());
    }
}

#[test]
fn the_unsafe_lane_decodes_every_obfuscated_fixture_the_default_lane_refuses() {
    let fixtures = js_files_in(OBFUSCATED);
    assert_eq!(fixtures.len(), 4, "{OBFUSCATED}");
    for (label, source) in fixtures {
        let pristine = Pipeline::new(runnable(true)).run(&source);
        assert!(!refused(&pristine), "{label}: {:?}", pristine.notes);
        let safe = Pipeline::new(runnable(false)).run(&source);
        assert!(refused(&safe), "{label}: {:?}", safe.notes);
    }
}

#[test]
fn the_unsafe_output_of_an_obfuscated_fixture_prints_what_its_original_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in js_files_in(OBFUSCATED) {
        let out = Pipeline::new(runnable(true)).run(&source);
        let program = dir.path().join(file_name(&label));
        std::fs::write(&program, out.code()).unwrap();
        let ran = run_node(&node, &program).unwrap().expect("node finished");
        let expected = beside(&label, "expected", &format!("{}.stdout", file_name(&label)));
        assert_eq!(ran.exit, Some(0), "{label}: {}", ran.stderr);
        assert_eq!(ran.stdout, expected, "{label}");
    }
}
