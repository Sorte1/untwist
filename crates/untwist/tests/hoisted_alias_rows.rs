use std::path::Path;

use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_deobfuscate::notes;
use untwist_testing::node::run;
use untwist_testing::node_rows::{mismatches, node_or_skip, observed, Mode};

const SCRIPT: &str = "alias-of-hoisted-decoder-before-rotator.js";
const IMPORTING: &str = "module-alias-before-rotator-import-after.mjs";
const EXPORTING: &str = "module-alias-read-by-an-export.mjs";

type Runner = Box<dyn Fn(&str) -> String>;

#[derive(Clone, Copy, PartialEq)]
enum Lane {
    Safe,
    Conservative,
    Unsafe,
}

const LANES: [(&str, Lane); 3] = [
    ("safe", Lane::Safe),
    ("conservative", Lane::Conservative),
    ("unsafe", Lane::Unsafe),
];

const RENAMES: [(&str, RenameMode); 4] = [
    ("none", RenameMode::None),
    ("mangle", RenameMode::Mangle),
    ("simple", RenameMode::Simple),
    ("unique", RenameMode::Unique),
];

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/behaviour/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(path).unwrap()
}

fn options(lane: Lane, rename: RenameMode) -> Options {
    Options {
        extended: lane != Lane::Conservative,
        unsafe_transforms: lane != Lane::Safe,
        rename,
        unpack: false,
        jsx: false,
        ..Options::default()
    }
}

fn deobfuscated(source: &str, lane: Lane) -> Output {
    Pipeline::new(options(lane, RenameMode::None)).run(source)
}

fn decoder_calls(code: &str) -> usize {
    let bytes = code.as_bytes();
    code.match_indices(['b', 'd'])
        .filter(|(at, _)| {
            let before = at.checked_sub(1).and_then(|i| bytes.get(i));
            let identifier =
                before.is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_' || *c == b'$');
            !identifier
                && bytes.get(at + 1) == Some(&b'(')
                && bytes.get(at + 2).is_some_and(u8::is_ascii_digit)
        })
        .count()
}

fn has_note(out: &Output, note: &str) -> bool {
    out.notes.iter().any(|found| found.message.contains(note))
}

#[test]
fn an_alias_of_a_hoisted_decoder_before_the_rotator_is_decoded_in_every_lane() {
    let source = fixture(SCRIPT);
    for (label, lane) in LANES {
        let out = deobfuscated(&source, lane);
        let code = out.code();
        assert_eq!(decoder_calls(code), 0, "{label}:\n{code}");
        assert!(code.contains("\"there\""), "{label}:\n{code}");
        assert!(!code.contains("function a("), "{label}:\n{code}");
    }
}

#[test]
fn a_module_importing_after_the_rotator_is_decoded_only_under_unsafe() {
    let source = fixture(IMPORTING);
    for (label, lane) in LANES {
        let out = deobfuscated(&source, lane);
        let code = out.code();
        if lane == Lane::Unsafe {
            assert_eq!(decoder_calls(code), 0, "{label}:\n{code}");
            assert!(code.contains("\"there\""), "{label}:\n{code}");
        } else {
            assert!(decoder_calls(code) > 0, "{label}:\n{code}");
            assert!(
                has_note(&out, notes::CODE_BEFORE_ROTATOR),
                "{label}: {:#?}",
                out.notes
            );
        }
    }
}

#[test]
fn an_alias_an_export_may_reach_through_a_cycle_is_kept_in_the_safe_lanes() {
    let source = fixture(EXPORTING);
    for (label, lane) in LANES {
        let out = deobfuscated(&source, lane);
        let code = out.code();
        if lane == Lane::Unsafe {
            assert_eq!(decoder_calls(code), 0, "{label}:\n{code}");
        } else {
            assert!(code.contains("const d = b"), "{label}:\n{code}");
            assert!(
                has_note(&out, notes::WRAPPER_ALIAS_READ_BEFORE_THE_ALIAS),
                "{label}: {:#?}",
                out.notes
            );
        }
    }
}

#[test]
fn without_the_export_the_same_alias_is_inlined_in_the_safe_lane() {
    let source = fixture(IMPORTING);
    let out = deobfuscated(&source, Lane::Safe);
    assert!(!out.code().contains("const d = b"), "{}", out.code());
}

fn script_mutants() -> Vec<(String, String)> {
    let source = fixture(SCRIPT);
    vec![
        (
            "a call reads the array before the rotator".to_string(),
            format!("console.log(b(0x0));\n{source}"),
        ),
        (
            "the alias reads a lexical binding before its declaration".to_string(),
            source.replacen("const d = b;", "const d = w;\nlet w = b;", 1),
        ),
        (
            "the alias reads a function declared in a block".to_string(),
            source.replacen("const d = b;", "const d = q;\n{ function q() {} }", 1),
        ),
    ]
}

#[test]
fn the_mutants_keep_their_decoder_calls_in_the_safe_lanes() {
    for (label, source) in script_mutants() {
        for (lane_label, lane) in LANES.into_iter().filter(|(_, l)| *l != Lane::Unsafe) {
            let code = deobfuscated(&source, lane).code().to_string();
            assert!(decoder_calls(&code) > 0, "{label} ({lane_label}):\n{code}");
        }
    }
}

fn transforms() -> Vec<(&'static str, Options)> {
    LANES
        .into_iter()
        .flat_map(|(lane_label, lane)| {
            RENAMES.into_iter().map(move |(rename_label, rename)| {
                let label: &'static str =
                    Box::leak(format!("{lane_label}/{rename_label}").into_boxed_str());
                (label, options(lane, rename))
            })
        })
        .collect()
}

#[test]
fn the_script_fixture_and_its_mutants_print_like_node_in_every_lane_and_rename_mode() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs = vec![("alias before rotator".to_string(), fixture(SCRIPT))];
    programs.extend(script_mutants());
    let runs: Vec<(&'static str, Runner)> = transforms()
        .into_iter()
        .map(|(label, options)| {
            let run: Runner = Box::new(move |source| {
                Pipeline::new(options.clone())
                    .run(source)
                    .code()
                    .to_string()
            });
            (label, run)
        })
        .collect();
    let modes: Vec<Mode<'_>> = runs
        .iter()
        .map(|(label, run)| (*label, run.as_ref() as &dyn Fn(&str) -> String))
        .collect();
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
}

fn printed_as_module(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    let out = run(node, &path).unwrap().expect("finishes");
    observed(&out)
}

#[test]
fn the_module_fixtures_print_like_node_in_every_lane_and_rename_mode() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for name in [IMPORTING, EXPORTING] {
        let source = fixture(name);
        let before = printed_as_module(&node, dir.path(), &source);
        assert_eq!(before, "there hi! x 7\nexit 0\n", "{name}");
        for (label, options) in transforms() {
            let code = Pipeline::new(options).run(&source).code().to_string();
            let after = printed_as_module(&node, dir.path(), &code);
            assert_eq!(after, before, "{name} ({label}):\n{code}");
        }
    }
}
