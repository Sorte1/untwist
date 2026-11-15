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
const HOST_INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_names_review");
const MIRRORED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/browserify_review_mirrored"
);
const MIRRORED_BUNDLE: &str = "ma.js";
const MIRRORED_INPUT: &str = "lib/node_modules/foo/index.js";
const MODULE_EXTENSION: &str = "mjs";
const UNPACKED: [&str; 8] = ["a19c", "a26", "a36", "b10", "c10", "c5", "w12", "w7"];

fn inputs(dir: &str) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(dir)
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

fn tree_output(node: &Path, input: &Path, out: &Path, flags: &[&str]) -> (Option<String>, String) {
    let status = common::bin()
        .args(flags)
        .arg("-v")
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
    let stderr = String::from_utf8_lossy(&status.stderr).into_owned();
    (observed(node, &written_entry(input, out)), stderr)
}

fn premise(name: &str, flags: &[&str]) -> bool {
    PREMISE_ROWS
        .iter()
        .any(|(row, lane)| *row == name && *lane == flags)
}

fn parity_failures(node: &Path, inputs: &[PathBuf]) -> Vec<String> {
    let work = tempfile::tempdir().expect("tempdir");
    let mut wrong = Vec::new();
    for input in inputs {
        let name = input.file_stem().expect("stem").to_string_lossy();
        let expected = observed(node, input);
        for (lane, flags) in LANES.iter().enumerate() {
            if premise(&name, flags) {
                continue;
            }
            let out = work.path().join(format!("{name}-{lane}"));
            let (seen, stderr) = tree_output(node, input, &out, flags);
            let unpacked = out.join("bundle.json").exists();
            let kept_whole = *flags == DEFAULT && common::frame_refused(&stderr);
            if PROTOTYPE_PREMISE_ROWS.contains(&name.as_ref()) {
                if !kept_whole && (!unpacked || seen.as_deref() != Some(PROTOTYPE_PREMISE_TREE)) {
                    wrong.push(format!("{name} {flags:?}: premise row gave {seen:?}"));
                }
                continue;
            }
            if DISPATCH_PREMISE_ROWS.contains(&name.as_ref()) {
                if !kept_whole && (!unpacked || seen == expected) {
                    wrong.push(format!("{name} {flags:?}: premise row gave {seen:?}"));
                }
                continue;
            }
            if UNPACKED.contains(&name.as_ref()) && !unpacked && !kept_whole {
                wrong.push(format!("{name} {flags:?}: no bundle written"));
            }
            if seen != expected {
                wrong.push(format!(
                    "{name} {flags:?}: bundle {expected:?}, output {seen:?}"
                ));
            }
        }
    }
    wrong
}

#[test]
fn review_probes_run_like_the_browserify_bundle_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let inputs = inputs(INPUTS);
    assert!(inputs.len() >= 42, "found {} inputs", inputs.len());
    let wrong = parity_failures(&node, &inputs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn host_name_probes_run_like_their_bundles_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let inputs = inputs(HOST_INPUTS);
    assert!(inputs.len() >= 14, "found {} inputs", inputs.len());
    let wrong = parity_failures(&node, &inputs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_mirrored_input_does_not_capture_a_linked_package_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = Path::new(MIRRORED);
    let expected = observed(&node, &dir.join(MIRRORED_BUNDLE));
    let work = tempfile::tempdir().expect("tempdir");
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("m4-{lane}"));
        let status = common::bin()
            .current_dir(dir)
            .args(*flags)
            .args(["-v", "-f", "-o"])
            .arg(&out)
            .args([MIRRORED_BUNDLE, MIRRORED_INPUT])
            .output()
            .expect("run untwist");
        assert!(status.status.success(), "{flags:?}: untwist failed");
        let stderr = String::from_utf8_lossy(&status.stderr);
        let kept_whole = *flags == DEFAULT && common::frame_refused(&stderr);
        let unpacked = out.join("bundle.json").exists();
        assert!(unpacked || kept_whole, "{flags:?}: no bundle written");
        if !unpacked {
            continue;
        }
        let seen = observed(&node, &out.join("index.js"));
        assert_eq!(seen, expected, "{flags:?}");
    }
}
