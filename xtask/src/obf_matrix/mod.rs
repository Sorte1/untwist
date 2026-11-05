pub mod cell;
pub mod checks;
pub mod expect;
pub mod notes;
pub mod residue;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use cell::{all_cells, Behaviour, Cell, Observation, Ran, Runner};
use checks::{evaluate, Observations, Subject};
use expect::{judge, Rule, Verdict};

pub const EXPECT: &str = "fixtures/obfuscated/EXPECT";
const TIMEOUT: Duration = Duration::from_secs(300);
const REALISTIC: [&str; 2] = ["fixtures/obfuscated", "fixtures/real-shapes"];
const FAMILY: [(&str, &str); 3] = [
    ("fixtures/reference/deobfuscate", "obfuscator.io"),
    ("fixtures/corpus-npm", "case-"),
    ("fixtures/deobfuscate-gaps", ""),
];

#[derive(clap::Args)]
pub struct Request {
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long)]
    pub only: Option<String>,
    #[arg(long)]
    pub metrics: Option<PathBuf>,
}

pub struct Fixture {
    pub name: String,
    pub path: PathBuf,
    pub realistic: bool,
    pub expected_stdout: Option<PathBuf>,
}

impl Fixture {
    fn is_bundle(&self) -> bool {
        let name = self.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.starts_with("webpack")
    }
}

fn listed(dir: &Path, prefix: &str) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_file() && name.ends_with(".js") && name.starts_with(prefix) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

pub fn fixtures(root: &Path) -> Result<Vec<Fixture>> {
    let mut out = Vec::new();
    for dir in REALISTIC {
        for path in listed(&root.join(dir), "")? {
            let name = format!(
                "{dir}/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            let expected = path
                .parent()
                .map(|p| {
                    p.join("expected").join(format!(
                        "{}.stdout",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ))
                })
                .filter(|p| p.is_file());
            out.push(Fixture {
                name: name.trim_start_matches("fixtures/").to_string(),
                path,
                realistic: true,
                expected_stdout: expected,
            });
        }
    }
    for (dir, prefix) in FAMILY {
        for path in listed(&root.join(dir), prefix)? {
            let name = format!(
                "{dir}/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            out.push(Fixture {
                name: name.trim_start_matches("fixtures/").to_string(),
                path,
                realistic: false,
                expected_stdout: None,
            });
        }
    }
    Ok(out)
}

pub fn pool<T: Send, R: Send>(items: Vec<T>, jobs: usize, work: impl Fn(T) -> R + Sync) -> Vec<R> {
    let queue = Mutex::new(items.into_iter().enumerate().collect::<Vec<_>>());
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| loop {
                let next = queue.lock().ok().and_then(|mut q| q.pop());
                let Some((index, item)) = next else {
                    break;
                };
                let result = work(item);
                if let Ok(mut r) = results.lock() {
                    r.push((index, result));
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap_or_default();
    results.sort_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, r)| r).collect()
}

pub fn observe_all(
    runner: &Runner,
    inputs: &[(String, PathBuf)],
    jobs: usize,
    work: &Path,
    behaviour: bool,
) -> BTreeMap<String, Observations> {
    let mut jobs_list = Vec::new();
    for (index, (name, path)) in inputs.iter().enumerate() {
        for cell in all_cells() {
            jobs_list.push((index, name.clone(), path.clone(), cell));
        }
    }
    jobs_list.reverse();
    let results = pool(jobs_list, jobs, |(index, name, path, cell)| {
        let dir = work
            .join(format!("{index}"))
            .join(cell.label().replace('/', "-"));
        let observed = runner.observe(&path, cell, &dir, behaviour);
        let _ = std::fs::remove_dir_all(&dir);
        (name, cell, observed)
    });
    let mut out: BTreeMap<String, Observations> = BTreeMap::new();
    for (name, cell, observed) in results {
        match observed {
            Ok(o) => {
                out.entry(name).or_default().insert(cell, o);
            }
            Err(error) => println!("cell {name} {} ERROR {error:#}", cell.label()),
        }
    }
    out
}

#[derive(Default)]
pub struct Tally {
    pub ok: usize,
    pub failed: usize,
    pub expected: usize,
    pub unexpected_pass: usize,
    pub allowed: usize,
    pub red_by_reason: BTreeMap<String, BTreeSet<String>>,
}

impl Tally {
    pub fn record(
        &mut self,
        fixture: &str,
        cell: Cell,
        check: &str,
        verdict: &Verdict,
        detail: Option<&str>,
    ) {
        match verdict {
            Verdict::Pass => {
                self.ok += 1;
                return;
            }
            Verdict::Fail(_) => self.failed += 1,
            Verdict::ExpectedFail(reason) => {
                self.expected += 1;
                self.red_by_reason
                    .entry(reason.clone())
                    .or_default()
                    .insert(format!("{fixture} {} {check}", cell.label()));
            }
            Verdict::UnexpectedPass(_) => self.unexpected_pass += 1,
            Verdict::Allowed(_) => self.allowed += 1,
        }
        let text = match verdict {
            Verdict::Fail(d) => d.clone(),
            Verdict::ExpectedFail(r) | Verdict::UnexpectedPass(r) | Verdict::Allowed(r) => {
                format!("[{r}] {}", detail.unwrap_or(""))
            }
            Verdict::Pass => String::new(),
        };
        println!(
            "cell {fixture} {} {check} {} {text}",
            cell.label(),
            verdict.tag()
        );
    }

    pub fn failures(&self) -> usize {
        self.failed + self.unexpected_pass
    }
}

pub fn judge_all(rules: &[Rule], name: &str, subject: &Subject, tally: &mut Tally) {
    for (cell, check, failure) in evaluate(subject) {
        let detail = failure.clone();
        let verdict = judge(rules, name, &cell.label(), check, failure);
        tally.record(name, cell, check, &verdict, detail.as_deref());
    }
    for (cell, o) in subject.cells {
        if let Behaviour::Skipped(reason) = o.behaviour {
            tally.record(
                name,
                *cell,
                "node",
                &Verdict::Allowed(reason.to_string()),
                None,
            );
        }
    }
}

pub fn warning_summary(all: &BTreeMap<String, Observations>) -> Vec<(String, usize, usize, usize)> {
    let mut by_class: BTreeMap<String, (BTreeSet<String>, usize, usize)> = BTreeMap::new();
    for (name, cells) in all {
        for o in cells.values() {
            for (class, n) in &o.warnings.classes {
                let entry = by_class.entry(class.clone()).or_default();
                entry.0.insert(name.clone());
                entry.1 += 1;
                entry.2 += n;
            }
            if o.warnings.errors > 0 {
                let entry = by_class
                    .entry("error: exit or panic".to_string())
                    .or_default();
                entry.0.insert(name.clone());
                entry.1 += 1;
                entry.2 += o.warnings.errors;
            }
        }
    }
    let mut rows: Vec<(String, usize, usize, usize)> = by_class
        .into_iter()
        .map(|(class, (files, cells, notes))| (class, files.len(), cells, notes))
        .collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)).then(a.0.cmp(&b.0)));
    rows
}

pub fn print_summary(rows: &[(String, usize, usize, usize)]) {
    println!("warn-summary rank\tfiles\tcells\tnotes\tid\tclass");
    for (rank, (class, files, cells, notes)) in rows.iter().enumerate() {
        println!(
            "warn-summary {}\t{files}\t{cells}\t{notes}\t{}\t{class}",
            rank + 1,
            notes::class_id(class)
        );
    }
}

pub fn metric_line(name: &str, cell: Cell, o: &Observation) -> String {
    let side = |m: Option<residue::Measured>| m.map_or("-".to_string(), |m| m.residue.render());
    format!(
        "{name}\t{}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        cell.label(),
        o.exit,
        o.wall_ms,
        o.rss_kb.map_or("-".to_string(), |r| r.to_string()),
        side(o.program),
        side(o.modules),
        o.module_set.len(),
        o.bundle,
        o.warnings.total(),
        notes::render_ids(&o.warnings.classes)
    )
}

pub fn runner(root: &Path, untwist: Option<&PathBuf>, node: bool) -> Result<Runner> {
    let untwist = untwist
        .cloned()
        .unwrap_or_else(|| root.join("target/release/untwist"));
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let node = if node {
        let found = untwist_testing::node::node();
        if found.is_none() {
            let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
            if !untwist_testing::node::skip_allowed(required.as_deref()) {
                bail!("required: {}", untwist_testing::node::SKIP_MESSAGE);
            }
        }
        found
    } else {
        None
    };
    Ok(Runner {
        untwist,
        node,
        root: root.to_path_buf(),
        timeout: TIMEOUT,
        env: Vec::new(),
    })
}

fn expected_of(runner: &Runner, fixture: &Fixture) -> Result<Option<Ran>> {
    if let Some(path) = &fixture.expected_stdout {
        let stdout = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
        return Ok(Some(Ran {
            exit: Some(0),
            stdout,
        }));
    }
    runner.run_node(&fixture.path)
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let started = Instant::now();
    let runner = runner(root, request.untwist.as_ref(), true)?;
    let rules = expect::load(&root.join(EXPECT))?;
    let all: Vec<Fixture> = fixtures(root)?
        .into_iter()
        .filter(|f| {
            request
                .only
                .as_deref()
                .is_none_or(|only| f.name.contains(only))
        })
        .collect();
    if all.is_empty() {
        bail!("no fixture matches");
    }
    std::fs::create_dir_all(root.join("target"))?;
    let work = tempfile::Builder::new()
        .prefix("obf-matrix-")
        .tempdir_in(root.join("target"))?;
    let check_node = runner.node.is_some();
    let inputs: Vec<(String, PathBuf)> = all
        .iter()
        .map(|f| (f.name.clone(), f.path.clone()))
        .collect();
    let observed = observe_all(&runner, &inputs, request.jobs, work.path(), check_node);
    let mut tally = Tally::default();
    let mut lines = Vec::new();
    for fixture in &all {
        let Some(cells) = observed.get(&fixture.name) else {
            tally.failed += 1;
            println!("fixture {} has no observed cell", fixture.name);
            continue;
        };
        let expected = if check_node {
            Some(expected_of(&runner, fixture)?)
        } else {
            None
        };
        let subject = Subject {
            cells,
            expected: expected.as_ref(),
            realistic: fixture.realistic,
            bundle: fixture.is_bundle(),
        };
        judge_all(&rules, &fixture.name, &subject, &mut tally);
        lines.extend(
            cells
                .iter()
                .map(|(cell, o)| metric_line(&fixture.name, *cell, o)),
        );
    }
    if let Some(path) = &request.metrics {
        std::fs::write(path, lines.join("\n") + "\n")
            .with_context(|| path.display().to_string())?;
    }
    print_summary(&warning_summary(&observed));
    for (reason, cells) in &tally.red_by_reason {
        println!("expected-fail {reason}: {} check(s)", cells.len());
    }
    println!(
        "obf-matrix: {} fixtures, {} cells, {} checks ok, {} expected-fail, {} allowed, {} failed, {} unexpected-pass, node {}, wall {:.1} s",
        all.len(),
        observed.values().map(|c| c.len()).sum::<usize>(),
        tally.ok,
        tally.expected,
        tally.allowed,
        tally.failed,
        tally.unexpected_pass,
        if check_node { "on" } else { "skipped" },
        started.elapsed().as_secs_f64()
    );
    if tally.failures() > 0 {
        bail!("{} check(s) failed", tally.failures());
    }
    Ok(())
}
