use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const FIXTURES: &str = "../../fixtures/webpack-soundness";
const ENTRY: &str = "index.js";

enum Expect {
    Runs,
    Refused(String),
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES)
}

fn cases() -> Vec<(String, Expect)> {
    let text = std::fs::read_to_string(root().join("EXPECT")).expect("EXPECT");
    text.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.splitn(3, '\t');
            let name = fields.next().expect("case").to_string();
            let expect = match fields.next().expect("verdict") {
                "runs" => Expect::Runs,
                "refused" => Expect::Refused(fields.next().expect("reason").to_string()),
                other => panic!("{name}: unknown verdict {other}"),
            };
            (name, expect)
        })
        .collect()
}

fn inputs(case: &str) -> Vec<String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join(case))
        .expect("case dir")
        .map(|entry| entry.expect("entry").path())
        .collect();
    files.sort();
    files
        .iter()
        .map(|file| std::fs::read_to_string(file).expect("input"))
        .collect()
}

fn lane(deobfuscate: bool) -> Options {
    Options {
        deobfuscate,
        unminify: deobfuscate,
        ..Options::default()
    }
}

fn check(case: &str, expect: &Expect, deobfuscate: bool, node: Option<&PathBuf>) -> Option<String> {
    let texts = inputs(case);
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let out = Pipeline::new(lane(deobfuscate)).run_many(&refs);
    let lane = if deobfuscate {
        "default"
    } else {
        "unpack-only"
    };
    let notes: Vec<String> = out.notes.iter().map(|note| note.message.clone()).collect();
    match expect {
        Expect::Refused(reason) => {
            if out.bundle.is_some() {
                return Some(format!("{case} ({lane}): unpacked, EXPECT says refused"));
            }
            (!notes.iter().any(|note| note.contains(reason.as_str())))
                .then(|| format!("{case} ({lane}): no note contains {reason:?}: {notes:?}"))
        }
        Expect::Runs => {
            if out.bundle.is_none() {
                return Some(format!("{case} ({lane}): not unpacked: {notes:?}"));
            }
            let node = node?;
            let dir = tempfile::tempdir().expect("temp dir");
            out.write_to(dir.path()).expect("write");
            let input = run(node, &root().join(case).join(ENTRY)).expect("spawn input")?;
            let tree = run(node, &dir.path().join(ENTRY)).expect("spawn tree")?;
            (tree.stdout != input.stdout).then(|| {
                format!(
                    "{case} ({lane}): output differs\n-- input\n{}-- tree\n{}-- stderr\n{}",
                    input.stdout, tree.stdout, tree.stderr
                )
            })
        }
    }
}

#[test]
fn hand_edited_webpack_inputs_are_refused_or_run_like_the_bundle() {
    let node = node();
    if node.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
    }
    let failures: Vec<String> = cases()
        .iter()
        .flat_map(|(case, expect)| {
            [false, true].map(|deobfuscate| check(case, expect, deobfuscate, node.as_ref()))
        })
        .flatten()
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
