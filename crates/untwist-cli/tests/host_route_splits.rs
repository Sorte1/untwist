mod common;

use std::path::{Path, PathBuf};

use untwist_testing::node::run;
use untwist_testing::node_rows::node_or_skip;

const DEFAULT: &[&str] = &[];
const UNSAFE: &[&str] = &["--unsafe"];
const LANES: [&[&str]; 2] = [DEFAULT, UNSAFE];
const CASES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/host_routes_split");
const ENTRY: &str = "index.js";
const LISTING: &str = "bundle.json";
const WITHOUT_RELATIVE_IMPORT: &str = "im1";

struct Split {
    case: &'static str,
    inputs: &'static [&'static str],
    links: &'static [(&'static str, &'static str)],
    renames: &'static [(&'static str, &'static str)],
    dirs: &'static [&'static str],
    unsafe_premise: bool,
    unpacks_unsafe: bool,
}

const fn split(case: &'static str, inputs: &'static [&'static str]) -> Split {
    Split {
        case,
        inputs,
        links: &[],
        renames: &[],
        dirs: &[],
        unsafe_premise: false,
        unpacks_unsafe: false,
    }
}

const fn premise(case: &'static str, inputs: &'static [&'static str]) -> Split {
    Split {
        unsafe_premise: true,
        ..split(case, inputs)
    }
}

const CHUNK: [&str; 2] = [ENTRY, "lazy.chunk.js"];

const SPLITS: [Split; 14] = [
    split("back1", &CHUNK),
    split("back2", &[ENTRY, "other.js"]),
    split("back3", &[ENTRY, "shared.chunk.js"]),
    Split {
        unpacks_unsafe: true,
        ..split("base", &CHUNK)
    },
    split("dir1", &[ENTRY, "lazy.js", "app/lazy.js"]),
    premise("im1", &[ENTRY, "other.js"]),
    split("sev1", &CHUNK),
    Split {
        links: &[(ENTRY, "real/bundle.js")],
        ..premise("sym1", &CHUNK)
    },
    Split {
        links: &[("alias.js", "lazy.chunk.js")],
        ..premise("sym2", &[ENTRY, "lazy.chunk.js", "alias.js"])
    },
    premise("tr1", &CHUNK),
    premise("tr2", &CHUNK),
    Split {
        dirs: &["sub"],
        ..split("two1", &[ENTRY, "lazy.chunk.js", "sub/../lazy.chunk.js"])
    },
    Split {
        renames: &[("l6azy.js", "%6Cazy.js")],
        ..split("url1", &[ENTRY, "%6Cazy.js"])
    },
    Split {
        renames: &[("lazy.js-x", "lazy.js?x")],
        ..split("url2", &[ENTRY, "lazy.js?x"])
    },
];

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("case dir");
    for entry in std::fs::read_dir(from).expect("case files") {
        let entry = entry.expect("entry");
        let target = to.join(entry.file_name());
        match entry.file_type().expect("file type").is_dir() {
            true => copy_tree(&entry.path(), &target),
            false => {
                std::fs::copy(entry.path(), target).expect("copy case file");
            }
        }
    }
}

fn staged(row: &Split, work: &Path) -> PathBuf {
    let dir = work.join(row.case);
    copy_tree(&Path::new(CASES).join(row.case), &dir);
    for (from, to) in row.renames {
        std::fs::rename(dir.join(from), dir.join(to)).expect("rename case file");
    }
    for (link, target) in row.links {
        std::os::unix::fs::symlink(target, dir.join(link)).expect("link case file");
    }
    for sub in row.dirs {
        std::fs::create_dir_all(dir.join(sub)).expect("case subdir");
    }
    dir
}

fn observed(node: &Path, dir: &Path, file: &str) -> Option<String> {
    run(node, &dir.join(file))
        .expect("run node")
        .map(|observed| format!("{:?}\n{}", observed.exit, observed.stdout))
}

fn entry_of(out: &Path) -> String {
    let text = std::fs::read_to_string(out.join(LISTING)).expect("bundle listing");
    let listing: serde_json::Value = serde_json::from_str(&text).expect("listing json");
    listing["modules"]
        .as_array()
        .and_then(|modules| modules.iter().find(|module| module["isEntry"] == true))
        .and_then(|module| module["path"].as_str())
        .expect("an entry module")
        .to_string()
}

fn lane_failures(node: &Path, row: &Split, dir: &Path, work: &Path) -> Vec<String> {
    let expected = observed(node, dir, ENTRY);
    let mut wrong = Vec::new();
    for (lane, flags) in LANES.iter().enumerate() {
        let out = work.join(format!("{}-out-{lane}", row.case));
        let status = common::bin()
            .current_dir(dir)
            .args(*flags)
            .args(["-v", "-f", "-o"])
            .arg(&out)
            .args(row.inputs)
            .output()
            .expect("run untwist");
        if !status.status.success() {
            wrong.push(format!("{} {flags:?}: untwist failed", row.case));
            continue;
        }
        let unpacked = out.join(LISTING).exists();
        let is_unsafe = *flags == UNSAFE;
        if is_unsafe && row.unpacks_unsafe && !unpacked {
            wrong.push(format!("{} {flags:?}: no bundle written", row.case));
        }
        if !unpacked || (is_unsafe && row.unsafe_premise) {
            continue;
        }
        let seen = observed(node, &out, &entry_of(&out));
        if seen != expected {
            wrong.push(format!(
                "{} {flags:?}: inputs {expected:?}, tree {seen:?}",
                row.case
            ));
        }
    }
    wrong
}

#[test]
fn split_probes_run_like_their_inputs_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let work = tempfile::tempdir().expect("tempdir");
    let wrong: Vec<String> = SPLITS
        .iter()
        .flat_map(|row| {
            let dir = staged(row, work.path());
            lane_failures(&node, row, &dir, work.path())
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_relative_import_refuses_every_split_in_the_default_lane() {
    let work = tempfile::tempdir().expect("tempdir");
    for row in SPLITS
        .iter()
        .filter(|row| row.case != WITHOUT_RELATIVE_IMPORT)
    {
        let dir = staged(row, work.path());
        let out = work.path().join(format!("{}-default", row.case));
        let status = common::bin()
            .current_dir(&dir)
            .args(["-v", "-f", "-o"])
            .arg(&out)
            .args(row.inputs)
            .output()
            .expect("run untwist");
        assert!(status.status.success(), "{}: untwist failed", row.case);
        assert!(!out.join(LISTING).exists(), "{}: split", row.case);
    }
}
