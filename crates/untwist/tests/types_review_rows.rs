#![cfg(feature = "types")]

use std::path::{Path, PathBuf};
use std::process::Command;

use untwist::{Options, Pipeline, TestHooks, TypesMode};
use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const NODE_PROBES: &[&str] = &[
    "a1.js",
    "a2.js",
    "a4.js",
    "a5.js",
    "a6.js",
    "b1.js",
    "g18.js",
    "g19.js",
    "g20.js",
    "g21.js",
    "a40r/ev1.js",
    "a40r/ev2.js",
    "a40r/ev8.js",
    "a40r/fi1.js",
    "a40r/t8b.js",
    "a40r5/u1.mjs",
    "a40r5/s11.mjs",
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

fn unpacked(mode: TypesMode) -> Options {
    Options {
        unsafe_transforms: true,
        ..typed(mode)
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
        let flat = name.replace('/', "-");
        let expected = node_run(&node, dir.path(), &format!("plain-{flat}"), &source);
        let direct = untwist_types::instrumented(&source).unwrap();
        let piped = Pipeline::new(checked()).run(&source).code().to_string();
        for (lane, code) in [("direct", direct), ("pipeline", piped)] {
            let observed = node_run(&node, dir.path(), &format!("{lane}-{flat}"), &code);
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
    let typed_out = Pipeline::new(unpacked(TypesMode::Ts)).run(&source);
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
    let plain_out = Pipeline::new(Options {
        unsafe_transforms: true,
        ..Options::default()
    })
    .run(&source);
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
    let typed_entry = typed_files
        .iter()
        .find(|file| {
            file.file_name()
                .is_some_and(|f| f == "index.ts" || f == "index.js")
        })
        .cloned()
        .unwrap_or_else(|| panic!("no index in {typed_files:?}"));
    assert!(
        typed_files
            .iter()
            .any(|file| file.extension().is_some_and(|ext| ext == "ts")),
        "{typed_files:?}"
    );
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

fn a40r(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist-types/tests/probes/a40r")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn checks_as_javascript(node: &Path, files: &[PathBuf]) -> Vec<String> {
    files
        .iter()
        .filter(|file| file.extension().is_some_and(|ext| ext == "js"))
        .filter_map(
            |file| match untwist_testing::node::syntax_check(node, file).unwrap() {
                Ok(()) => None,
                Err(message) => Some(format!("{}: {message}", file.display())),
            },
        )
        .collect()
}

#[test]
fn a_typed_module_tree_never_holds_typescript_in_a_js_file() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut failures = Vec::new();
    for (name, splits) in [("wp8.js", false), ("wp11.js", true), ("wp12.js", true)] {
        let source = a40r(name);
        let expected = node_run(&node, tempfile::tempdir().unwrap().path(), name, &source);
        let output = Pipeline::new(unpacked(TypesMode::Ts)).run(&source);
        assert_eq!(output.bundle.is_some(), splits, "{name}");
        let (dir, files) = written(&output);
        failures.extend(checks_as_javascript(&node, &files));
        let entry = files
            .iter()
            .find(|file| {
                file.file_name()
                    .is_some_and(|f| f == "index.js" || f == "index.ts")
            })
            .cloned();
        if let Some(entry) = entry {
            let observed = Command::new(&node)
                .arg("--experimental-strip-types")
                .arg("--no-warnings")
                .arg(&entry)
                .output()
                .unwrap();
            if String::from_utf8_lossy(&observed.stdout) != expected.stdout {
                failures.push(format!(
                    "{name}: printed {:?}, the bundle printed {:?}; {}",
                    String::from_utf8_lossy(&observed.stdout),
                    expected.stdout,
                    String::from_utf8_lossy(&observed.stderr)
                ));
            }
        }
        drop(dir);
    }
    let single = Pipeline::new(Options {
        single_file: true,
        ..unpacked(TypesMode::Ts)
    })
    .run(&a40r("wp16.js"));
    let (dir, files) = written(&single);
    failures.extend(checks_as_javascript(&node, &files));
    drop(dir);
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn an_exported_function_takes_its_types_from_no_caller_inside_the_module() {
    let (a, b) = (a40r("ex-a.mjs"), a40r("ex-b.mjs"));
    let output = Pipeline::new(typed(TypesMode::Ts)).run_many(&[&a, &b]);
    for code in &output.codes {
        assert!(!code.contains("c: 3") && !code.contains("d: 4"), "{code}");
    }
}

#[test]
fn a_caller_read_in_another_input_closes_nothing_in_the_pipeline() {
    let (helper, user) = (a40r("fr-b.js"), a40r("fr-a.js"));
    let output = Pipeline::new(typed(TypesMode::Ts)).run_many(&[&helper, &user]);
    assert!(!output.codes[1].contains("a: 1"), "{}", output.codes[1]);
}

#[test]
fn a_local_require_keeps_its_arguments_when_modules_become_typescript() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../untwist-types/tests/probes/a40r2/es3.js");
    let source = std::fs::read_to_string(path).unwrap();
    let output = Pipeline::new(Options {
        split_hoisted: true,
        ..typed(TypesMode::Ts)
    })
    .run(&source);
    let texts: Vec<&str> = output
        .codes
        .iter()
        .map(String::as_str)
        .chain(
            output
                .bundle
                .iter()
                .flat_map(|bundle| bundle.modules.iter().map(|m| m.code.as_str())),
        )
        .collect();
    let joined = texts.join("\n");
    assert!(joined.contains("require(\"./chunk_add.js\")"), "{joined}");
    assert!(joined.contains("require(\"./chunk_add\")"), "{joined}");
    let Some(node) = node_or_skip() else {
        return;
    };
    let expected = node_run(
        &node,
        tempfile::tempdir().unwrap().path(),
        "es3.js",
        &source,
    );
    let (dir, files) = written(&output);
    let entry = files
        .iter()
        .find(|file| {
            file.file_name()
                .is_some_and(|f| f == "index.ts" || f == "index.js")
        })
        .cloned()
        .unwrap_or_else(|| panic!("no index in {files:?}"));
    let observed = Command::new(&node)
        .arg("--experimental-strip-types")
        .arg("--no-warnings")
        .arg(&entry)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&observed.stdout), expected.stdout);
    drop(dir);
}

fn a40r3(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../untwist-types/tests/probes/a40r3")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn a_caller_read_through_the_global_object_breaks_no_type_in_node() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("peek.js"),
        "module.exports = (g) => g.caller;\n",
    )
    .unwrap();
    let outside = "globalThis.peekOutside = (g) => g.caller;\n";
    let mut failures = Vec::new();
    for (name, prelude) in [
        ("h-h1.js", ""),
        ("h-h2.js", ""),
        ("h-h3.js", outside),
        ("h-h4.js", outside),
    ] {
        let source = a40r3(name);
        let expected = node_run(
            &node,
            dir.path(),
            &format!("plain-{name}"),
            &format!("{prelude}{source}"),
        );
        assert!(
            expected.stdout.contains("string"),
            "{name}: {:?} {:?}",
            expected.stdout,
            expected.stderr
        );
        for mode in [TypesMode::Ts, TypesMode::Jsdoc] {
            let options = typed(mode).with_test_hooks(TestHooks {
                types_assert: true,
                ..TestHooks::default()
            });
            let code = Pipeline::new(options).run(&source).code().to_string();
            let observed = node_run(
                &node,
                dir.path(),
                &format!("{mode:?}-{name}"),
                &format!("{prelude}{code}"),
            );
            if observed.stderr.contains("__UT_FAIL") {
                failures.push(format!("{name} {mode:?}: a type did not hold\n{code}"));
            }
            if observed.stdout != expected.stdout {
                failures.push(format!(
                    "{name} {mode:?}: printed {:?}, node printed {:?}",
                    observed.stdout, expected.stdout
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        std::fs::copy(&path, to.join(path.file_name().unwrap())).unwrap();
    }
}

#[test]
fn every_fourth_review_probe_holds_its_types_when_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../untwist-types/tests/probes/a40r4");
    let work = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let mut ran = 0;
    for dir in std::fs::read_dir(&root).unwrap() {
        let dir = dir.unwrap().path();
        let copy = work.path().join(dir.file_name().unwrap());
        copy_dir(&dir, &copy);
        for file in std::fs::read_dir(&dir).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if ["patch", "peek", "run"]
                .iter()
                .any(|helper| name.starts_with(helper))
            {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let expected = node_run(&node, &copy, &format!("plain-{name}"), &source);
            let direct = untwist_types::instrumented(&source).unwrap_or_else(|| source.clone());
            let piped = Pipeline::new(checked()).run(&source).code().to_string();
            for (lane, code) in [("direct", direct), ("pipeline", piped)] {
                let observed = node_run(&node, &copy, &format!("{lane}-{name}"), &code);
                ran += 1;
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
    }
    assert!(ran > 100, "{ran}");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn every_sixth_review_probe_holds_its_types_when_node_runs_it() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../untwist-types/tests/probes/a40r6");
    let work = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    let mut ran = 0;
    for dir in std::fs::read_dir(&root).unwrap() {
        let dir = dir.unwrap().path();
        let copy = work.path().join(dir.file_name().unwrap());
        copy_dir(&dir, &copy);
        for file in std::fs::read_dir(&dir).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let source = std::fs::read_to_string(&path).unwrap();
            let expected = node_run(&node, &copy, &format!("plain-{name}"), &source);
            let annotated = Pipeline::new(typed(TypesMode::Jsdoc))
                .run(&source)
                .code()
                .to_string();
            let observed = node_run(&node, &copy, &format!("jsdoc-{name}"), &annotated);
            if observed.stdout != expected.stdout {
                failures.push(format!(
                    "{name} jsdoc: printed {:?}, node printed {:?}",
                    observed.stdout, expected.stdout
                ));
            }
            let checked = untwist_types::instrumented(&source).unwrap_or_else(|| source.clone());
            let observed = node_run(&node, &copy, &format!("checked-{name}"), &checked);
            ran += 1;
            if observed.stderr.contains("__UT_FAIL") {
                failures.push(format!("{name}: a type did not hold\n{checked}"));
            }
            if observed.stdout != expected.stdout {
                failures.push(format!(
                    "{name}: printed {:?}, node printed {:?}",
                    observed.stdout, expected.stdout
                ));
            }
        }
    }
    assert!(ran > 40, "{ran}");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
