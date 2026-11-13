use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const LANES: [(&str, bool); 2] = [("safe", false), ("unsafe", true)];

struct Row {
    path: String,
    witness: String,
    lanes: [String; 2],
    sha256: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/obfuscators")
}

fn manifest() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("MANIFEST")).expect("read MANIFEST");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [path, _tool, _options, witness, safe, unsafe_lane, sha256] = fields[..] else {
                panic!("MANIFEST line has {} fields: {line}", fields.len());
            };
            Row {
                path: path.to_string(),
                witness: witness.to_string(),
                lanes: [safe.to_string(), unsafe_lane.to_string()],
                sha256: sha256.to_string(),
            }
        })
        .collect()
}

fn source(row: &Row) -> String {
    std::fs::read_to_string(root().join(&row.path))
        .unwrap_or_else(|err| panic!("{}: {err}", row.path))
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        ..Options::default()
    }
}

fn transformed(unsafe_transforms: bool, text: &str) -> String {
    Pipeline::new(options(unsafe_transforms))
        .run(text)
        .code()
        .to_string()
}

fn outcome(witness: &str, code: &str) -> &'static str {
    if witness == "-" {
        return "-";
    }
    let held = match witness.strip_prefix('!') {
        Some(gone) => !code.contains(gone),
        None => code.contains(witness),
    };
    if held {
        "undone"
    } else {
        "kept"
    }
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn listed_files(dir: &Path, base: &Path, out: &mut Vec<String>) {
    for path in std::fs::read_dir(dir)
        .expect("read fixtures")
        .flatten()
        .map(|e| e.path())
    {
        if path.is_dir() {
            listed_files(&path, base, out);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path.strip_prefix(base).unwrap().display().to_string());
        }
    }
}

#[test]
fn the_manifest_lists_every_fixture_with_its_hash() {
    let rows = manifest();
    let mut found = Vec::new();
    listed_files(&root(), &root(), &mut found);
    found.sort();
    let listed: Vec<String> = rows.iter().map(|row| row.path.clone()).collect();
    assert_eq!(listed, found, "MANIFEST and fixtures/obfuscators disagree");
    for row in &rows {
        assert_eq!(
            hex(source(row).as_bytes()),
            row.sha256,
            "{}: the hash moved",
            row.path
        );
    }
}

#[test]
fn each_obfuscator_fixture_is_undone_or_kept_as_the_manifest_says() {
    let mut wrong = Vec::new();
    for row in manifest() {
        let text = source(&row);
        for (at, (lane, unsafe_transforms)) in LANES.iter().enumerate() {
            let got = outcome(&row.witness, &transformed(*unsafe_transforms, &text));
            if got != row.lanes[at] {
                wrong.push(format!(
                    "{} ({lane}): {got}, MANIFEST {}",
                    row.path, row.lanes[at]
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn each_obfuscator_fixture_runs_like_node_after_untwist_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = manifest()
        .iter()
        .map(|row| (row.path.clone(), source(row)))
        .collect();
    let run_safe = |text: &str| transformed(false, text);
    let run_unsafe = |text: &str| transformed(true, text);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
