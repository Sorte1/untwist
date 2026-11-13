use std::path::{Path, PathBuf};
use std::sync::Mutex;

use untwist::{Options, Output, Pipeline};
use untwist_rename::commonjs::PASS;
use untwist_testing::node_rows::{node_or_skip, printed};

const DIR: &str = "fixtures/rollup-commonjs";
const EXPECT: &str = "fixtures/rollup-commonjs/EXPECT";
const BLESS: &str = "UNTWIST_BLESS";
const THREADS: usize = 8;
const LANES: [(&str, bool); 2] = [("default", false), ("unsafe", true)];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixtures() -> Vec<String> {
    let manifest = std::fs::read_to_string(root().join(DIR).join("MANIFEST")).expect("MANIFEST");
    manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.split('\t').next().expect("path").to_string())
        .collect()
}

fn options(unsafe_transforms: bool) -> Options {
    Options {
        unsafe_transforms,
        unpack: false,
        ..Options::default()
    }
}

fn counts(out: &Output) -> (usize, usize, usize) {
    out.notes
        .iter()
        .filter(|note| note.pass == PASS)
        .find_map(|note| {
            let mut numbers = note
                .message
                .split(|ch: char| !ch.is_ascii_digit())
                .filter(|part| !part.is_empty())
                .map(|part| part.parse::<usize>().expect("count"));
            Some((numbers.next()?, numbers.next()?, numbers.next()?))
        })
        .unwrap_or((0, 0, 0))
}

struct Row {
    line: String,
    wrong: Option<String>,
}

fn check(node: &Path, dir: &Path, path: &str) -> Vec<Row> {
    let source = std::fs::read_to_string(root().join(DIR).join(path)).expect("fixture");
    let before = printed(node, dir, &source);
    LANES
        .iter()
        .map(|&(lane, unsafe_transforms)| {
            let out = Pipeline::new(options(unsafe_transforms)).run(&source);
            let (found, helpers, named) = counts(&out);
            let after = printed(node, dir, out.code());
            let wrong = (before.is_err() || after != before).then(|| {
                format!(
                    "{path} ({lane}): node printed {before:?}, untwist's output printed {after:?}"
                )
            });
            Row {
                line: format!("{path}\t{lane}\t{found}\t{helpers}\t{named}"),
                wrong,
            }
        })
        .collect()
}

fn run_all(node: &Path) -> (Vec<String>, Vec<String>) {
    let paths = fixtures();
    let next = Mutex::new(0usize);
    let rows = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| {
                let dir = tempfile::tempdir().expect("tempdir");
                loop {
                    let index = {
                        let mut next = next.lock().unwrap();
                        *next += 1;
                        *next - 1
                    };
                    let Some(path) = paths.get(index) else {
                        break;
                    };
                    let checked = check(node, dir.path(), path);
                    rows.lock().unwrap().extend(checked);
                }
            });
        }
    });
    let rows = rows.into_inner().unwrap();
    let mut lines: Vec<String> = rows.iter().map(|row| row.line.clone()).collect();
    lines.sort();
    let wrong = rows.into_iter().filter_map(|row| row.wrong).collect();
    (lines, wrong)
}

#[test]
fn every_generated_rollup_commonjs_bundle_runs_the_same_and_is_recognised_as_expected() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let (lines, wrong) = run_all(&node);
    assert!(
        wrong.is_empty(),
        "{} fixtures diverge:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
    let text = format!(
        "# path\tlane\twrappers\thelpers\tnamed\n{}\n",
        lines.join("\n")
    );
    let expect = root().join(EXPECT);
    if std::env::var_os(BLESS).is_some() {
        std::fs::write(&expect, &text).expect("write EXPECT");
        return;
    }
    let committed = std::fs::read_to_string(&expect).unwrap_or_default();
    let changed: Vec<String> = text
        .lines()
        .filter(|line| !committed.lines().any(|known| known == *line))
        .map(str::to_string)
        .collect();
    assert!(
        changed.is_empty() && committed.lines().count() == text.lines().count(),
        "recognition moved (rerun with {BLESS}=1 to accept):\n{}",
        changed.join("\n")
    );
}
