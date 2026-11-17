mod common;

use std::path::PathBuf;

use common::scaling::assert_scales_linearly;

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node_rows::{self, node_or_skip};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/a33rr9");

const MODES: [(&str, Lane); 3] = [
    ("default", Lane::Default),
    ("unsafe", Lane::Unsafe),
    ("rename smart", Lane::Rename(RenameMode::Smart)),
];

const UNSAFE_PREMISE: [&str; 20] = [
    "800", "807", "808", "809", "813", "830", "832", "873", "875", "877", "840", "841", "878",
    "879", "880", "881", "882", "883", "884", "885",
];

const SWITCH_CASE_FUNCTIONS: [&str; 7] = ["800", "808", "809", "832", "873", "875", "877"];
const FLAG_IN_DEAD_ZONE: [&str; 4] = ["801", "802", "803", "872"];
const PATCHED_BUILT_INS: [&str; 10] = [
    "840", "841", "878", "879", "880", "881", "882", "883", "884", "885",
];

type Run = Box<dyn Fn(&str) -> String>;

#[derive(Clone, Copy)]
enum Lane {
    Default,
    Unsafe,
    Rename(RenameMode),
}

fn options(lane: Lane) -> Options {
    match lane {
        Lane::Default => Options::default(),
        Lane::Unsafe => Options {
            unsafe_transforms: true,
            ..Options::default()
        },
        Lane::Rename(rename) => Options {
            rename,
            ..Options::default()
        },
    }
}

fn transformed(lane: Lane, source: &str) -> String {
    Pipeline::new(options(lane)).run(source).code().to_string()
}

fn probe(label: &str) -> String {
    std::fs::read_to_string(PathBuf::from(DATA).join(format!("{label}.js"))).expect("read probe")
}

fn probes() -> Vec<(String, String)> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(DATA)
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
        .into_iter()
        .map(|path| {
            let label = path.file_stem().unwrap().to_string_lossy().to_string();
            (label, std::fs::read_to_string(&path).expect("read probe"))
        })
        .collect()
}

#[test]
fn every_a33_rr9_probe_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes();
    assert!(programs.len() >= 27, "found {} probes", programs.len());
    let lanes: Vec<(&'static str, Run)> = MODES
        .iter()
        .map(|&(name, lane)| {
            let run: Run = Box::new(move |source| transformed(lane, source));
            (name, run)
        })
        .collect();
    let modes: Vec<node_rows::Mode<'_>> = lanes
        .iter()
        .map(|(name, run)| (*name, run.as_ref()))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let mut found: Vec<String> = wrong
        .iter()
        .map(|row| {
            row.split_once(':')
                .map_or(row.as_str(), |(head, _)| head)
                .to_string()
        })
        .collect();
    found.sort();
    let mut expected: Vec<String> = UNSAFE_PREMISE
        .iter()
        .map(|label| format!("{label} (unsafe)"))
        .collect();
    expected.sort();
    assert_eq!(found, expected, "{}", wrong.join("\n"));
}

#[test]
fn a_function_declared_in_a_switch_case_keeps_its_registration_by_default() {
    for label in SWITCH_CASE_FUNCTIONS {
        let code = transformed(Lane::Default, &probe(label));
        assert!(code.contains("apply"), "{label}: {code}");
    }
}

#[test]
fn a_let_flag_declared_after_the_returner_keeps_the_construct_in_every_lane() {
    for label in FLAG_IN_DEAD_ZONE {
        for (name, lane) in MODES {
            let code = transformed(lane, &probe(label));
            assert!(code.contains("apply"), "{label} ({name}): {code}");
        }
    }
}

#[test]
fn a_patched_built_in_keeps_the_construct_by_default_and_not_under_unsafe() {
    for label in PATCHED_BUILT_INS {
        let source = probe(label);
        let kept = transformed(Lane::Default, &source);
        assert!(kept.contains("apply"), "{label}: {kept}");
        let removed = transformed(Lane::Unsafe, &source);
        assert!(!removed.contains("apply"), "{label} (unsafe): {removed}");
    }
}

#[test]
fn a_flag_declared_before_the_returner_is_still_removed() {
    for (name, lane) in MODES {
        let code = transformed(lane, &probe("820b"));
        assert!(!code.contains("apply"), "820b ({name}): {code}");
    }
}

const SHELLS: usize = 20_000;

fn construct_with_empty_shells(shells: usize) -> String {
    probe("820b").replacen(
        "console.log('after');",
        &format!(
            "{}console.log('after');",
            "(function () {})();".repeat(shells)
        ),
        1,
    )
}

#[test]
fn removing_many_empty_shells_scales_linearly() {
    assert_scales_linearly(
        "empty shells beside a removed construct",
        SHELLS,
        construct_with_empty_shells,
        Options::default,
    );
}
