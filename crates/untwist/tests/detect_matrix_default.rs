use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};

const BUNDLERS: &str = "../../fixtures/unpack-bundlers";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(BUNDLERS)
}

fn manifest_paths() -> Vec<String> {
    let text = std::fs::read_to_string(root().join("MANIFEST")).expect("read MANIFEST");
    let mut paths: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').next().unwrap_or_default().to_string())
        .collect();
    paths.sort();
    paths
}

fn unpack_only() -> Options {
    Options {
        deobfuscate: false,
        unminify: false,
        jsx: false,
        ..Options::default()
    }
}

fn split(options: &Options, source: &str) -> Option<(String, usize)> {
    let output = Pipeline::new(options.clone()).run(source);
    output
        .bundle
        .map(|bundle| (bundle.kind, bundle.modules.len()))
}

#[test]
fn the_default_pipeline_splits_every_bundler_fixture_as_unpack_only_does() {
    let defaults = Options::default();
    let mut wrong = Vec::new();
    let mut split_rows = 0;
    for path in manifest_paths() {
        let source = std::fs::read_to_string(root().join(&path)).expect("read fixture");
        let want = split(&unpack_only(), &source);
        let got = split(&defaults, &source);
        split_rows += usize::from(want.is_some());
        if got != want {
            wrong.push(format!("{path}: unpack-only {want:?}, default {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(split_rows > 0, "no fixture was split");
}
