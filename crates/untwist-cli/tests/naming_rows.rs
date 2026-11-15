mod common;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/naming_review");
const PACKAGE: &str = "{\"type\":\"module\"}\n";

const LANES: [&[&str]; 5] = [
    &[],
    &["--rename", "smart"],
    &["--rename", "mangle"],
    &["--rename", "simple"],
    &["--rename", "unique"],
];

const RR4_WHOLE: [&str; 14] = [
    "e12", "e14", "e18", "e19", "e2", "e20", "e3", "e6", "e7", "w2", "w3", "w4", "w6", "w8",
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

struct Written {
    printed: Option<String>,
    tree: bool,
}

fn tree(node: &Path, input: &Path, out: &Path, flags: &[&str], module: bool) -> Written {
    let mut args = lane_args(flags);
    args.extend([
        OsString::from("-f"),
        OsString::from("-o"),
        out.as_os_str().to_owned(),
        input.as_os_str().to_owned(),
    ]);
    untwist(&args);
    let entry = out.join("index.js");
    let tree = entry.exists();
    let printed = match (tree, module) {
        (true, true) => {
            std::fs::write(out.join("package.json"), PACKAGE).expect("package.json");
            Some(printed(node, &entry))
        }
        (true, false) => Some(printed(node, &entry)),
        (false, true) => None,
        (false, false) => Some(printed(node, &out.join("deobfuscated.js"))),
    };
    Written { printed, tree }
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
            true,
        ),
        (
            "closure-shared-var.js",
            "closure-shared-var.js",
            false,
            "base",
            true,
        ),
        (
            "webpack-var-injection.js",
            "webpack-var-injection.js",
            false,
            "inj",
            true,
        ),
        (
            "systemjs-export-call.js",
            "systemjs-loader.cjs",
            true,
            "helpers-function",
            true,
        ),
        (
            "systemjs-declarator-export.js",
            "systemjs-declarator-export.loader.cjs",
            true,
            "area",
            true,
        ),
        (
            "systemjs-object-default.js",
            "systemjs-object-default.loader.cjs",
            true,
            "area",
            false,
        ),
        (
            "systemjs-default-collision.js",
            "systemjs-default-collision.loader.cjs",
            true,
            "area",
            true,
        ),
        (
            "closure-assigned-var.js",
            "closure-assigned-var.js",
            false,
            "base",
            true,
        ),
        (
            "closure-assigned-var.js",
            "closure-assigned-var.js",
            false,
            "eval b",
            true,
        ),
        (
            "esbuild-parameter-target.js",
            "esbuild-parameter-target.js",
            false,
            "pn",
            true,
        ),
        (
            "esbuild-inner-exports.js",
            "esbuild-inner-exports.js",
            false,
            "pn",
            true,
        ),
        (
            "webpack-inner-exports.js",
            "webpack-inner-exports.js",
            false,
            "pn",
            true,
        ),
        (
            "metro-parameter-target.js",
            "metro-parameter-target.js",
            false,
            "pn",
            false,
        ),
    ];
    let mut wrong = Vec::new();
    for (index, (input, runner, module, marker, splits)) in cases.into_iter().enumerate() {
        let expected = name_lines(&printed(&node, &data(runner)), marker);
        assert!(!expected.is_empty(), "{input}: no {marker} line");
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("{index}-{lane}"));
            let written = tree(&node, &data(input), &out, flags, module);
            if flags.is_empty() && written.tree != splits {
                wrong.push(format!("{input} {flags:?}: tree written {}", written.tree));
            }
            let Some(observed) = written.printed else {
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

#[test]
fn a_name_read_in_another_input_keeps_the_names_an_unpacked_bundle_gives() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let runner = data("realm-runner.cjs");
    let (bundle, reader) = (
        data("closure-two-inputs-a.js"),
        data("closure-two-inputs-b.js"),
    );
    let realm = |first: &std::ffi::OsStr, then: &std::ffi::OsStr, last: &Path| {
        let args = [runner.as_os_str(), first, then];
        run_with(&node, &args, last)
            .expect("run node")
            .map_or("did not finish".to_string(), |out| out.stdout)
    };
    let expected = realm(std::ffi::OsStr::new("-"), bundle.as_os_str(), &reader);
    assert!(expected.contains("sharedFn"), "{expected}");
    let mut wrong = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.path().join(format!("out-{lane}"));
        let mut args = lane_args(flags);
        args.extend([
            OsString::from("-f"),
            OsString::from("-o"),
            out.as_os_str().to_owned(),
            bundle.as_os_str().to_owned(),
            reader.as_os_str().to_owned(),
        ]);
        untwist(&args);
        let entry = out.join("index.js");
        if !entry.exists() {
            wrong.push(format!("{flags:?}: no tree written"));
            continue;
        }
        let entry = std::fs::canonicalize(&entry).expect("entry");
        let observed = realm(
            entry.as_os_str(),
            std::ffi::OsStr::new("--"),
            &out.join("closure-two-inputs-b.js"),
        );
        if observed != expected {
            wrong.push(format!("{flags:?}: inputs {expected:?}, tree {observed:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn rr4_inputs() -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = std::fs::read_dir(data("rr4"))
        .expect("rr4 dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    inputs.sort();
    inputs
}

#[test]
fn factory_and_shield_renames_keep_every_name_a_strict_bundle_gives() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let inputs = rr4_inputs();
    assert!(inputs.len() >= 30, "found {} inputs", inputs.len());
    let mut wrong = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let label = input.file_stem().unwrap().to_string_lossy().to_string();
        let expected = name_lines(&printed(&node, input), "pn");
        assert!(!expected.is_empty(), "{label}: no pn line");
        let splits = !RR4_WHOLE.contains(&label.as_str());
        for (lane, flags) in LANES.iter().enumerate() {
            let out = work.path().join(format!("rr4-{index}-{lane}"));
            let written = tree(&node, input, &out, flags, false);
            if flags.is_empty() && written.tree != splits {
                wrong.push(format!("{label} {flags:?}: tree written {}", written.tree));
            }
            let observed = name_lines(&written.printed.unwrap_or_default(), "pn");
            if observed != expected {
                wrong.push(format!(
                    "{label} {flags:?}: bundle {expected:?}, output {observed:?}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
