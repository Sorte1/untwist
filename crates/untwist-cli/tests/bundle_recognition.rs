mod common;

use std::path::{Path, PathBuf};

use common::bin;
use untwist_testing::node;

const DEFAULT: &[&str] = &[];
const UNSAFE: &[&str] = &["--unsafe", "--split-hoisted"];
const LANES: [&[&str]; 2] = [DEFAULT, UNSAFE];
const UNSAFE_ONLY: [&[&str]; 1] = [UNSAFE];

struct Cell {
    app: &'static str,
    name: &'static str,
    family: &'static str,
    modules: usize,
    lanes: &'static [&'static [&'static str]],
}

const CELLS: &[Cell] = &[
    Cell {
        app: "base",
        name: "esbuild0.28-iife+terser",
        family: "esbuild",
        modules: 25,
        lanes: &LANES,
    },
    Cell {
        app: "base",
        name: "esbuild0.28-iife+swc-min",
        family: "esbuild",
        modules: 25,
        lanes: &LANES,
    },
    Cell {
        app: "base",
        name: "bun-iife-minify",
        family: "esbuild",
        modules: 25,
        lanes: &LANES,
    },
    Cell {
        app: "mini",
        name: "bun-iife-minify",
        family: "esbuild",
        modules: 2,
        lanes: &LANES,
    },
    Cell {
        app: "base",
        name: "esbuild0.28-iife+obf-object-keys",
        family: "esbuild",
        modules: 25,
        lanes: &UNSAFE_ONLY,
    },
    Cell {
        app: "mini",
        name: "parcel-hoist-min",
        family: "parcel",
        modules: 4,
        lanes: &LANES,
    },
    Cell {
        app: "mini",
        name: "parcel-hoist",
        family: "parcel",
        modules: 4,
        lanes: &UNSAFE_ONLY,
    },
    Cell {
        app: "mini",
        name: "parcel-nohoist-min",
        family: "parcel",
        modules: 5,
        lanes: &LANES,
    },
    Cell {
        app: "mini",
        name: "parcel2.12-nohoist",
        family: "parcel",
        modules: 5,
        lanes: &LANES,
    },
    Cell {
        app: "mini",
        name: "parcel2.9-nohoist-min",
        family: "parcel",
        modules: 5,
        lanes: &LANES,
    },
    Cell {
        app: "base",
        name: "esbuild0.28-esm-split",
        family: "esbuild",
        modules: 25,
        lanes: &LANES,
    },
    Cell {
        app: "mini",
        name: "esbuild0.14-esm-split",
        family: "esbuild",
        modules: 2,
        lanes: &LANES,
    },
    Cell {
        app: "base",
        name: "bun1.2-esm-splitting",
        family: "esbuild",
        modules: 25,
        lanes: &LANES,
    },
    Cell {
        app: "corejs",
        name: "parcel-nohoist",
        family: "parcel",
        modules: 86,
        lanes: &LANES,
    },
];

fn matrix() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bundle-matrix")
}

fn inputs(cell: &Cell) -> Vec<PathBuf> {
    let dir = matrix().join(cell.app).join(cell.name);
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort_by_key(|path| path.file_name() != Some("index.js".as_ref()));
    files
}

fn expected(cell: &Cell) -> String {
    let file = matrix()
        .join("expected")
        .join(cell.app)
        .join(format!("{}.stdout", cell.name));
    std::fs::read_to_string(file).unwrap()
}

fn listing(out: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(out.join("bundle.json")).expect("a bundle listing");
    serde_json::from_str(&text).unwrap()
}

fn entry_path(listing: &serde_json::Value) -> String {
    listing["modules"]
        .as_array()
        .and_then(|modules| {
            modules
                .iter()
                .find(|module| module["isEntry"].as_bool() == Some(true))
        })
        .and_then(|module| module["path"].as_str())
        .unwrap_or("index.js")
        .to_string()
}

fn written(cell: &Cell, flags: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let run = bin()
        .args(flags)
        .arg("-o")
        .arg(&out)
        .args(inputs(cell))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}/{}: {}",
        cell.app,
        cell.name,
        String::from_utf8_lossy(&run.stderr)
    );
    (dir, out)
}

#[test]
fn reminified_and_reshaped_bundles_are_unpacked_in_every_lane() {
    for cell in CELLS {
        for flags in cell.lanes {
            let (_dir, out) = written(cell, flags);
            let listing = listing(&out);
            let label = format!("{}/{} {flags:?}", cell.app, cell.name);
            assert_eq!(listing["type"], cell.family, "{label}");
            let modules = listing["modules"].as_array().map_or(0, Vec::len);
            assert_eq!(modules, cell.modules, "{label}");
        }
    }
}

#[test]
fn the_written_tree_prints_what_the_bundle_printed() {
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
    for cell in CELLS {
        for flags in cell.lanes {
            let (_dir, out) = written(cell, flags);
            let entry = entry_path(&listing(&out));
            let ran = node::run(&node, &out.join(entry))
                .unwrap()
                .expect("node finished in time");
            let exit = ran
                .exit
                .map_or_else(|| "timeout".to_string(), |code| code.to_string());
            let observed = format!("exit={exit}\n{}", ran.stdout);
            assert_eq!(
                observed,
                expected(cell),
                "{}/{} {flags:?}: {}",
                cell.app,
                cell.name,
                ran.stderr
            );
        }
    }
}
