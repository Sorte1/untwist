mod common;

use std::path::{Path, PathBuf};

use common::bin;
use untwist_testing::node;

const FLAGS: [&[&str]; 3] = [&[], &["--split-hoisted"], &["--unsafe", "--split-hoisted"]];

struct Cell {
    name: &'static str,
    inputs: &'static [&'static str],
    mirrored: bool,
    entry_module_runs: bool,
}

const CELLS: &[Cell] = &[
    Cell {
        name: "mini-parcel-nohoist-min",
        inputs: &["index.js", "lazy.d601cb87.js"],
        mirrored: true,
        entry_module_runs: false,
    },
    Cell {
        name: "mini-parcel-nohoist",
        inputs: &["index.js", "lazy.ab68b3d2.js"],
        mirrored: false,
        entry_module_runs: true,
    },
    Cell {
        name: "base-vite8-es-min",
        inputs: &["index.js", "lazy.chunk.js"],
        mirrored: true,
        entry_module_runs: true,
    },
    Cell {
        name: "base-rolldown-esm-min",
        inputs: &["index.js", "lazy.chunk.js"],
        mirrored: true,
        entry_module_runs: true,
    },
    Cell {
        name: "base-bun-esm-splitting",
        inputs: &["index.js", "lazy-xv2chy4w.chunk.js"],
        mirrored: true,
        entry_module_runs: true,
    },
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/write-tree")
}

fn expected(cell: &Cell) -> String {
    let path = fixtures()
        .join("expected")
        .join(format!("{}.stdout", cell.name));
    std::fs::read_to_string(&path).unwrap()
}

fn observed(node: &Path, file: &Path) -> String {
    let ran = node::run(node, file)
        .unwrap()
        .expect("node finished in time");
    let exit = ran
        .exit
        .map_or_else(|| "timeout".to_string(), |code| code.to_string());
    format!("exit={exit}\n{}", ran.stdout)
}

fn written(cell: &Cell, flags: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let inputs = cell
        .inputs
        .iter()
        .map(|input| fixtures().join(cell.name).join(input));
    let run = bin()
        .args(flags)
        .arg("-o")
        .arg(&out)
        .args(inputs)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}: {}",
        cell.name,
        String::from_utf8_lossy(&run.stderr)
    );
    (dir, out)
}

fn listing(out: &Path) -> serde_json::Value {
    let text = std::fs::read_to_string(out.join("bundle.json")).expect("a bundle listing");
    serde_json::from_str(&text).unwrap()
}

fn module_paths(out: &Path) -> Vec<(String, bool)> {
    listing(out)["modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|module| {
            (
                module["path"].as_str().unwrap().to_string(),
                module["isEntry"].as_bool().unwrap(),
            )
        })
        .collect()
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node::node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            node::skip_allowed(required.as_deref()),
            "{}",
            node::SKIP_MESSAGE
        );
        eprintln!("{}", node::SKIP_MESSAGE);
    }
    found
}

fn has_deobfuscated_file(out: &Path) -> bool {
    std::fs::read_dir(out).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("deobfuscated.")
    })
}

#[test]
fn every_written_tree_runs_like_the_inputs() {
    let Some(node) = node_or_skip() else {
        return;
    };
    for cell in CELLS {
        let want = expected(cell);
        for flags in FLAGS {
            let (_dir, out) = written(cell, flags);
            if !out.join("bundle.json").is_file() {
                assert!(
                    !flags.is_empty(),
                    "{}: the default lane unpacks it",
                    cell.name
                );
                continue;
            }
            assert_eq!(
                observed(&node, &out.join("index.js")),
                want,
                "{} {flags:?}",
                cell.name
            );
        }
    }
}

#[test]
fn an_entry_module_beside_chunk_inputs_runs_like_the_inputs() {
    let Some(node) = node_or_skip() else {
        return;
    };
    for cell in CELLS.iter().filter(|cell| cell.entry_module_runs) {
        let (_dir, out) = written(cell, &[]);
        let paths = module_paths(&out);
        let (entry, _) = paths.iter().find(|(_, is_entry)| *is_entry).unwrap();
        assert_eq!(
            observed(&node, &out.join(entry)),
            expected(cell),
            "{} {entry}",
            cell.name
        );
    }
}

#[test]
fn a_partial_bundle_writes_each_input_under_its_own_name() {
    for cell in CELLS {
        let (_dir, out) = written(cell, &[]);
        let paths = module_paths(&out);
        assert_eq!(has_deobfuscated_file(&out), !cell.mirrored, "{}", cell.name);
        if !cell.mirrored {
            continue;
        }
        for input in cell.inputs {
            assert!(out.join(input).is_file(), "{} {input}", cell.name);
            assert!(
                paths.iter().all(|(path, _)| path != input),
                "{}: a module took the input name {input}",
                cell.name
            );
        }
    }
}

#[test]
fn a_chunk_unpacked_without_its_loader_does_not_claim_index_js() {
    let cell = &CELLS[0];
    let (_dir, out) = written(cell, &[]);

    let paths = module_paths(&out);
    let main = std::fs::read_to_string(out.join("index.js")).unwrap();

    assert!(
        paths.iter().all(|(path, _)| path != "index.js"),
        "{paths:?}"
    );
    assert!(main.contains("lazy.d601cb87.js"), "{main}");
}
