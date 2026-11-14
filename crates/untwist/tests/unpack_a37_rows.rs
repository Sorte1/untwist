use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, RenameMode};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const PREMISES: [(&str, &str); 3] = [
    ("p2.js --unsafe --rename smart", "a method installed by every use and called with a function receiver: the documented --unsafe self-read premise"),
    ("r8.js --unsafe", "--unsafe assumes no .caller/.arguments read reaches a module function"),
    ("k1.js --unsafe", "--unsafe assumes no .caller/.arguments read reaches a module function"),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/unpack-a37")
}

fn probes() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for dir in [root(), root().join("m6")] {
        for entry in std::fs::read_dir(dir)
            .expect("probes")
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path
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
            .is_some_and(|name| name.to_string_lossy().starts_with('p'));
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
    match dir.join("bundle.json").exists() {
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
            .file_name()
            .expect("name")
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
    assert!(rows >= 200, "{rows}");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
