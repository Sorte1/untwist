#![cfg(feature = "types")]

use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, TestHooks, TypesMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const FAMILIES: &[(&str, usize)] = &[
    ("behaviour", usize::MAX),
    ("eval", usize::MAX),
    ("unminify", usize::MAX),
    ("untranspile", 120),
    ("obfuscator-io", 120),
];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn js_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            js_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "js") {
            found.push(path);
        }
    }
}

fn sampled(family: &str, limit: usize) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    js_files(&workspace().join("fixtures").join(family), &mut paths);
    let step = match paths.len() > limit {
        true => paths.len().div_ceil(limit),
        false => 1,
    };
    paths.into_iter().step_by(step).collect()
}

fn single_output(options: Options, source: &str) -> Option<String> {
    let output = Pipeline::new(options).run(source);
    (output.bundle.is_none() && output.codes.len() == 1).then(|| output.codes[0].clone())
}

fn assert_options() -> Options {
    Options {
        types: Some(TypesMode::Ts),
        ..Options::default()
    }
    .with_test_hooks(TestHooks {
        types_assert: true,
        ..TestHooks::default()
    })
}

fn extension(code: &str) -> &'static str {
    let alloc = oxc_allocator::Allocator::default();
    match untwist_core::parse::parse_program(&alloc, code).source_type.is_module() {
        true => "mjs",
        false => "cjs",
    }
}

#[test]
fn every_proven_type_holds_when_node_runs_the_fixtures() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for (family, limit) in FAMILIES {
        for (index, path) in sampled(family, *limit).into_iter().enumerate() {
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Some(plain) = single_output(Options::default(), &source) else {
                continue;
            };
            let Some(instrumented) = single_output(assert_options(), &source) else {
                continue;
            };
            if !instrumented.contains("__ut((") {
                continue;
            }
            let ext = extension(&plain);
            let plain_file = dir.path().join(format!("{family}-{index}-plain.{ext}"));
            let checked_file = dir.path().join(format!("{family}-{index}-checked.{ext}"));
            std::fs::write(&plain_file, &plain).unwrap();
            std::fs::write(&checked_file, &instrumented).unwrap();
            let (Ok(Some(expected)), Ok(Some(observed))) =
                (run(&node, &plain_file), run(&node, &checked_file))
            else {
                continue;
            };
            checked += 1;
            let label = path.display().to_string();
            if observed.stderr.contains("__UT_FAIL") {
                failures.push(format!("{label}: a proven type did not hold\n{}", observed.stderr));
            }
            if observed.stdout != expected.stdout || observed.exit != expected.exit {
                failures.push(format!(
                    "{label}: the checked run printed something else\n--- plain\n{}\n--- checked\n{}",
                    expected.stdout, observed.stdout
                ));
            }
        }
    }
    eprintln!("types_assert: {checked} instrumented fixtures ran in node");
    assert!(checked > 0, "no fixture carried a proven type");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_sabotaged_predicate_is_caught_by_the_node_check() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let source = "(function () {\n  function twice(n) {\n    return n * 2;\n  }\n  console.log(twice(1) + twice(2));\n})();\n";
    let instrumented = single_output(assert_options(), source).unwrap();
    assert!(instrumented.contains("__ut(("), "{instrumented}");
    let sabotaged = instrumented.replace("=== \"number\"", "=== \"string\"");
    assert_ne!(sabotaged, instrumented);
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("sabotaged.cjs");
    std::fs::write(&file, &sabotaged).unwrap();
    let observed = run(&node, &file).unwrap().unwrap();
    assert!(observed.stderr.contains("__UT_FAIL"), "{}", observed.stderr);
    assert_eq!(observed.stdout.trim(), "6");
}
