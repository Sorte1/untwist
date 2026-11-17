mod common;

use std::path::{Path, PathBuf};

use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const DEFAULT: &[&str] = &[];
const UNSAFE: &[&str] = &["--unsafe"];
const LANES: [&[&str]; 2] = [DEFAULT, UNSAFE];
const PREMISE_ROWS: [(&str, &[&str]); 16] = [
    ("a23", UNSAFE),
    ("r5bs3", UNSAFE),
    ("r5ev1", UNSAFE),
    ("r5ev2", UNSAFE),
    ("r5ev6", UNSAFE),
    ("r5ev7", UNSAFE),
    ("r5ev15", UNSAFE),
    ("r5pr9", UNSAFE),
    ("r5pr10", UNSAFE),
    ("r5im2", UNSAFE),
    ("r5pr14", UNSAFE),
    ("r5bfyev", UNSAFE),
    ("r5esbfn", UNSAFE),
    ("r5esbfns", UNSAFE),
    ("r5esbargv", UNSAFE),
    ("r5esbargvs", UNSAFE),
];
const RUNNER_PREMISE_ROWS: [&str; 61] = [
    "r6bsc", "r6c1", "r6c2", "r6c3", "r6c4", "r6c5", "r6c6", "r6c7", "r6d1", "r6d2", "r6d3",
    "r6d4", "r6d5", "r6d6", "r6d7", "r6d8", "r6d9", "r6e1", "r6e10", "r6e11", "r6e12", "r6e5",
    "r6e6", "r6escs", "r6g1", "r6g10", "r6g2", "r6g3", "r6g4", "r6g5", "r6g6", "r6g7", "r6g8",
    "r6g9", "r6k1", "r6k2", "r6k3", "r6k5", "r6k6", "r6k7", "r6k8", "r6k9", "r6rdc", "r6t1",
    "r6t2", "r6t3", "r6t4", "r6t5", "r7hv1", "r7l1", "r7l2", "r7n1", "r7n2", "r7n3", "r7rb1",
    "r7rb2", "r7rd1", "r7rd2", "r7rd3", "r7sp1", "r7sp2",
];
const DISPATCH_PREMISE_ROWS: [&str; 7] = ["e1", "e2", "r4n10", "r5pr1", "r5pr2", "r5pr4", "r5pr11"];
const PROTOTYPE_PREMISE_ROWS: [&str; 9] = ["a27", "b1", "b2", "b3", "b4", "b5", "b6", "b7", "b8"];
const OWN_NAME_ROWS: [&str; 4] = ["r5im2", "r5pr16", "r6e6", "r6e11"];
const PROTOTYPE_PREMISE_TREE: &str = "Some(0)\ntwo\n";
const INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/browserify_review");
const HOST_INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_names_review");
const MIRRORED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/browserify_review_mirrored"
);
const ROUTE_INPUTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_routes_review");
const ROUTE_MIRRORED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_routes_mirrored");
const ROUTE_FLAGS: [(&str, &[&str]); 1] = [("r4r6", &["--split-hoisted"])];
const ROUTE_UNPACKED: [&str; 6] = ["r4es0", "r4es2", "r4es3", "r4n5", "r4n6", "r4n7"];
const ROUTE_BUNDLE: &str = "app/bundle.js";
const ROUTE_LINKED: &str = "app/node_modules/foo/index.js";
const ROUTE_BESIDE: &str = "z.js";
const LEFT_REQUEST_REACHES: &str = "which node would resolve to a file of the written tree";
const SPLIT_ESM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/unpack-rolldown/rolldown1.3-esm-split"
);
const SPLIT_ENTRY: &str = "index.js";
const SPLIT_CHUNK: &str = "lazy.chunk.js";
const LAZY_IMPORT: &str = "import(\"./lazy.chunk.js\")";
const MIRRORED_BUNDLE: &str = "ma.js";
const MIRRORED_INPUT: &str = "lib/node_modules/foo/index.js";
const MODULE_EXTENSION: &str = "mjs";
const UNPACKED: [&str; 9] = ["a19c", "a26", "a36", "b10", "c10", "c5", "w13", "w16", "w7"];

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
    let module = input
        .extension()
        .is_some_and(|extension| extension == MODULE_EXTENSION);
    match module {
        true => out.join(format!("deobfuscated.{MODULE_EXTENSION}")),
        false => out.join("deobfuscated.js"),
    }
}

fn extra_flags(name: &str) -> &'static [&'static str] {
    ROUTE_FLAGS
        .iter()
        .find(|(row, _)| *row == name)
        .map_or(&[], |(_, flags)| flags)
}

fn tree_output(node: &Path, input: &Path, out: &Path, flags: &[&str]) -> (Option<String>, String) {
    let name = input.file_stem().expect("stem").to_string_lossy();
    let status = common::bin()
        .args(flags)
        .args(extra_flags(&name))
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
    let runner = flags == UNSAFE && RUNNER_PREMISE_ROWS.contains(&name);
    runner
        || PREMISE_ROWS
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
            if OWN_NAME_ROWS.contains(&name.as_ref()) {
                if *flags == DEFAULT && unpacked {
                    wrong.push(format!(
                        "{name} {flags:?}: split a module that reads its own name"
                    ));
                }
                continue;
            }
            if DISPATCH_PREMISE_ROWS.contains(&name.as_ref()) {
                if !kept_whole && (!unpacked || seen == expected) {
                    wrong.push(format!("{name} {flags:?}: premise row gave {seen:?}"));
                }
                continue;
            }
            let must_unpack =
                UNPACKED.contains(&name.as_ref()) || ROUTE_UNPACKED.contains(&name.as_ref());
            if must_unpack && !unpacked && !kept_whole {
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

#[test]
fn host_route_probes_run_like_their_bundles_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let inputs = inputs(ROUTE_INPUTS);
    assert!(inputs.len() >= 164, "found {} inputs", inputs.len());
    let wrong = parity_failures(&node, &inputs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_package_beside_the_bundle_input_refuses_the_split_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = Path::new(ROUTE_MIRRORED);
    let expected = observed(&node, &dir.join(ROUTE_BUNDLE));
    let work = tempfile::tempdir().expect("tempdir");
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("mm1-{lane}"));
        let status = common::bin()
            .current_dir(dir)
            .args(*flags)
            .args(["-v", "-f", "-o"])
            .arg(&out)
            .args([ROUTE_BUNDLE, ROUTE_LINKED, ROUTE_BESIDE])
            .output()
            .expect("run untwist");
        assert!(status.status.success(), "{flags:?}: untwist failed");
        let stderr = String::from_utf8_lossy(&status.stderr);
        let kept_whole = *flags == DEFAULT && common::frame_refused(&stderr);
        assert!(
            kept_whole || stderr.contains(LEFT_REQUEST_REACHES),
            "{flags:?}: {stderr}"
        );
        if out.join("bundle.json").exists() {
            assert_eq!(
                observed(&node, &out.join("index.js")),
                expected,
                "{flags:?}"
            );
        }
    }
}

fn entry_of(out: &Path) -> PathBuf {
    let text = std::fs::read_to_string(out.join("bundle.json")).expect("bundle listing");
    let listing: serde_json::Value = serde_json::from_str(&text).expect("listing json");
    let entry = listing["modules"]
        .as_array()
        .and_then(|modules| modules.iter().find(|module| module["isEntry"] == true))
        .and_then(|module| module["path"].as_str())
        .expect("an entry module");
    out.join(entry)
}

#[test]
fn a_relative_import_is_kept_only_under_unsafe_when_it_names_the_same_mirrored_input() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = Path::new(SPLIT_ESM);
    let expected = observed(&node, &dir.join(SPLIT_ENTRY));
    let work = tempfile::tempdir().expect("tempdir");
    for (lane, flags) in LANES.iter().enumerate() {
        let joined = work.path().join(format!("joined-{lane}"));
        let status = common::bin()
            .current_dir(dir)
            .args(*flags)
            .args(["-v", "-f", "-o"])
            .arg(&joined)
            .args([SPLIT_ENTRY, SPLIT_CHUNK])
            .output()
            .expect("run untwist");
        assert!(status.status.success(), "{flags:?}: untwist failed");
        let split = joined.join("bundle.json").exists();
        assert_eq!(split, *flags == UNSAFE, "{flags:?}: split joined");
        if split {
            assert_eq!(observed(&node, &entry_of(&joined)), expected, "{flags:?}");
        }
        let alone = work.path().join(format!("alone-{lane}"));
        let status = common::bin()
            .current_dir(dir)
            .args(*flags)
            .args(["-v", "-f", "-o"])
            .arg(&alone)
            .arg(SPLIT_ENTRY)
            .output()
            .expect("run untwist");
        let stderr = String::from_utf8_lossy(&status.stderr);
        assert!(
            !alone.join("bundle.json").exists(),
            "{flags:?}: split alone"
        );
        assert!(stderr.contains(LAZY_IMPORT), "{flags:?}: {stderr}");
    }
}
