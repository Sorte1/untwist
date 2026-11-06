pub mod blockers;
pub mod lanes;
pub mod machinery;
pub mod manifest;
pub mod oracle;
pub mod report;
pub mod suggest;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::obf_matrix::expect::{self, judge, Verdict};
use crate::obf_matrix::{pool, runner};
use lanes::{observe_untwist, LaneRun, UNTWIST_LANES};
use machinery::{measure, MACHINERY, OUTPUT_ONLY, SIMPLIFY};
use manifest::{Entry, DIR};

pub const EXPECT: &str = "fixtures/obfuscator-io/EXPECT";

#[derive(clap::Args)]
pub struct Request {
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long)]
    pub only: Option<String>,
    #[arg(
        long,
        help = "run the pinned webcrack oracle and rewrite ORACLE (full runs only)"
    )]
    pub webcrack: bool,
    #[arg(long, help = "write the per-cell baseline table to this file")]
    pub baseline: Option<PathBuf>,
    #[arg(
        long,
        help = "write EXPECT lines for the unexpected failures to this file"
    )]
    pub draft_expect: Option<PathBuf>,
    #[arg(
        long,
        help = "write a whole suggested EXPECT, every failure classified by cause, keeping each cause's reason from the current EXPECT"
    )]
    pub suggest_expect: Option<PathBuf>,
    #[arg(
        long,
        help = "write one row per safe-lane refusal (cause, note, first span, global source) to this file; needs a test-hooks build"
    )]
    pub blockers: Option<PathBuf>,
    #[arg(long, help = "also print every expected failure")]
    pub verbose: bool,
}

pub struct Outcome {
    pub entry: Entry,
    pub lanes: BTreeMap<&'static str, LaneRun>,
}

pub struct Judged {
    pub file: String,
    pub cell: String,
    pub lane: &'static str,
    pub check: &'static str,
    pub verdict: Verdict,
    pub detail: Option<String>,
    pub note: Option<String>,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn verify(root: &Path, entries: &[Entry]) -> Result<Vec<String>> {
    let mut problems = Vec::new();
    for entry in entries {
        let bytes = std::fs::read(entry.path(root))
            .with_context(|| entry.path(root).display().to_string())?;
        if sha256(&bytes) != entry.sha256 {
            problems.push(format!("{} does not match its MANIFEST sha256", entry.file));
        }
    }
    let sources: BTreeSet<&str> = entries.iter().map(|e| e.source.as_str()).collect();
    let classes: Vec<&str> = MACHINERY.iter().chain(SIMPLIFY.iter()).copied().collect();
    for source in sources {
        let text =
            std::fs::read_to_string(root.join(source)).with_context(|| source.to_string())?;
        let found = measure(&text);
        if !found.parses || !found.present(&classes).is_empty() {
            problems.push(format!(
                "the original {source} measures {} (parses {}); the meter must read it as clean",
                found.render(&classes),
                found.parses
            ));
        }
    }
    Ok(problems)
}

fn blind_detectors(root: &Path, entries: &[Entry]) -> Vec<&'static str> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let found = measure(&std::fs::read_to_string(entry.path(root)).unwrap_or_default());
        seen.extend(found.counts.keys().copied());
    }
    MACHINERY
        .iter()
        .chain(SIMPLIFY.iter())
        .copied()
        .filter(|c| !seen.contains(c) && !OUTPUT_ONLY.contains(c))
        .collect()
}

fn expected_stdouts(root: &Path, entries: &[Entry]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for entry in entries {
        if out.contains_key(entry.program()) {
            continue;
        }
        let path = entry.expected_stdout(root);
        let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
        out.insert(entry.program().to_string(), text);
    }
    Ok(out)
}

fn observe(
    root: &Path,
    request: &Request,
    entries: &[Entry],
    expected: &BTreeMap<String, String>,
    work: &Path,
) -> Result<Vec<Outcome>> {
    let mut runner = runner(root, request.untwist.as_ref(), true)?;
    if request.blockers.is_some() {
        runner.env.push((blockers::SPANS_HOOK, "1"));
    }
    if runner.node.is_none() {
        bail!("obf-options needs node: it compares each output with the original program");
    }
    let mut jobs = Vec::new();
    for index in 0..entries.len() {
        for lane in UNTWIST_LANES {
            jobs.push((index, lane));
        }
    }
    let results = pool(jobs, request.jobs, |(index, lane)| {
        let entry = &entries[index];
        let dir = work.join(format!("{index}-{lane}"));
        let want = expected.get(entry.program()).map_or("", String::as_str);
        let run = observe_untwist(&runner, entry, lane, &entry.path(root), &dir, want);
        let _ = std::fs::remove_dir_all(&dir);
        (index, lane, run)
    });
    let mut outcomes: Vec<Outcome> = entries
        .iter()
        .map(|entry| Outcome {
            entry: entry.clone(),
            lanes: BTreeMap::new(),
        })
        .collect();
    for (index, lane, run) in results {
        let run = run.with_context(|| format!("{} {lane}", entries[index].file))?;
        outcomes[index].lanes.insert(lane, run);
    }
    if request.webcrack {
        let oracle = oracle::run(root, &runner, entries, expected, work, request.jobs)?;
        for outcome in &mut outcomes {
            if let Some(run) = oracle.get(&outcome.entry.file) {
                outcome.lanes.insert(lanes::WEBCRACK, run.clone());
            }
        }
    }
    Ok(outcomes)
}

fn judge_all(rules: &[expect::Rule], outcomes: &[Outcome]) -> Vec<Judged> {
    let mut out = Vec::new();
    for outcome in outcomes {
        for lane in UNTWIST_LANES {
            let Some(run) = outcome.lanes.get(lane) else {
                continue;
            };
            for (check, failure) in run.checks() {
                let detail = failure.clone();
                out.push(Judged {
                    file: outcome.entry.file.clone(),
                    cell: outcome.entry.cell.clone(),
                    lane,
                    check,
                    verdict: judge(rules, &outcome.entry.file, lane, check, failure),
                    detail,
                    note: run.first_note().map(String::from),
                });
            }
        }
    }
    out
}

fn selected(root: &Path, request: &Request) -> Result<Vec<Entry>> {
    let entries: Vec<Entry> = manifest::load(root)?
        .into_iter()
        .filter(|e| {
            request
                .only
                .as_deref()
                .is_none_or(|only| e.file.contains(only))
        })
        .collect();
    if entries.is_empty() {
        bail!("no fixture under {DIR} matches");
    }
    if request.webcrack && request.only.is_some() {
        bail!("--webcrack rewrites ORACLE for every fixture, so it does not take --only");
    }
    Ok(entries)
}

fn harness_problems(root: &Path, request: &Request, entries: &[Entry]) -> Result<Vec<String>> {
    let mut problems = verify(root, entries)?;
    if request.only.is_none() {
        for class in blind_detectors(root, entries) {
            problems.push(format!(
                "no fixture input shows {class}, so its detector is untested"
            ));
        }
    }
    Ok(problems)
}

fn oracle_rows(
    root: &Path,
    request: &Request,
    outcomes: &[Outcome],
) -> Result<BTreeMap<String, oracle::Row>> {
    if !request.webcrack {
        return oracle::load(root);
    }
    let rows: BTreeMap<String, oracle::Row> = outcomes
        .iter()
        .filter_map(|o| {
            let run = o.lanes.get(lanes::WEBCRACK)?;
            Some((o.entry.file.clone(), oracle::row_of(run)))
        })
        .collect();
    oracle::write(root, &rows)?;
    Ok(rows)
}

fn write_outputs(
    (root, request): (&Path, &Request),
    (rules, cells): (&[expect::Rule], &[report::CellRow]),
    judged: &[Judged],
    outcomes: &[Outcome],
    oracle: &BTreeMap<String, oracle::Row>,
) -> Result<()> {
    if let Some(path) = &request.baseline {
        report::write_baseline(path, cells, judged, oracle)?;
    }
    if let Some(path) = &request.draft_expect {
        report::write_draft(path, judged, outcomes)?;
    }
    if let Some(path) = &request.suggest_expect {
        suggest::write(path, &root.join(EXPECT), rules, judged, outcomes)?;
    }
    if let Some(path) = &request.blockers {
        blockers::write(root, path, judged, outcomes)?;
    }
    Ok(())
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let started = Instant::now();
    let rules = expect::load(&root.join(EXPECT))?;
    let entries = selected(root, request)?;
    let problems = harness_problems(root, request, &entries)?;
    let expected = expected_stdouts(root, &entries)?;
    std::fs::create_dir_all(root.join("target"))?;
    let work = tempfile::Builder::new()
        .prefix("obf-options-")
        .tempdir_in(root.join("target"))?;
    let outcomes = observe(root, request, &entries, &expected, work.path())?;
    let oracle = oracle_rows(root, request, &outcomes)?;
    let judged = judge_all(&rules, &outcomes);
    let failures = report::print_verdicts(&judged, request.verbose);
    let cells = report::cells(&outcomes, &oracle);
    report::print_cells(&cells);
    report::print_causes(&judged);
    report::print_parity(&outcomes, &oracle, request.verbose);
    write_outputs(
        (root, request),
        (&rules, &cells),
        &judged,
        &outcomes,
        &oracle,
    )?;
    for problem in &problems {
        println!("harness: {problem}");
    }
    let tally = report::tally(&judged);
    println!(
        "obf-options: {} fixtures in {} cells, {} checks ok, {} expected-fail, {} failed, {} unexpected-pass, wall {:.1} s",
        entries.len(),
        cells.len(),
        tally.ok,
        tally.expected,
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
