#![cfg(feature = "types")]

use std::path::{Path, PathBuf};
use std::process::Command;

use untwist::{Options, Pipeline, TestHooks, TypesMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const NODE_PROBES: &[&str] = &[
    "a1.js", "a2.js", "a4.js", "a5.js", "a6.js", "b1.js", "g18.js", "g19.js", "g20.js", "g21.js",
];

fn probe(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist-types/tests/probes")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn typed(mode: TypesMode) -> Options {
    Options {
        types: Some(mode),
        ..Options::default()
    }
}

fn checked() -> Options {
    typed(TypesMode::Ts).with_test_hooks(TestHooks {
        types_assert: true,
        ..TestHooks::default()
    })
}

fn node_run(node: &Path, dir: &Path, name: &str, code: &str) -> untwist_testing::node::Observed {
    let file = dir.join(name);
    std::fs::write(&file, code).unwrap();
    run(node, &file).unwrap().expect("node finished")
}

#[test]
fn every_review_probe_holds_its_types_when_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for name in NODE_PROBES {
        let source = probe(name);
        let expected = node_run(&node, dir.path(), &format!("plain-{name}"), &source);
        let direct = untwist_types::instrumented(&source).unwrap();
        let piped = Pipeline::new(checked()).run(&source).code().to_string();
        for (lane, code) in [("direct", direct), ("pipeline", piped)] {
            let observed = node_run(&node, dir.path(), &format!("{lane}-{name}"), &code);
            if observed.stderr.contains("__UT_FAIL") {
                failures.push(format!("{name} {lane}: a type did not hold\n{code}"));
            }
            if observed.stdout != expected.stdout {
                failures.push(format!(
                    "{name} {lane}: printed {:?}, node printed {:?}",
                    observed.stdout, expected.stdout
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_file_typescript_reads_differently_keeps_its_js_name() {
    let output = Pipeline::new(typed(TypesMode::Ts)).run(&probe("b3.js"));
    let dir = tempfile::tempdir().unwrap();
    output.write_to(dir.path()).unwrap();
    assert!(dir.path().join("deobfuscated.js").exists());
    assert!(!dir.path().join("deobfuscated.ts").exists());
}

#[test]
fn two_inputs_share_one_realm_for_the_built_in_rows() {
    let (patch, user) = (probe("m1.js"), probe("m2.js"));
    let output = Pipeline::new(typed(TypesMode::Ts)).run_many(&[&patch, &user]);
    assert!(
        !output.codes[1].contains("r: number"),
        "{}",
        output.codes[1]
    );
    assert!(
        !output.codes[1].contains("t: string"),
        "{}",
        output.codes[1]
    );
}

fn written(output: &untwist::Output) -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempfile::tempdir().unwrap();
    let report = output.write_to(dir.path()).unwrap();
    let files = report
        .files
        .iter()
        .map(|file| dir.path().join(file))
        .collect();
    (dir, files)
}

#[test]
fn a_typescript_module_tree_runs_where_the_javascript_one_does() {
    let source = probe("wp.js");
    let typed_out = Pipeline::new(typed(TypesMode::Ts)).run(&source);
    let bundle = typed_out.bundle.as_ref().expect("the probe unpacks");
    for module in &bundle.modules {
        assert!(!module.code.contains("r: number"), "{}", module.code);
        assert!(
            !module.code.contains("\"./1.js\"") && !module.code.contains("\"./2.js\""),
            "{}",
            module.code
        );
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    let plain_out = Pipeline::new(Options::default()).run(&source);
    let (plain_dir, plain_files) = written(&plain_out);
    let (typed_dir, typed_files) = written(&typed_out);
    let entry = |files: &[PathBuf], ext: &str| {
        files
            .iter()
            .find(|file| {
                file.file_name()
                    .is_some_and(|name| name == format!("index.{ext}").as_str())
            })
            .cloned()
            .unwrap_or_else(|| panic!("no index.{ext} in {files:?}"))
    };
    let plain_entry = entry(&plain_files, "js");
    let typed_entry = entry(&typed_files, "ts");
    let expected = run(&node, &plain_entry).unwrap().expect("node finished");
    let observed = Command::new(&node)
        .arg("--experimental-strip-types")
        .arg("--no-warnings")
        .arg(&typed_entry)
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&observed.stdout),
        expected.stdout,
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    drop((plain_dir, typed_dir));
}
