pub mod checks;
pub mod manifest;
pub mod observe;
pub mod report;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::obf_matrix::expect::{self, judge};
use crate::obf_matrix::pool;
use crate::obf_options::machinery::Found;
use checks::{evaluate, measure_files, parse_expected};
use manifest::{files_of, Cell, DIR};
use observe::{observe, Setup, LANES};
use report::Judged;

pub const EXPECT: &str = "fixtures/bundle-matrix/EXPECT";

#[derive(clap::Args)]
pub struct Request {
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long, help = "only cells whose app/cell id contains this text")]
    pub only: Option<String>,
    #[arg(long, help = "only this lane (default, split or unsafe)")]
    pub lane: Option<String>,
    #[arg(
        long,
        help = "write a whole suggested EXPECT, keeping the reason of every rule that still matches"
    )]
    pub suggest_expect: Option<PathBuf>,
    #[arg(long, help = "also print every expected failure")]
    pub verbose: bool,
}

fn selected(root: &Path, request: &Request) -> Result<Vec<Cell>> {
    let cells: Vec<Cell> = manifest::load(root)?
        .into_iter()
        .filter(|c| request.only.as_deref().is_none_or(|o| c.id().contains(o)))
        .collect();
    if cells.is_empty() {
        bail!("no cell in {DIR}/MANIFEST matches");
    }
    Ok(cells)
}

fn lanes(request: &Request) -> Result<Vec<(&'static str, &'static [&'static str])>> {
    let lanes: Vec<_> = LANES
        .iter()
        .copied()
        .filter(|(name, _)| request.lane.as_deref().is_none_or(|l| l == *name))
        .collect();
    if lanes.is_empty() {
        bail!("no lane named {:?}", request.lane);
    }
    Ok(lanes)
}

fn bases(root: &Path, all: &[Cell], sums: &BTreeMap<String, String>) -> BTreeMap<String, Found> {
    let mut out = BTreeMap::new();
    for cell in all {
        let key = format!("{}/{}", cell.app, cell.base());
        if out.contains_key(&key) {
            continue;
        }
        let base = all
            .iter()
            .find(|c| c.app == cell.app && c.name == cell.base())
            .unwrap_or(cell);
        out.insert(key, measure_files(&files_of(root, base, sums)));
    }
    out
}

fn tools(root: &Path, request: &Request) -> Result<(PathBuf, PathBuf)> {
    let untwist = request
        .untwist
        .clone()
        .unwrap_or_else(|| root.join("target/release/untwist"));
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli");
    }
    let Some(node) = untwist_testing::node::node() else {
        bail!("bundle-matrix needs node: it runs every bundle and every output");
    };
    if !root.join("tools/bundle-matrix/node_modules").is_dir() {
        bail!("install the loaders first: npm ci --prefix tools/bundle-matrix --ignore-scripts");
    }
    Ok((untwist, node))
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let started = Instant::now();
    let rules = expect::load(&root.join(EXPECT))?;
    let all = manifest::load(root)?;
    let cells = selected(root, request)?;
    let lanes = lanes(request)?;
    let sums = manifest::sums(root)?;
    let problems = manifest::verify(root, &cells, &sums)?;
    let (untwist, node) = tools(root, request)?;
    let setup = Setup {
        untwist: &untwist,
        node: &node,
        root,
    };
    let base = bases(root, &all, &sums);
    std::fs::create_dir_all(root.join("target"))?;
    let work = tempfile::Builder::new()
        .prefix("bundle-matrix-")
        .tempdir_in(root.join("target"))?;
    let mut jobs = Vec::new();
    for index in 0..cells.len() {
        for &lane in &lanes {
            jobs.push((index, lane));
        }
    }
    jobs.reverse();
    let results = pool(jobs, request.jobs, |(index, (lane, flags))| {
        let cell = &cells[index];
        let dir = work.path().join(format!("{index}-{lane}"));
        let observed = observe(&setup, cell, &files_of(root, cell, &sums), flags, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        (index, lane, observed)
    });
    let mut judged = Vec::new();
    for (index, lane, observed) in results {
        let cell = &cells[index];
        let observed = observed.with_context(|| format!("{} {lane}", cell.id()))?;
        let path = cell.expected_stdout(root);
        let expected = parse_expected(
            &std::fs::read_to_string(&path).with_context(|| path.display().to_string())?,
        );
        let base = &base[&format!("{}/{}", cell.app, cell.base())];
        for (check, failure) in evaluate(cell, &observed, &expected, base) {
            judged.push(Judged {
                cell: cell.clone(),
                lane,
                check,
                verdict: judge(&rules, &cell.id(), lane, check, failure.clone()),
                detail: failure,
                bundle: observed.bundle.clone(),
            });
        }
    }
    let failures = report::print_verdicts(&judged, request.verbose);
    report::print_summary(&judged);
    if let Some(path) = &request.suggest_expect {
        report::write_suggestion(path, &rules, &judged)?;
    }
    for problem in &problems {
        println!("harness: {problem}");
    }
    let tally = report::tally(&judged);
    println!(
        "bundle-matrix: {} cells x {} lanes, {} checks ok, {} expected-fail, {} allowed, {} failed, {} unexpected-pass, wall {:.1} s",
        cells.len(),
        lanes.len(),
        tally.ok,
        tally.expected,
        tally.allowed,
        tally.failed,
        tally.unexpected_pass,
        started.elapsed().as_secs_f64()
    );
    if failures + problems.len() > 0 {
        bail!(
            "{failures} check(s) failed, {} harness problem(s)",
            problems.len()
        );
    }
    Ok(())
}
