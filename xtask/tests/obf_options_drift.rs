use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use untwist::{Options, Pipeline};
use xtask::obf_matrix::expect::{judge, load, Rule};
use xtask::obf_options::machinery::{measure, MACHINERY, SIMPLIFY};
use xtask::obf_options::manifest::{self, Entry, DIR};
use xtask::obf_options::VERSIONS_DIR;

const PROGRAMS: [&str; 8] = [
    "empty-log",
    "greeting",
    "functions",
    "objects",
    "classes",
    "control",
    "strings",
    "mixed",
];
const LANES: [&str; 2] = ["safe", "unsafe"];
const STACK_BYTES: usize = 64 << 20;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn sample(entries: Vec<Entry>) -> Vec<Entry> {
    let mut cells = Vec::new();
    for entry in &entries {
        if !cells.contains(&entry.cell) {
            cells.push(entry.cell.clone());
        }
    }
    let wanted: BTreeSet<String> = cells
        .iter()
        .enumerate()
        .map(|(index, cell)| format!("{cell}--{}-1.js", PROGRAMS[index % PROGRAMS.len()]))
        .collect();
    entries
        .into_iter()
        .filter(|entry| wanted.contains(&entry.file))
        .collect()
}

fn checks(code: &str) -> [(&'static str, Option<String>); 3] {
    let found = measure(code);
    let present = |classes: &[&'static str]| {
        (!found.present(classes).is_empty()).then(|| found.render(classes))
    };
    [
        (
            "runs",
            (!found.parses).then(|| "output does not parse".to_string()),
        ),
        ("machinery", present(&MACHINERY)),
        ("simplified", present(&SIMPLIFY)),
    ]
}

fn drift(root: &Path, rules: &[Rule], entry: &Entry, lane: &str) -> Vec<String> {
    let source = std::fs::read_to_string(entry.path(root)).expect("fixture");
    let options = Options {
        unsafe_transforms: lane == "unsafe",
        ..Options::default()
    };
    let output = Pipeline::new(options).run(&source);
    checks(output.code())
        .into_iter()
        .filter_map(|(check, failure)| {
            let detail = failure.clone().unwrap_or_default();
            let verdict = judge(rules, &entry.file, lane, check, failure);
            verdict
                .is_failure()
                .then(|| format!("{} {lane} {check} {} {detail}", entry.file, verdict.tag()))
        })
        .collect()
}

fn problems_in(root: &Path, dir: &str, entries: &[Entry]) -> Vec<String> {
    let rules = load(&root.join(dir).join("EXPECT")).expect("EXPECT");
    let jobs: Mutex<Vec<(&Entry, &str)>> = Mutex::new(
        entries
            .iter()
            .flat_map(|entry| LANES.map(|lane| (entry, lane)))
            .collect(),
    );
    let problems = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            std::thread::Builder::new()
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, || loop {
                    let Some((entry, lane)) = jobs.lock().expect("jobs").pop() else {
                        break;
                    };
                    let found = drift(root, &rules, entry, lane);
                    problems.lock().expect("problems").extend(found);
                })
                .expect("spawn");
        }
    });
    let mut problems = problems.into_inner().expect("problems");
    problems.sort();
    problems
}

#[test]
fn a_sample_of_the_option_matrix_matches_expect_without_node() {
    let root = root();
    let entries = sample(manifest::load(&root).expect("MANIFEST"));
    assert!(entries.len() >= 100, "{} sampled fixtures", entries.len());
    let problems = problems_in(&root, DIR, &entries);
    assert!(
        problems.is_empty(),
        "{DIR} drifted from EXPECT; run cargo xtask obf-options:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_version_matrix_matches_expect_without_node() {
    let root = root();
    let entries = manifest::load_dir(&root, VERSIONS_DIR).expect("MANIFEST");
    assert!(entries.len() >= 500, "{} fixtures", entries.len());
    let problems = problems_in(&root, VERSIONS_DIR, &entries);
    assert!(
        problems.is_empty(),
        "{VERSIONS_DIR} drifted from EXPECT; run cargo xtask obf-options --dir {VERSIONS_DIR}:\n{}",
        problems.join("\n")
    );
}
