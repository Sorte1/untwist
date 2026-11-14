mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/naming_review");
const PACKAGE: &str = "{\"type\":\"module\"}\n";

const LANES: [&[&str]; 4] = [
    &[],
    &["--rename", "smart"],
    &["--rename", "mangle"],
    &["--rename", "simple"],
];

fn data(name: &str) -> PathBuf {
    Path::new(DATA).join(name)
}

fn untwist(args: &[OsString]) -> std::process::Output {
    let output = common::bin().args(args).output().expect("run untwist");
    assert!(
        output.status.success(),
        "untwist {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn printed(node: &Path, file: &Path) -> String {
    let observed = run_with(node, &[], file).expect("run node");
    observed.map_or("did not finish".to_string(), |out| {
        format!("{:?}\n{}", out.exit, out.stdout)
    })
}

fn lane_args(flags: &[&str]) -> Vec<OsString> {
    flags.iter().map(OsString::from).collect()
}

fn tree(node: &Path, input: &Path, out: &Path, flags: &[&str], module: bool) -> Option<String> {
    let mut args = lane_args(flags);
    args.extend([
        OsString::from("-f"),
        OsString::from("-o"),
        out.as_os_str().to_owned(),
        input.as_os_str().to_owned(),
    ]);
    untwist(&args);
    let entry = out.join("index.js");
    match (entry.exists(), module) {
        (true, true) => {
            std::fs::write(out.join("package.json"), PACKAGE).expect("package.json");
            Some(printed(node, &entry))
        }
        (true, false) => Some(printed(node, &entry)),
        (false, true) => None,
        (false, false) => Some(printed(node, &out.join("deobfuscated.js"))),
    }
}

fn name_lines(printed: &str, marker: &str) -> String {
    printed
        .lines()
        .filter(|line| line.contains(marker))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_name_read_in_another_classic_script_keeps_the_wrappers() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let inputs = [data("scripts-a.js"), data("scripts-b.js")];
    let joined = work.path().join("joined-in.js");
    let text: Vec<String> = inputs
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read input"))
        .collect();
    std::fs::write(&joined, text.join("\n")).expect("write joined input");
    let expected = printed(&node, &joined);
    let mut wrong = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("out-{lane}"));
        let mut args = lane_args(flags);
        args.extend([
            OsString::from("-f"),
            OsString::from("-o"),
            out.as_os_str().to_owned(),
        ]);
        args.extend(inputs.iter().map(|path| path.as_os_str().to_owned()));
        untwist(&args);
        let written: Vec<String> = ["deobfuscated.scripts-a.js", "deobfuscated.scripts-b.js"]
            .iter()
            .map(|name| std::fs::read_to_string(out.join(name)).expect("read output"))
            .collect();
        let joined_out = work.path().join(format!("joined-out-{lane}.js"));
        std::fs::write(&joined_out, written.join("\n")).expect("write joined output");
        let observed = printed(&node, &joined_out);
        if observed != expected {
            wrong.push(format!(
                "{flags:?}: inputs {expected:?}, outputs {observed:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_exported_value_keeps_the_name_its_importer_reads() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let input = data("module-exports.mjs");
    let importer = std::fs::read_to_string(data("module-importer.mjs")).expect("importer");
    let before = work.path().join("before");
    std::fs::create_dir_all(&before).expect("dir");
    std::fs::copy(&input, before.join("module-exports.mjs")).expect("copy module");
    std::fs::write(before.join("module-importer.mjs"), &importer).expect("importer");
    let expected = printed(&node, &before.join("module-importer.mjs"));
    let mut wrong = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let dir = work.path().join(format!("lane-{lane}"));
        std::fs::create_dir_all(&dir).expect("dir");
        let mut args = lane_args(flags);
        args.push(input.as_os_str().to_owned());
        let output = untwist(&args);
        std::fs::write(dir.join("module-exports.mjs"), &output.stdout).expect("module");
        std::fs::write(dir.join("module-importer.mjs"), &importer).expect("importer");
        let observed = printed(&node, &dir.join("module-importer.mjs"));
        if observed != expected {
            wrong.push(format!(
                "{flags:?}: input {expected:?}, output {observed:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn unpacked_trees_keep_the_names_the_bundle_gives() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let cases = [
        (
            "closure-shared-var.js",
            "closure-shared-var.js",
            false,
            "eval",
        ),
        (
            "closure-shared-var.js",
            "closure-shared-var.js",
            false,
            "base",
        ),
        (
            "webpack-var-injection.js",
            "webpack-var-injection.js",
            false,
            "inj",
        ),
        (
            "systemjs-export-call.js",
            "systemjs-loader.cjs",
            true,
            "helpers-function",
        ),
    ];
    let mut wrong = Vec::new();
    for (index, (input, runner, module, marker)) in cases.into_iter().enumerate() {
        let expected = name_lines(&printed(&node, &data(runner)), marker);
        assert!(!expected.is_empty(), "{input}: no {marker} line");
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("{index}-{lane}"));
            let Some(observed) = tree(&node, &data(input), &out, flags, module) else {
                continue;
            };
            let observed = name_lines(&observed, marker);
            if observed != expected {
                wrong.push(format!(
                    "{input} {flags:?}: bundle {expected:?}, tree {observed:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
