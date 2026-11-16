use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::{self, node_or_skip, observed};

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/a33rr6");

const MODES: [(&str, Lane); 6] = [
    ("default", Lane::Default),
    ("unsafe", Lane::Unsafe),
    ("rename smart", Lane::Rename(RenameMode::Smart)),
    ("rename mangle", Lane::Rename(RenameMode::Mangle)),
    ("rename simple", Lane::Rename(RenameMode::Simple)),
    ("rename unique", Lane::Rename(RenameMode::Unique)),
];

const CALLER_PREMISE: [&str; 4] = ["130", "131", "134", "136"];

const RENAME_PREMISE: [&str; 8] = ["40", "42", "44", "45", "46", "47", "48", "49"];

const UNSAFE_WITNESSES: [&str; 13] = [
    "101", "103", "108", "303", "306", "313", "40", "42", "44", "45", "46", "47", "49",
];

fn expected_mismatches() -> Vec<String> {
    let every_lane = CALLER_PREMISE.iter().flat_map(|label| {
        MODES
            .iter()
            .map(move |(mode, _)| format!("{label} ({mode})"))
    });
    let renamed = RENAME_PREMISE.iter().flat_map(|label| {
        MODES
            .iter()
            .filter(|(mode, _)| mode.starts_with("rename"))
            .map(move |(mode, _)| format!("{label} ({mode})"))
    });
    let unsafe_lane = UNSAFE_WITNESSES
        .iter()
        .map(|label| format!("{label} (unsafe)"));
    let mut expected: Vec<String> = every_lane.chain(renamed).chain(unsafe_lane).collect();
    expected.sort();
    expected.dedup();
    expected
}

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

fn probes(extension: &str) -> Vec<(String, String)> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(DATA)
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == extension))
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

fn module_printed(node: &Path, dir: &Path, text: &str) -> String {
    std::fs::write(dir.join("program.mjs"), text).unwrap();
    let runner = dir.join("run.mjs");
    std::fs::write(
        &runner,
        "const m = await import(\"./program.mjs\");\nconsole.log(\"x\", m.o);\n",
    )
    .unwrap();
    run(node, &runner)
        .unwrap()
        .map_or("did not finish".to_string(), |out| observed(&out))
}

fn labels(wrong: Vec<String>) -> Vec<String> {
    let mut found: Vec<String> = wrong
        .into_iter()
        .map(|row| {
            row.split_once(':')
                .map_or(row.clone(), |(head, _)| head.to_string())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn every_a33_rr6_script_probe_prints_what_node_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes("js");
    assert!(programs.len() >= 70, "found {} probes", programs.len());
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
    assert_eq!(labels(wrong), expected_mismatches());
}

#[test]
fn an_exported_flag_keeps_the_controller_in_every_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs = probes("mjs");
    assert_eq!(programs.len(), 3);
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in &programs {
        let before = module_printed(&node, dir.path(), source);
        assert!(before.starts_with("x false"), "{label}: {before}");
        for (name, lane) in MODES {
            let code = transformed(lane, source);
            assert!(code.contains("o = false"), "{label} ({name}): {code}");
        }
    }
}

#[test]
fn a_flag_that_is_not_a_private_local_keeps_the_controller() {
    for label in ["200", "201", "202", "204"] {
        let source = std::fs::read_to_string(Path::new(DATA).join(format!("{label}.js"))).unwrap();
        for (name, lane) in MODES {
            let code = transformed(lane, &source);
            assert!(code.contains("apply"), "{label} ({name}): {code}");
        }
    }
}
