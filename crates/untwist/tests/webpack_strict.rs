use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const MODULES: [&str; 5] = [
    "index.js",
    "greet.js",
    "shop/cart.js",
    "shop/format.js",
    "shop/tax.js",
];
const FLAT_MODULES: [&str; 5] = [
    "index.js",
    "greet.js",
    "shop_cart.js",
    "shop_format.js",
    "shop_tax.js",
];

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/webpack-strict")
}

fn cells() -> Vec<(String, String, String)> {
    let mut found: Vec<_> = std::fs::read_dir(fixtures())
        .expect("fixtures/webpack-strict")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .map(|path| {
            let code = std::fs::read_to_string(&path).expect("read");
            let stdout = std::fs::read_to_string(path.with_extension("stdout")).expect("stdout");
            (
                path.file_stem().unwrap().to_string_lossy().to_string(),
                code,
                stdout,
            )
        })
        .collect();
    found.sort();
    found
}

fn lane(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        jsx: false,
        ..Options::default()
    }
}

fn printed(path: &Path) -> Option<String> {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        return None;
    };
    let observed = run(&node, path).expect("spawn").expect("terminates");
    Some(format!("{}{}", observed.stdout, observed.stderr))
}

#[test]
fn the_fixtures_hold_both_webpack_modes() {
    let names: Vec<String> = cells().into_iter().map(|(name, _, _)| name).collect();
    assert_eq!(names, ["w5-dev", "w5-none"]);
}

#[test]
fn a_strict_webpack_5_bundle_with_its_entry_inlined_splits_in_both_lanes_and_runs_alike() {
    for (name, code, stdout) in cells() {
        let modules = match name.ends_with("-dev") {
            true => MODULES,
            false => FLAT_MODULES,
        };
        for unsafe_transforms in [false, true] {
            let out = Pipeline::new(lane(unsafe_transforms)).run(&code);
            let notes: Vec<String> = out.notes.iter().map(|note| note.message.clone()).collect();
            let dir = tempfile::tempdir().expect("temp dir");
            out.write_to(dir.path()).expect("write");
            assert!(
                dir.path().join("bundle.json").exists(),
                "{name} unsafe={unsafe_transforms}: {notes:?}"
            );
            for module in modules {
                let text = std::fs::read_to_string(dir.path().join(module))
                    .unwrap_or_else(|_| panic!("{name} unsafe={unsafe_transforms}: {module}"));
                assert!(
                    text.starts_with("\"use strict\";"),
                    "{name} unsafe={unsafe_transforms}: {module} is written sloppy"
                );
            }
            if let Some(out) = printed(&dir.path().join("index.js")) {
                assert_eq!(out, stdout, "{name} unsafe={unsafe_transforms}");
            }
        }
    }
}
