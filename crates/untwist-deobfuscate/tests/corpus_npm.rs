use std::path::{Path, PathBuf};

use untwist_testing::node::{node, normalise_stderr, run, skip_allowed, Observed, SKIP_MESSAGE};

const SKIP: &str =
    "skipping npm corpus: fixtures/corpus-npm is absent; run `cargo xtask corpus-npm`";

#[derive(serde::Deserialize)]
struct Case {
    file: String,
    expected_output: String,
}

fn options(extended: bool) -> untwist::Options {
    untwist::Options {
        unsafe_transforms: true,
        extended,
        ..untwist::Options::default()
    }
}

fn node_or_skip() -> Option<PathBuf> {
    if let Some(found) = node() {
        return Some(found);
    }
    let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
    assert!(
        skip_allowed(required.as_deref()),
        "required: {SKIP_MESSAGE}"
    );
    println!("{SKIP_MESSAGE}");
    None
}

fn behaviour_required() -> bool {
    std::env::var("UNTWIST_REQUIRE_NODE").is_ok_and(|value| !value.is_empty())
}

fn observe(node: &Path, dir: &Path, name: &str, code: &str) -> (PathBuf, Option<Observed>) {
    let path = dir.join(name);
    std::fs::write(&path, code).expect("write case output");
    let seen = run(node, &path).expect("run node");
    (path, seen)
}

fn rendered(seen: &Observed, path: &Path) -> String {
    format!(
        "exit={:?}\nstdout={}\nstderr={}",
        seen.exit,
        seen.stdout,
        normalise_stderr(&seen.stderr, &[path])
    )
}

fn check_case(node: &Path, dir: &Path, case: &Case, source: &str, failures: &mut Vec<String>) {
    let input = observe(node, dir, &format!("{}.input.js", case.file), source);
    for (mode, extended) in [("conservative", false), ("default", true)] {
        let out = untwist::Pipeline::new(options(extended)).run(source);
        let name = format!("{}.{mode}.js", case.file);
        let path = dir.join(&name);
        std::fs::write(&path, out.code()).expect("write case output");
        match untwist_testing::node::syntax_check(node, &path).expect("node --check") {
            Ok(()) => {}
            Err(stderr) => failures.push(format!("{name}: node --check failed: {stderr}")),
        }
        if !behaviour_required() {
            continue;
        }
        let (after_path, after) = observe(node, dir, &name, out.code());
        let (Some(before), Some(after)) = (input.1.as_ref(), after.as_ref()) else {
            failures.push(format!("{name}: a run did not terminate"));
            continue;
        };
        let before_text = rendered(before, &input.0);
        let after_text = rendered(after, &after_path);
        if before_text != after_text {
            failures.push(format!(
                "{name}:\nbefore {before_text}\nafter  {after_text}"
            ));
        }
    }
}

#[test]
fn every_generated_case_deobfuscates_to_the_known_program() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/corpus-npm");
    let required = std::env::var("UNTWIST_REQUIRE_CORPUS").is_ok_and(|value| !value.is_empty());
    let Ok(manifest) = std::fs::read_to_string(dir.join("manifest.json")) else {
        assert!(!required, "required: {SKIP}");
        println!("{SKIP}");
        return;
    };
    let cases: Vec<Case> = serde_json::from_str(&manifest).expect("manifest");
    assert!(!cases.is_empty());
    let found = node_or_skip();
    let tmp = tempfile::tempdir().expect("temp dir");
    let mut failures: Vec<String> = Vec::new();
    for case in &cases {
        let path = dir.join(&case.file);
        let source = std::fs::read_to_string(&path).expect("case");
        let out = untwist::Pipeline::new(options(true)).run(&source);
        if !out.code().contains(&case.expected_output) {
            failures.push(format!("{}: did not decode", case.file));
        }
        if let Some(node) = &found {
            check_case(node, tmp.path(), case, &source, &mut failures);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
