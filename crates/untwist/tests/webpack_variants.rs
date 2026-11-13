use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const FIXTURES: &str = "../../fixtures/webpack-variants";
const ENTRY: &str = "index.js";

struct Cell {
    name: String,
    files: Vec<String>,
    expect: Expect,
}

#[derive(Debug, PartialEq, Eq)]
enum Expect {
    Runs,
    RunsWhenDeobfuscated,
    Refused(String),
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES)
}

fn expectations() -> Vec<(String, Expect)> {
    let text = std::fs::read_to_string(root().join("EXPECT")).expect("EXPECT");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next().expect("cell").to_string();
            let expect = match fields.next().expect("verdict") {
                "runs" => Expect::Runs,
                "runs-default" => Expect::RunsWhenDeobfuscated,
                "refused" => Expect::Refused(fields.next().unwrap_or_default().to_string()),
                other => panic!("{name}: unknown verdict {other}"),
            };
            (name, expect)
        })
        .collect()
}

fn cells() -> Vec<Cell> {
    let manifest = std::fs::read_to_string(root().join("MANIFEST")).expect("MANIFEST");
    let mut expect = expectations();
    let cells: Vec<Cell> = manifest
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let name = fields[0].to_string();
            let at = expect
                .iter()
                .position(|(cell, _)| *cell == name)
                .unwrap_or_else(|| panic!("{name} has no EXPECT row"));
            Cell {
                files: fields[4].split(',').map(str::to_string).collect(),
                expect: expect.remove(at).1,
                name,
            }
        })
        .collect();
    assert!(
        expect.is_empty(),
        "EXPECT rows without a cell: {:?}",
        expect.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    cells
}

fn lane(deobfuscate: bool) -> Options {
    Options {
        deobfuscate,
        unminify: deobfuscate,
        ..Options::default()
    }
}

struct Written {
    dir: tempfile::TempDir,
    notes: Vec<String>,
    bundled: bool,
}

fn unpacked(cell: &Cell, options: Options) -> Written {
    let texts: Vec<String> = cell
        .files
        .iter()
        .map(|file| std::fs::read_to_string(root().join(&cell.name).join(file)).expect("input"))
        .collect();
    let inputs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let out = Pipeline::new(options).run_many(&inputs);
    let dir = tempfile::tempdir().expect("temp dir");
    out.write_to(dir.path()).expect("write");
    Written {
        bundled: out.bundle.is_some(),
        notes: out.notes.into_iter().map(|note| note.message).collect(),
        dir,
    }
}

fn check(cell: &Cell, deobfuscate: bool, node: Option<&PathBuf>) -> Option<String> {
    let written = unpacked(cell, lane(deobfuscate));
    let lane = if deobfuscate {
        "default"
    } else {
        "unpack-only"
    };
    match &cell.expect {
        Expect::RunsWhenDeobfuscated if !deobfuscate => {
            if written.bundled && written.dir.path().join(ENTRY).exists() {
                return Some(format!(
                    "{} ({lane}): the entry was written, EXPECT says only the default lane writes it",
                    cell.name
                ));
            }
            None
        }
        Expect::Refused(reason) => {
            if written.bundled {
                return Some(format!(
                    "{} ({lane}): unpacked, EXPECT says refused ({reason})",
                    cell.name
                ));
            }
            if !written
                .notes
                .iter()
                .any(|note| note.contains(reason.as_str()))
            {
                return Some(format!(
                    "{} ({lane}): no note contains {reason:?}: {:?}",
                    cell.name, written.notes
                ));
            }
            None
        }
        Expect::Runs | Expect::RunsWhenDeobfuscated => {
            if !written.bundled {
                return Some(format!(
                    "{} ({lane}): not unpacked: {:?}",
                    cell.name, written.notes
                ));
            }
            let node = node?;
            let expected = std::fs::read_to_string(root().join(format!("{}.stdout", cell.name)))
                .expect("stdout");
            let tree = run(node, &written.dir.path().join(ENTRY)).expect("spawn");
            match tree {
                Some(tree) if tree.stdout == expected => None,
                Some(tree) => Some(format!(
                    "{} ({lane}): output differs\n-- expected\n{expected}-- actual\n{}-- stderr\n{}",
                    cell.name, tree.stdout, tree.stderr
                )),
                None => Some(format!("{} ({lane}): did not terminate", cell.name)),
            }
        }
    }
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
    }
    found
}

#[test]
fn every_webpack_variant_unpacks_and_runs_as_its_expect_row_says() {
    let node = node_or_skip();
    let failures: Vec<String> = cells()
        .iter()
        .flat_map(|cell| [false, true].map(|deobfuscate| check(cell, deobfuscate, node.as_ref())))
        .flatten()
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
