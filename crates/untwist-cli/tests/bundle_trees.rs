mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use untwist_testing::node::run_with;
use untwist_testing::node_rows::node_or_skip;

const EXPECT: &str = "EXPECT";
const XFAIL: &str = "XFAIL";
const BLESS: &str = "UNTWIST_BLESS";
const THREADS: usize = 6;
type Lane = (&'static str, &'static [&'static str]);

const LANES: [Lane; 3] = [
    ("default", &[]),
    ("split", &["--split-hoisted"]),
    ("unsafe", &["--unsafe", "--split-hoisted"]),
];
const ASSUMED_LANES: [Lane; 4] = [
    ("default", &[]),
    ("split", &["--split-hoisted"]),
    ("assume", &["--split-hoisted", "--split-assume-modules"]),
    (
        "assume-unsafe",
        &["--unsafe", "--split-hoisted", "--split-assume-modules"],
    ),
];
const DOCUMENT: &str = "document.mjs";

struct Suite {
    dir: &'static str,
    lanes: &'static [Lane],
    preload: Option<&'static str>,
}

const ROLLDOWN: Suite = Suite {
    dir: "fixtures/unpack-rolldown",
    lanes: &LANES,
    preload: None,
};
const ROLLUP_ES: Suite = Suite {
    dir: "fixtures/unpack-rollup-es",
    lanes: &LANES,
    preload: None,
};
const ROLLUP_ASSUMED: Suite = Suite {
    dir: "fixtures/unpack-rollup-assume",
    lanes: &ASSUMED_LANES,
    preload: Some(DOCUMENT),
};

const ROLLUP_BOUNDARIES: Suite = Suite {
    dir: "fixtures/unpack-rollup-boundaries",
    lanes: &ASSUMED_LANES,
    preload: None,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cells(dir: &str) -> BTreeMap<String, Vec<String>> {
    let manifest = std::fs::read_to_string(root().join(dir).join("MANIFEST")).expect("MANIFEST");
    let mut cells: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let path = line.split('\t').next().expect("path");
        let (cell, file) = path.split_once('/').expect("cell/file");
        let files = cells.entry(cell.to_string()).or_default();
        match file == "index.js" {
            true => files.insert(0, file.to_string()),
            false => files.push(file.to_string()),
        }
    }
    cells
}

struct Written {
    kind: String,
    modules: usize,
    entry: Option<String>,
}

fn bundle(out: &Path) -> Written {
    let Ok(text) = std::fs::read_to_string(out.join("bundle.json")) else {
        return Written {
            kind: "none".to_string(),
            modules: 0,
            entry: None,
        };
    };
    let json: serde_json::Value = serde_json::from_str(&text).expect("bundle.json");
    let modules = json["modules"].as_array().cloned().unwrap_or_default();
    let entry = modules
        .iter()
        .find(|module| module["isEntry"].as_bool() == Some(true))
        .and_then(|module| module["path"].as_str())
        .map(str::to_string);
    Written {
        kind: json["type"].as_str().unwrap_or("?").to_string(),
        modules: modules.len(),
        entry,
    }
}

fn preload_args(suite: &Suite) -> Vec<std::ffi::OsString> {
    let Some(file) = suite.preload else {
        return Vec::new();
    };
    let path = root().join(suite.dir).join(file);
    let path = path.canonicalize().expect("preload file");
    let url = format!("file://{}", path.display());
    vec!["--import".into(), url.into()]
}

fn row(
    suite: &Suite,
    node: &Path,
    work: &Path,
    cell: &str,
    files: &[String],
    (lane, flags): Lane,
) -> String {
    let dir = suite.dir;
    let source = root().join(dir).join(cell);
    let out = work.join(format!("{cell}-{lane}"));
    let inputs: Vec<PathBuf> = files.iter().map(|file| source.join(file)).collect();
    let status = common::bin()
        .args(flags)
        .arg("-f")
        .arg("-o")
        .arg(&out)
        .args(&inputs)
        .output()
        .expect("run untwist");
    assert!(
        status.status.success(),
        "{cell} ({lane}): untwist failed: {}",
        String::from_utf8_lossy(&status.stderr)
    );
    let written = bundle(&out);
    let expected = std::fs::read_to_string(
        root()
            .join(dir)
            .join("expected")
            .join(format!("{cell}.stdout")),
    )
    .expect("expected stdout");
    let entry = match (&written.entry, out.join("index.js").exists()) {
        (Some(path), _) => out.join(path),
        (None, true) => out.join("index.js"),
        (None, false) => out.join("deobfuscated.js"),
    };
    let preload = preload_args(suite);
    let args: Vec<&std::ffi::OsStr> = preload.iter().map(|arg| arg.as_os_str()).collect();
    let observed = run_with(node, &args, &entry).expect("run node");
    let same = observed.is_some_and(|o| o.exit == Some(0) && o.stdout == expected);
    let verdict = if same { "same" } else { "differs" };
    format!(
        "{cell}\t{lane}\t{}\t{}\t{verdict}",
        written.kind, written.modules
    )
}

fn rows(suite: &Suite, node: &Path, work: &Path) -> Vec<String> {
    let cells: Vec<(String, Vec<String>)> = cells(suite.dir).into_iter().collect();
    let lanes = suite.lanes.len();
    let jobs: Vec<(usize, usize)> = (0..cells.len())
        .flat_map(|cell| (0..lanes).map(move |lane| (cell, lane)))
        .collect();
    let next = Mutex::new(0usize);
    let found = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..THREADS {
            scope.spawn(|| loop {
                let index = {
                    let mut next = next.lock().unwrap();
                    *next += 1;
                    *next - 1
                };
                let Some(&(cell, lane)) = jobs.get(index) else {
                    break;
                };
                let (name, files) = &cells[cell];
                let line = row(suite, node, work, name, files, suite.lanes[lane]);
                found.lock().unwrap().push(line);
            });
        }
    });
    let mut lines = found.into_inner().unwrap();
    lines.sort();
    lines
}

fn xfails(dir: &str) -> Vec<String> {
    std::fs::read_to_string(root().join(dir).join(XFAIL))
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .filter_map(|line| line.split('\t').next().map(str::to_string))
        .collect()
}

fn glob(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((head, rest)) => {
            let Some(tail) = text.strip_prefix(head) else {
                return false;
            };
            (0..=tail.len()).any(|at| tail.is_char_boundary(at) && glob(rest, &tail[at..]))
        }
    }
}

fn listed(xfails: &[String], cell: &str) -> bool {
    xfails.iter().any(|pattern| glob(pattern, cell))
}

fn check(suite: &Suite) -> Option<tempfile::TempDir> {
    let node = node_or_skip()?;
    let work = tempfile::tempdir().expect("tempdir");
    let dir = suite.dir;
    let lines = rows(suite, &node, work.path());
    let xfails = xfails(dir);
    let wrong: Vec<&String> = lines
        .iter()
        .filter(|line| line.ends_with("\tdiffers"))
        .filter(|line| {
            let mut fields = line.split('\t');
            let cell = fields.next().unwrap_or_default();
            let lane = fields.next().unwrap_or_default();
            !listed(&xfails, &format!("{cell}/{lane}"))
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "written trees that do not run like the bundle:\n{}",
        wrong
            .iter()
            .map(|line| line.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    );
    let text = format!(
        "# cell\tlane\tbundle\tmodules\tnode\n{}\n",
        lines.join("\n")
    );
    let expect = root().join(dir).join(EXPECT);
    if std::env::var_os(BLESS).is_some() {
        std::fs::write(&expect, &text).expect("write EXPECT");
        return Some(work);
    }
    let committed = std::fs::read_to_string(&expect).unwrap_or_default();
    let changed: Vec<&str> = text
        .lines()
        .filter(|line| !committed.lines().any(|known| known == *line))
        .collect();
    assert!(
        changed.is_empty() && committed.lines().count() == text.lines().count(),
        "recognition moved (rerun with {BLESS}=1 to accept):\n{}",
        changed.join("\n")
    );
    Some(work)
}

#[test]
fn every_generated_rolldown_bundle_unpacks_into_a_tree_that_runs_the_same() {
    check(&ROLLDOWN);
}

#[test]
fn every_generated_rollup_es_bundle_unpacks_into_a_tree_that_runs_the_same() {
    check(&ROLLUP_ES);
}

fn written(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .expect("output dir")
        .map(|entry| entry.expect("entry").path())
        .map(|path| {
            let name = path
                .file_name()
                .expect("name")
                .to_string_lossy()
                .into_owned();
            (name, std::fs::read(&path).expect("read output"))
        })
        .collect()
}

#[test]
fn every_generated_rollup_iife_bundle_splits_under_assumed_modules_into_a_tree_that_runs_the_same()
{
    let Some(work) = check(&ROLLUP_ASSUMED) else {
        return;
    };
    for cell in cells(ROLLUP_ASSUMED.dir).keys() {
        for lane in ["default", "split"] {
            let files = written(&work.path().join(format!("{cell}-{lane}")));
            assert_eq!(
                files.keys().collect::<Vec<_>>(),
                ["deobfuscated.js"],
                "{cell}: the {lane} lane must not split without --split-assume-modules"
            );
        }
    }
}

#[test]
fn every_generated_rollup_app_splits_into_inferred_modules_that_run_the_same() {
    check(&ROLLUP_BOUNDARIES);
}
