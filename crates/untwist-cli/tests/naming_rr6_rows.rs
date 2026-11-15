mod common;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/naming_review/rr6");

const LANES: [&[&str]; 6] = [
    &[],
    &["--rename", "smart"],
    &["--rename", "mangle"],
    &["--rename", "simple"],
    &["--rename", "unique"],
    &["--unsafe"],
];

const BUNDLE_PREMISE: [&str; 10] = [
    "b6 (default)",
    "b6 (--rename smart)",
    "b6 (--rename mangle)",
    "b6 (--rename simple)",
    "b6 (--rename unique)",
    "b6 (--unsafe)",
    "s10 (--unsafe)",
    "s24 (--unsafe)",
    "w31 (--unsafe)",
    "w33 (--unsafe)",
];

const PASSED_THROUGH: i32 = 3;

const MULTI_PREMISE: [&str; 0] = [];

fn data(name: &str) -> PathBuf {
    Path::new(DATA).join(name)
}

fn untwist(args: &[OsString]) {
    untwist_allowing(args, 0);
}

fn untwist_allowing(args: &[OsString], code: i32) {
    let output = common::bin().args(args).output().expect("run untwist");
    assert!(
        output.status.success() || output.status.code() == Some(code),
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

fn is_probe(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "js" || ext == "mjs")
}

fn inputs() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(data(""))
        .expect("probe dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_probe(path))
        .collect();
    found.sort();
    found
}

fn starting(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
                && is_probe(path)
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

fn entry(input: &Path, out: &Path, flags: &[&str]) -> PathBuf {
    let mut args: Vec<OsString> = flags.iter().map(OsString::from).collect();
    args.extend([
        OsString::from("-f"),
        OsString::from("-o"),
        out.as_os_str().to_owned(),
        input.as_os_str().to_owned(),
    ]);
    untwist(&args);
    starting(out, "index.")
        .or_else(|| starting(out, "deobfuscated"))
        .expect("a written file")
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
fn every_a33_rr6_bundle_probe_prints_what_the_bundle_prints() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let probes = inputs();
    assert!(probes.len() >= 80, "found {} probes", probes.len());
    let mut rows = Vec::new();
    for (index, input) in probes.iter().enumerate() {
        let expected = printed(&node, &[], input);
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("{index}-{lane}"));
            let observed = printed(&node, &[], &entry(input, &out, flags));
            rows.push((
                format!("{} ({})", label(input), lane_name(flags)),
                expected.clone(),
                observed,
            ));
        }
    }
    assert_eq!(mismatches(rows), BUNDLE_PREMISE);
}

#[test]
fn the_inline_webpack_entry_keeps_a_bound_require_whole() {
    let work = tempfile::tempdir().expect("tempdir");
    for name in ["w31.js", "w33.js"] {
        for (lane, flags) in LANES.iter().take(5).enumerate() {
            let out = work.path().join(format!("{name}-{lane}"));
            let written = entry(&data(name), &out, flags);
            let code = std::fs::read_to_string(&written).expect("read");
            assert!(!code.contains("require.bind"), "{name} {flags:?}: {code}");
        }
    }
}

#[test]
fn two_classic_scripts_keep_a_shared_self_defending_flag() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let runner = data("vm-runner.cjs");
    let inputs = [data("multi/206.js"), data("multi/206b.js")];
    let run_both = |files: &[PathBuf; 2]| {
        let args = [runner.as_os_str(), files[0].as_os_str()];
        printed(&node, &args, &files[1])
    };
    let expected = run_both(&inputs);
    assert!(expected.contains("x false"), "{expected}");
    let mut rows = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("multi-{lane}"));
        let mut args: Vec<OsString> = flags.iter().map(OsString::from).collect();
        args.extend([
            OsString::from("-f"),
            OsString::from("-o"),
            out.as_os_str().to_owned(),
        ]);
        args.extend(inputs.iter().map(|path| path.as_os_str().to_owned()));
        untwist_allowing(&args, PASSED_THROUGH);
        let outputs = [
            out.join("deobfuscated.206.js"),
            out.join("deobfuscated.206b.js"),
        ];
        rows.push((lane_name(flags), expected.clone(), run_both(&outputs)));
    }
    assert_eq!(mismatches(rows), MULTI_PREMISE);
}
