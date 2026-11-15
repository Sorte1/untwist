mod common;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/naming_review");

const LANES: [&[&str]; 6] = [
    &[],
    &["--rename", "smart"],
    &["--rename", "mangle"],
    &["--rename", "simple"],
    &["--rename", "unique"],
    &["--unsafe"],
];

const BUNDLE_PREMISE: [&str; 1] = ["s10 (--unsafe)"];

const AMD_PREMISE: [&str; 8] = [
    "a4 (--rename smart)",
    "a4 (--rename mangle)",
    "a4 (--rename simple)",
    "a4 (--rename unique)",
    "a6 (--rename smart)",
    "a6 (--rename mangle)",
    "a6 (--rename simple)",
    "a6 (--rename unique)",
];

const CALLER_PREMISE: [&str; 5] = [
    "--rename smart",
    "--rename mangle",
    "--rename simple",
    "--rename unique",
    "--unsafe",
];

fn data(name: &str) -> PathBuf {
    Path::new(DATA).join(name)
}

fn untwist(args: &[OsString]) {
    let output = common::bin().args(args).output().expect("run untwist");
    assert!(
        output.status.success(),
        "untwist {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn printed(node: &Path, args: &[&OsStr], file: &Path) -> String {
    run_with(node, args, file)
        .expect("run node")
        .map_or("did not finish".to_string(), |out| {
            format!("{:?}\n{}", out.exit, out.stdout)
        })
}

fn inputs(dir: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(data(dir))
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    found.sort();
    found
}

fn written(input: &Path, out: &Path, flags: &[&str]) -> Option<PathBuf> {
    let mut args: Vec<OsString> = flags.iter().map(OsString::from).collect();
    args.extend([
        OsString::from("-f"),
        OsString::from("-o"),
        out.as_os_str().to_owned(),
        input.as_os_str().to_owned(),
    ]);
    untwist(&args);
    let entry = out.join("index.js");
    entry.exists().then_some(entry)
}

fn label(input: &Path) -> String {
    input.file_stem().unwrap().to_string_lossy().to_string()
}

fn lane_name(flags: &[&str]) -> String {
    match flags.is_empty() {
        true => "default".to_string(),
        false => flags.join(" "),
    }
}

fn mismatches(rows: Vec<(String, String, String)>) -> Vec<String> {
    rows.into_iter()
        .filter(|(_, expected, observed)| expected != observed)
        .map(|(row, _, _)| row)
        .collect()
}

#[test]
fn unpacked_trees_keep_every_name_and_bound_require_the_bundle_gives() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let probes = inputs("rr5");
    assert!(probes.len() >= 25, "found {} probes", probes.len());
    let mut rows = Vec::new();
    for (index, input) in probes.iter().enumerate() {
        let expected = printed(&node, &[], input);
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("{index}-{lane}"));
            let entry = written(input, &out, flags).unwrap_or(out.join("deobfuscated.js"));
            let observed = printed(&node, &[], &entry);
            rows.push((
                format!("{} ({})", label(input), lane_name(flags)),
                expected.clone(),
                observed,
            ));
        }
    }
    let wrong = mismatches(rows);
    assert_eq!(wrong, BUNDLE_PREMISE);
}

#[test]
fn an_amd_factory_keeps_the_names_its_define_gives() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let amd = data("rr5/amd-runner.cjs");
    let cjs = data("rr5/cjs-runner.cjs");
    let probes = inputs("rr5/amd");
    assert!(probes.len() >= 6, "found {} probes", probes.len());
    let result = |printed: String| {
        printed
            .lines()
            .filter(|line| line.starts_with("result "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let mut rows = Vec::new();
    for (index, input) in probes.iter().enumerate() {
        let expected = result(printed(&node, &[amd.as_os_str()], input));
        assert!(!expected.is_empty(), "{}: no result line", label(input));
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("amd-{index}-{lane}"));
            let observed = match written(input, &out, flags) {
                Some(entry) => printed(&node, &[cjs.as_os_str()], &entry),
                None => printed(&node, &[amd.as_os_str()], &out.join("deobfuscated.js")),
            };
            rows.push((
                format!("{} ({})", label(input), lane_name(flags)),
                expected.clone(),
                result(observed),
            ));
        }
    }
    let wrong = mismatches(rows);
    assert_eq!(wrong, AMD_PREMISE);
}

#[test]
fn a_caller_read_in_another_classic_script_keeps_the_wrapper_name() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let inputs = [data("caller-b.js"), data("caller-a.js")];
    let read = |paths: &[PathBuf]| -> Vec<String> {
        paths
            .iter()
            .map(|path| std::fs::read_to_string(path).expect("read"))
            .collect()
    };
    let joined = work.path().join("joined-in.js");
    std::fs::write(&joined, read(&inputs).join("\n")).expect("write joined input");
    let expected = printed(&node, &[], &joined);
    assert!(expected.contains('r'), "{expected}");
    let mut rows = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("caller-{lane}"));
        let mut args: Vec<OsString> = flags.iter().map(OsString::from).collect();
        args.extend([
            OsString::from("-f"),
            OsString::from("-o"),
            out.as_os_str().to_owned(),
        ]);
        args.extend(inputs.iter().map(|path| path.as_os_str().to_owned()));
        untwist(&args);
        let outputs = [
            out.join("deobfuscated.caller-b.js"),
            out.join("deobfuscated.caller-a.js"),
        ];
        let joined_out = work.path().join(format!("joined-out-{lane}.js"));
        std::fs::write(&joined_out, read(&outputs).join("\n")).expect("write joined output");
        rows.push((
            lane_name(flags),
            expected.clone(),
            printed(&node, &[], &joined_out),
        ));
    }
    assert_eq!(mismatches(rows), CALLER_PREMISE);
}
