use std::path::{Path, PathBuf};
use std::sync::Mutex;

use untwist::{Options, Pipeline};
use xtask::obf_matrix::expect::{judge, load, Rule};
use xtask::obf_matrix::residue::measure;
use xtask::obf_matrix::EXPECT;

const DIR: &str = "fixtures/real-shapes";
const SLOW: &str = "game-9.2-base64-rc4.js";
const LANES: [&str; 2] = ["safe", "unsafe"];
const STACK_BYTES: usize = 64 << 20;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn shapes(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join(DIR))
        .expect("real-shapes")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".js"))
        .collect();
    names.sort();
    names
}

fn jobs(names: &[String]) -> Vec<(&str, &'static str)> {
    let mut jobs: Vec<(&str, &'static str)> = names
        .iter()
        .flat_map(|name| LANES.map(|lane| (name.as_str(), lane)))
        .collect();
    jobs.sort_by_key(|(name, _)| *name == SLOW);
    jobs
}

fn checks(code: &str) -> [(&'static str, Option<String>); 2] {
    let found = measure(code);
    [
        (
            "runs",
            (!found.parses).then(|| "output does not parse".to_string()),
        ),
        (
            "decoded",
            (!found.residue.is_clean()).then(|| format!("residue {}", found.residue.render())),
        ),
    ]
}

fn drift(root: &Path, rules: &[Rule], name: &str, lane: &str) -> Vec<String> {
    let source = std::fs::read_to_string(root.join(DIR).join(name)).expect("fixture");
    let options = Options {
        unsafe_transforms: lane == "unsafe",
        ..Options::default()
    };
    let output = Pipeline::new(options).run(&source);
    let fixture = format!("real-shapes/{name}");
    let cell = format!("{lane}/none/stdout");
    checks(output.code())
        .into_iter()
        .filter_map(|(check, failure)| {
            let detail = failure.clone().unwrap_or_default();
            let verdict = judge(rules, &fixture, &cell, check, failure);
            verdict
                .is_failure()
                .then(|| format!("{fixture} {cell} {check} {} {detail}", verdict.tag()))
        })
        .collect()
}

#[test]
fn every_real_shape_matches_expect_in_both_lanes_without_node() {
    let root = root();
    let rules = load(&root.join(EXPECT)).expect("EXPECT");
    let names = shapes(&root);
    assert!(names.iter().any(|name| name == SLOW), "{SLOW} is missing");
    let jobs = Mutex::new(jobs(&names));
    let problems = Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .max(LANES.len());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            std::thread::Builder::new()
                .stack_size(STACK_BYTES)
                .spawn_scoped(scope, || loop {
                    let Some((name, lane)) = jobs.lock().expect("jobs").pop() else {
                        break;
                    };
                    let found = drift(&root, &rules, name, lane);
                    problems.lock().expect("problems").extend(found);
                })
                .expect("spawn");
        }
    });
    let mut problems = problems.into_inner().expect("problems");
    problems.sort();
    assert!(
        problems.is_empty(),
        "fixtures/real-shapes drifted from fixtures/obfuscated/EXPECT; run cargo xtask obf-matrix:\n{}",
        problems.join("\n")
    );
}
