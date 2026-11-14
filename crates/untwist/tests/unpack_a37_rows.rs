use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const CALLER_PREMISE: &str =
    "--unsafe assumes no .caller/.arguments read reaches a module function";

const PREMISES: [(&str, &str); 16] = [
    (
        "unpack-a37/p2.js --unsafe --rename smart",
        "a method installed by every use and called with a function receiver: the documented --unsafe self-read premise",
    ),
    ("unpack-a37/r4.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/r7.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/r8.js --unsafe", CALLER_PREMISE),
    ("unpack-a37/k1.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es0.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es2.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/es3.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g0.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g1.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g2.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g3.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g4.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g5.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g6.js --unsafe", CALLER_PREMISE),
    ("unpack-a37rr/g7.js --unsafe", CALLER_PREMISE),
];

const LINKED: [&str; 2] = ["lib1.js", "mi1.js"];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn probes() -> Vec<PathBuf> {
    let root = fixtures();
    let mut found = Vec::new();
    for dir in [
        root.join("unpack-a37"),
        root.join("unpack-a37").join("m6"),
        root.join("unpack-a37rr"),
    ] {
        for entry in std::fs::read_dir(dir)
            .expect("probes")
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let linked = path
                .parent()
                .is_some_and(|dir| dir.ends_with("unpack-a37rr"))
                && path
                    .file_name()
                    .is_some_and(|name| LINKED.contains(&name.to_string_lossy().as_ref()));
            if !linked
                && path
                    .extension()
                    .is_some_and(|ext| ext == "js" || ext == "mjs")
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn lanes(path: &Path) -> [(&'static str, Options); 2] {
    let renamed = path.extension().is_some_and(|ext| ext == "mjs")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with('p'))
            && !path
                .parent()
                .is_some_and(|dir| dir.ends_with("unpack-a37rr"));
    let options = |unsafe_transforms: bool| Options {
        unsafe_transforms,
        rename: if renamed {
            RenameMode::Smart
        } else {
            RenameMode::None
        },
        ..Options::default()
    };
    match renamed {
        true => [
            ("--rename smart", options(false)),
            ("--unsafe --rename smart", options(true)),
        ],
        false => [("", options(false)), ("--unsafe", options(true))],
    }
}

fn printed(node: &Path, file: &Path) -> String {
    let observed = run(node, file).expect("spawn").expect("terminates");
    observed.stdout
}

fn written(path: &Path, options: Options, dir: &Path) -> PathBuf {
    let code = std::fs::read_to_string(path).expect("probe");
    let out = Pipeline::new(options).run(&code);
    if path.extension().is_some_and(|ext| ext == "mjs") {
        let file = dir.join(path.file_name().expect("name"));
        std::fs::write(&file, out.code()).expect("write");
        return file;
    }
    out.write_to(dir).expect("write");
    match dir.join("index.js").exists() {
        true => dir.join("index.js"),
        false => dir.join("deobfuscated.js"),
    }
}

#[test]
fn every_review_probe_prints_the_same_after_untwist_in_both_lanes() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let mut wrong = Vec::new();
    let mut rows = 0;
    for path in probes() {
        let expected = printed(&node, &path);
        let name = path
            .strip_prefix(fixtures())
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        for (flags, options) in lanes(&path) {
            let row = format!("{name} {flags}").trim().to_string();
            let dir = tempfile::tempdir().expect("temp dir");
            let entry = written(&path, options, dir.path());
            let got = printed(&node, &entry);
            let premise = PREMISES.iter().any(|(at, _)| *at == row);
            rows += 1;
            match (got == expected, premise) {
                (true, true) => wrong.push(format!("{row}: now agrees, drop it from PREMISES")),
                (false, false) => wrong.push(format!(
                    "{row}: node printed {expected:?}, untwist's output printed {got:?}"
                )),
                _ => {}
            }
        }
    }
    assert!(rows >= 290, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const REALM: &str = r#"const vm = require("vm"), fs = require("fs");
const [library, main] = process.argv.slice(2);
vm.runInThisContext(fs.readFileSync(library, "utf8"), { filename: library });
if (main.endsWith("index.js")) require(main);
else vm.runInThisContext(fs.readFileSync(main, "utf8"), { filename: main });
"#;

fn in_one_realm(node: &Path, runner: &Path, library: &Path, main: &Path) -> (String, String) {
    let observed =
        untwist_testing::node::run_with(node, &[runner.as_os_str(), library.as_os_str()], main)
            .expect("spawn")
            .expect("terminates");
    (observed.stdout, observed.stderr)
}

#[test]
fn a_caller_read_in_another_input_keeps_the_bundle_whole_in_the_default_lane() {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return;
    };
    let root = fixtures().join("unpack-a37rr");
    let [library, main] = LINKED.map(|name| root.join(name));
    let dir = tempfile::tempdir().expect("temp dir");
    let runner = dir.path().join("realm.js");
    std::fs::write(&runner, REALM).expect("runner");
    let (expected, _) = in_one_realm(&node, &runner, &library, &main);
    let inputs = [library.as_path(), main.as_path()]
        .map(|path| std::fs::read_to_string(path).expect("probe"));
    let out = Pipeline::new(Options::default()).run_many(&[inputs[0].as_str(), inputs[1].as_str()]);
    let tree = dir.path().join("tree");
    out.write_to(&tree).expect("write");
    assert!(!tree.join("bundle.json").exists(), "{:?}", out.notes);
    let (got, errors) = in_one_realm(
        &node,
        &runner,
        &tree.join("deobfuscated.input0.js"),
        &tree.join("deobfuscated.input1.js"),
    );
    let written: Vec<_> = std::fs::read_dir(&tree)
        .expect("tree")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(got, expected, "{errors}\n{written:?}");
}
