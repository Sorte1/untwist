mod common;

use std::path::{Path, PathBuf};

use common::bin;
use untwist_testing::node;

const DEFAULT: &[&str] = &[];
const UNSAFE: &[&str] = &["--unsafe", "--split-hoisted"];
const LANES: [&[&str]; 2] = [DEFAULT, UNSAFE];
const MODULES: usize = 3;
const MODULES_TWO_ENTRIES: usize = 4;
const REBOUND_REQUIRE: &str = "+uglify";
const EXPORTED_REQUIRE: &str = "bp6.1-exports";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/browserify-preludes")
}

fn cells() -> Vec<String> {
    std::fs::read_to_string(fixtures().join("MANIFEST"))
        .unwrap()
        .lines()
        .filter_map(|line| line.split('\t').next())
        .map(str::to_string)
        .collect()
}

fn unpacked(cell: &str) -> Option<usize> {
    let refused = cell.ends_with(REBOUND_REQUIRE) || cell == EXPORTED_REQUIRE;
    let modules = match cell.ends_with("-two-entries") {
        true => MODULES_TWO_ENTRIES,
        false => MODULES,
    };
    (!refused).then_some(modules)
}

fn written(cell: &str, flags: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let run = bin()
        .args(flags)
        .arg("-o")
        .arg(&out)
        .arg(fixtures().join(cell).join("index.js"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{cell}: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    (dir, out)
}

fn modules(out: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(out.join("bundle.json")).ok()?;
    let listing: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(listing["type"], "browserify");
    listing["modules"].as_array().map(Vec::len)
}

#[test]
fn every_browser_pack_prelude_is_unpacked_in_every_lane() {
    for cell in cells() {
        for flags in LANES {
            let (_dir, out) = written(&cell, flags);
            assert_eq!(modules(&out), unpacked(&cell), "{cell} {flags:?}");
        }
    }
}

#[test]
fn the_written_tree_prints_what_the_browserify_bundle_printed() {
    let Some(node) = node::node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            node::skip_allowed(required.as_deref()),
            "{}",
            node::SKIP_MESSAGE
        );
        eprintln!("{}", node::SKIP_MESSAGE);
        return;
    };
    for cell in cells() {
        let expected =
            std::fs::read_to_string(fixtures().join("expected").join(format!("{cell}.stdout")))
                .unwrap();
        for flags in LANES {
            let (_dir, out) = written(&cell, flags);
            let entry = match modules(&out) {
                Some(_) => "index.js",
                None => "deobfuscated.js",
            };
            let ran = node::run(&node, &out.join(entry))
                .unwrap()
                .expect("node finished in time");
            let exit = ran
                .exit
                .map_or_else(|| "timeout".to_string(), |code| code.to_string());
            let observed = format!("exit={exit}\n{}", ran.stdout);
            assert_eq!(observed, expected, "{cell} {flags:?}: {}", ran.stderr);
        }
    }
}
