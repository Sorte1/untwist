use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const LANES: [(&str, bool); 2] = [("safe", false), ("unsafe", true)];

const RESIDUE: [&str; 7] = ["ﾟ", "___", "[][", "![]", "+[]", "eval(", "Function("];

struct Row {
    file: String,
    sha256: String,
}

struct Rule {
    fixture: String,
    lane: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/encoders")
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("MANIFEST")).expect("read MANIFEST");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [file, _cell, _source, _seed, _tool, _behaviour, _options, sha256] = fields[..]
            else {
                panic!("MANIFEST line has {} fields: {line}", fields.len());
            };
            Row {
                file: file.to_string(),
                sha256: sha256.to_string(),
            }
        })
        .collect()
}

fn machinery_xfails() -> Vec<Rule> {
    let text = std::fs::read_to_string(root().join("EXPECT")).expect("read EXPECT");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            let [fixture, lane, check, verdict, ..] = fields[..] else {
                panic!("EXPECT line has {} fields: {line}", fields.len());
            };
            (check == "machinery" && verdict == "xfail").then(|| Rule {
                fixture: fixture.to_string(),
                lane: lane.to_string(),
            })
        })
        .collect()
}

fn glob(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, tail)) => {
            text.len() >= head.len() + tail.len() && text.starts_with(head) && text.ends_with(tail)
        }
    }
}

fn source(file: &str) -> String {
    std::fs::read_to_string(root().join(file)).unwrap_or_else(|err| panic!("{file}: {err}"))
}

fn transformed(unsafe_transforms: bool, text: &str) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(text).code().to_string()
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn the_manifest_lists_every_encoded_fixture_with_its_hash() {
    let rows = rows();
    let mut found: Vec<String> = std::fs::read_dir(root())
        .expect("read fixtures")
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".js"))
        .collect();
    found.sort();
    let mut listed: Vec<String> = rows.iter().map(|row| row.file.clone()).collect();
    listed.sort();
    assert_eq!(listed, found, "MANIFEST and fixtures/encoders disagree");
    for row in &rows {
        assert_eq!(
            hex(source(&row.file).as_bytes()),
            row.sha256,
            "{}",
            row.file
        );
    }
}

#[test]
fn each_encoded_fixture_that_expect_does_not_keep_is_decoded() {
    let xfails = machinery_xfails();
    let mut wrong = Vec::new();
    for row in rows() {
        let text = source(&row.file);
        for (lane, unsafe_transforms) in LANES {
            let kept = xfails
                .iter()
                .any(|rule| glob(&rule.fixture, &row.file) && glob(&rule.lane, lane));
            if kept {
                continue;
            }
            let code = transformed(unsafe_transforms, &text);
            if let Some(mark) = RESIDUE.iter().find(|mark| code.contains(*mark)) {
                wrong.push(format!("{} ({lane}) still holds {mark}", row.file));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn each_encoded_fixture_runs_like_node_after_untwist_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let programs: Vec<(String, String)> = rows()
        .iter()
        .map(|row| (row.file.clone(), source(&row.file)))
        .collect();
    let run_safe = |text: &str| transformed(false, text);
    let run_unsafe = |text: &str| transformed(true, text);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("unsafe", &run_unsafe)];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
