use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const BUN_PREMISE: &str =
    "a Bun @bun-cjs file is read as Bun runs it; node only evaluates the wrapper expression";

const PREMISES: [(&str, &str); 1] = [("unpack-a37rr3/bun1.js --unsafe", BUN_PREMISE)];

const SPLITS: [&str; 6] = [
    "unpack-a37rr3/n15.js",
    "unpack-a37rr3/n15.js --unsafe",
    "unpack-a37rr3/n18.js --unsafe",
    "unpack-a37rr3/n21.js --unsafe",
    "unpack-a37rr3/e11.js --unsafe",
    "unpack-a37rr3/e12.js --unsafe",
];

const REFUSALS: [(&str, &str); 14] = [
    ("unpack-a37rr3/b1.js", "differ in strictness"),
    ("unpack-a37rr3/n6.js", "differ in strictness"),
    ("unpack-a37rr3/n14.js", "differ in strictness"),
    ("unpack-a37rr3/n1.js", "it reads this"),
    ("unpack-a37rr3/n10.js", "it reads this"),
    ("unpack-a37rr3/n11.js", "it reads arguments"),
    ("unpack-a37rr3/n13.js", "it reads this"),
    (
        "unpack-a37rr3/n4.js",
        "the bundle sits inside a top-level statement",
    ),
    (
        "unpack-a37rr3/n5.js",
        "the bundle sits inside a top-level statement",
    ),
    (
        "unpack-a37rr3/n16.js",
        "the bundle sits inside a top-level statement",
    ),
    (
        "unpack-a37rr3/n20.js --unsafe",
        "the bundle sits inside a top-level statement",
    ),
    (
        "unpack-a37rr3/n22.js --unsafe",
        "the bundle sits inside a top-level statement",
    ),
    ("unpack-a37rr3/mg5.js", "would be written with import.meta"),
    (
        "unpack-a37rr3/mg2b.js",
        "would be written with a top-level await",
    ),
];

const SLOPPY_TOP_LEVEL: [&str; 3] = [
    "unpack-a37rr3/e11.js --unsafe",
    "unpack-a37rr3/e12.js --unsafe",
    "unpack-a37rr3/bun1.js --unsafe",
];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn probes() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures().join("unpack-a37rr3"))
        .expect("probes")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext == "js" || ext == "mjs")
        })
        .collect();
    found.sort();
    found
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn lanes(path: &Path) -> Vec<(String, Options)> {
    let hoisted = file_name(path).starts_with("rh");
    let options = |unsafe_transforms: bool, split_hoisted: bool| Options {
        unsafe_transforms,
        split_hoisted,
        input_names: vec![file_name(path)],
        ..Options::default()
    };
    let mut lanes = vec![
        (String::new(), options(false, false)),
        ("--unsafe".to_string(), options(true, false)),
    ];
    if hoisted {
        lanes.push(("--split-hoisted".to_string(), options(false, true)));
        lanes.push(("--unsafe --split-hoisted".to_string(), options(true, true)));
    }
    lanes
}

fn observed(node: &Path, file: &Path) -> (String, bool) {
    let observed = run(node, file).expect("spawn").expect("terminates");
    (observed.stdout, observed.exit == Some(0))
}

struct Written {
    entry: PathBuf,
    tree: bool,
    notes: String,
}

fn written(path: &Path, options: Options, dir: &Path) -> Written {
    let code = std::fs::read_to_string(path).expect("probe");
    let out = Pipeline::new(options).run(&code);
    let notes = format!("{:?}", out.notes);
    if path.extension().is_some_and(|ext| ext == "mjs") {
        let entry = dir.join(file_name(path));
        std::fs::write(&entry, out.code()).expect("write");
        return Written {
            entry,
            tree: false,
            notes,
        };
    }
    out.write_to(dir).expect("write");
    let tree = dir.join("bundle.json").exists();
    let entry = match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    };
    Written { entry, tree, notes }
}

fn checked_row(row: &str, written: &Written, wrong: &mut Vec<String>) {
    if SPLITS.contains(&row) && !written.tree {
        wrong.push(format!("{row}: expected a written tree: {}", written.notes));
    }
    if let Some((_, why)) = REFUSALS.iter().find(|(at, _)| *at == row) {
        if written.tree || !written.notes.contains(why) {
            wrong.push(format!(
                "{row}: expected the refusal {why:?}: {}",
                written.notes
            ));
        }
    }
    if SLOPPY_TOP_LEVEL.contains(&row) {
        let entry = std::fs::read_to_string(&written.entry).unwrap_or_default();
        if !written.tree || entry.trim_start().starts_with("\"use strict\"") {
            wrong.push(format!(
                "{row}: the sloppy entry was written strict or not at all"
            ));
        }
    }
}

#[test]
fn every_a37rr3_probe_prints_and_exits_the_same_after_untwist_in_both_lanes() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let mut wrong = Vec::new();
    let mut rows = 0;
    for path in probes() {
        let expected = observed(&node, &path);
        let name = format!("unpack-a37rr3/{}", file_name(&path));
        for (flags, options) in lanes(&path) {
            let row = format!("{name} {flags}").trim().to_string();
            let dir = tempfile::tempdir().expect("temp dir");
            let written = written(&path, options, dir.path());
            let got = observed(&node, &written.entry);
            checked_row(&row, &written, &mut wrong);
            rows += 1;
            let premise = PREMISES.iter().any(|(at, _)| *at == row);
            match (got == expected, premise) {
                (true, true) => wrong.push(format!("{row}: now agrees, drop it from PREMISES")),
                (false, false) => wrong.push(format!(
                    "{row}: node gave {expected:?}, untwist's output gave {got:?}"
                )),
                _ => {}
            }
        }
    }
    assert!(rows >= 238, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
