mod common;

use std::path::{Path, PathBuf};

use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const DEFAULT: &[&str] = &[];
const UNSAFE: &[&str] = &["--unsafe"];
const LANES: [&[&str]; 2] = [DEFAULT, UNSAFE];
const PREMISE_ROWS: [(&str, &[&str]); 1] = [("a23", UNSAFE)];
const DISPATCH_PREMISE_ROWS: [&str; 2] = ["e1", "e2"];
const PROTOTYPE_PREMISE_ROWS: [&str; 9] = ["a27", "b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8"];
const PROTOTYPE_PREMISE_TREE: &str = "Some(0)\ntwo\n";
const INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/browserify_review");
const MODULE_EXTENSION: &str = "mjs";
const UNPACKED: [&str; 6] = ["a19c", "a26", "a36", "b10", "c10", "c5"];

fn inputs() -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(INPUTS)
        .expect("review inputs")
        .map(|entry| entry.expect("entry").path())
        .collect();
    inputs.sort();
    inputs
}

fn observed(node: &Path, file: &Path) -> Option<String> {
    run(node, file)
        .expect("run node")
        .map(|observed| format!("{:?}\n{}", observed.exit, observed.stdout))
}

fn written_entry(input: &Path, out: &Path) -> PathBuf {
    if out.join("bundle.json").exists() {
        return out.join("index.js");
    }
    let whole = out.join("deobfuscated.js");
    let module = input
        .extension()
        .is_some_and(|extension| extension == MODULE_EXTENSION);
    if !module {
        return whole;
    }
    let renamed = whole.with_extension(MODULE_EXTENSION);
    std::fs::rename(&whole, &renamed).expect("rename output");
    renamed
}

fn tree_output(node: &Path, input: &Path, out: &Path, flags: &[&str]) -> Option<String> {
    let status = common::bin()
        .args(flags)
        .arg("-f")
        .arg("-o")
        .arg(out)
        .arg(input)
        .output()
        .expect("run untwist");
    assert!(
        status.status.success(),
        "{}: untwist failed: {}",
        input.display(),
        String::from_utf8_lossy(&status.stderr)
    );
    observed(node, &written_entry(input, out))
}

fn premise(name: &str, flags: &[&str]) -> bool {
    PREMISE_ROWS
        .iter()
        .any(|(row, lane)| *row == name && *lane == flags)
}

#[test]
fn review_probes_run_like_the_browserify_bundle_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let inputs = inputs();
    assert!(inputs.len() >= 42, "found {} inputs", inputs.len());
    let work = tempfile::tempdir().expect("tempdir");
    let mut wrong = Vec::new();
    for input in &inputs {
        let name = input.file_stem().expect("stem").to_string_lossy();
        let expected = observed(&node, input);
        for (lane, flags) in LANES.iter().enumerate() {
            if premise(&name, flags) {
                continue;
            }
            let out = work.path().join(format!("{name}-{lane}"));
            let seen = tree_output(&node, input, &out, flags);
            let unpacked = out.join("bundle.json").exists();
            if PROTOTYPE_PREMISE_ROWS.contains(&name.as_ref()) {
                if !unpacked || seen.as_deref() != Some(PROTOTYPE_PREMISE_TREE) {
                    wrong.push(format!("{name} {flags:?}: premise row gave {seen:?}"));
                }
                continue;
            }
            if DISPATCH_PREMISE_ROWS.contains(&name.as_ref()) {
                if !unpacked || seen == expected {
                    wrong.push(format!("{name} {flags:?}: premise row gave {seen:?}"));
                }
                continue;
            }
            if UNPACKED.contains(&name.as_ref()) && !unpacked {
                wrong.push(format!("{name} {flags:?}: no bundle written"));
            }
            if seen != expected {
                wrong.push(format!(
                    "{name} {flags:?}: bundle {expected:?}, output {seen:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
