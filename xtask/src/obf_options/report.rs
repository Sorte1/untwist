use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};

use super::lanes::{note_classes, CHECKS, SAFE, UNSAFE, UNTWIST_LANES};
use super::oracle::Row;
use super::{Judged, Outcome};
use crate::obf_matrix::expect::Verdict;

#[derive(Default)]
pub struct Tally {
    pub ok: usize,
    pub expected: usize,
    pub failed: usize,
    pub unexpected_pass: usize,
}

pub fn tally(judged: &[Judged]) -> Tally {
    let mut t = Tally::default();
    for j in judged {
        match j.verdict {
            Verdict::Pass | Verdict::Allowed(_) => t.ok += 1,
            Verdict::ExpectedFail(_) => t.expected += 1,
            Verdict::Fail(_) => t.failed += 1,
            Verdict::UnexpectedPass(_) => t.unexpected_pass += 1,
        }
    }
    t
}

pub fn cause_key(reason: &str) -> &str {
    reason.split_once(": ").map_or(reason, |(key, _)| key)
}

pub fn print_verdicts(judged: &[Judged], verbose: bool) -> usize {
    let mut failures = 0;
    for j in judged {
        let text = match &j.verdict {
            Verdict::Pass | Verdict::Allowed(_) => continue,
            Verdict::ExpectedFail(_) if !verbose => continue,
            Verdict::Fail(detail) => detail.clone(),
            Verdict::ExpectedFail(r) | Verdict::UnexpectedPass(r) => {
                format!("[{}] {}", cause_key(r), j.detail.as_deref().unwrap_or(""))
            }
        };
        failures += usize::from(j.verdict.is_failure());
        println!(
            "fixture {} {} {} {} {text} | {}",
            j.file,
            j.lane,
            j.check,
            j.verdict.tag(),
            j.note.as_deref().unwrap_or("-")
        );
    }
    failures
}

pub struct CellRow {
    pub cell: String,
    pub fixtures: usize,
    pub passed: BTreeMap<&'static str, usize>,
    pub webcrack: Option<usize>,
    pub gaps: usize,
}

pub fn cells(outcomes: &[Outcome], oracle: &BTreeMap<String, Row>) -> Vec<CellRow> {
    let mut rows: BTreeMap<&str, CellRow> = BTreeMap::new();
    for o in outcomes {
        let row = rows
            .entry(o.entry.cell.as_str())
            .or_insert_with(|| CellRow {
                cell: o.entry.cell.clone(),
                fixtures: 0,
                passed: BTreeMap::new(),
                webcrack: None,
                gaps: 0,
            });
        row.fixtures += 1;
        for lane in UNTWIST_LANES {
            let perfect = o.lanes.get(lane).is_some_and(|r| r.perfect());
            *row.passed.entry(lane).or_default() += usize::from(perfect);
        }
        if let Some(oracle_row) = oracle.get(&o.entry.file) {
            *row.webcrack.get_or_insert(0) += usize::from(oracle_row.perfect);
            let unsafe_perfect = o.lanes.get(UNSAFE).is_some_and(|r| r.perfect());
            row.gaps += usize::from(oracle_row.perfect && !unsafe_perfect);
        }
    }
    rows.into_values().collect()
}

fn ratio(n: usize, of: usize) -> String {
    format!("{n}/{of}")
}

fn webcrack_text(row: &CellRow) -> String {
    row.webcrack
        .map_or("-".to_string(), |w| ratio(w, row.fixtures))
}

pub fn print_cells(rows: &[CellRow]) {
    println!("cell-summary cell\tfixtures\tsafe\tunsafe\twebcrack\tparity-gaps");
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for row in rows {
        println!(
            "cell-summary {}\t{}\t{}\t{}\t{}\t{}",
            row.cell,
            row.fixtures,
            ratio(row.passed[SAFE], row.fixtures),
            ratio(row.passed[UNSAFE], row.fixtures),
            webcrack_text(row),
            row.gaps
        );
        for lane in UNTWIST_LANES {
            *totals.entry(lane).or_default() += usize::from(row.passed[lane] == row.fixtures);
        }
    }
    let fixtures: usize = rows.iter().map(|r| r.fixtures).sum();
    let passed = |lane: &str| rows.iter().map(|r| r.passed[lane]).sum::<usize>();
    let webcrack_cells = rows
        .iter()
        .filter(|r| r.webcrack == Some(r.fixtures))
        .count();
    let webcrack_fixtures: usize = rows.iter().filter_map(|r| r.webcrack).sum();
    println!(
        "lanes: safe {} cells ({} fixtures), unsafe {} cells ({} fixtures), webcrack {} cells ({} fixtures) perfect of {} cells ({fixtures} fixtures)",
        totals[SAFE],
        passed(SAFE),
        totals[UNSAFE],
        passed(UNSAFE),
        webcrack_cells,
        webcrack_fixtures,
        rows.len()
    );
}

struct Cause {
    reason: String,
    cells: BTreeSet<String>,
    fixtures: BTreeSet<String>,
    runs: BTreeSet<(String, &'static str)>,
}

fn causes(judged: &[Judged]) -> Vec<Cause> {
    let mut by_key: BTreeMap<String, Cause> = BTreeMap::new();
    for j in judged {
        let Verdict::ExpectedFail(reason) = &j.verdict else {
            continue;
        };
        let cause = by_key
            .entry(cause_key(reason).to_string())
            .or_insert_with(|| Cause {
                reason: reason.clone(),
                cells: BTreeSet::new(),
                fixtures: BTreeSet::new(),
                runs: BTreeSet::new(),
            });
        cause.cells.insert(j.cell.clone());
        cause.fixtures.insert(j.file.clone());
        cause.runs.insert((j.file.clone(), j.lane));
    }
    let mut out: Vec<Cause> = by_key.into_values().collect();
    out.sort_by(|a, b| {
        b.cells
            .len()
            .cmp(&a.cells.len())
            .then(b.fixtures.len().cmp(&a.fixtures.len()))
            .then(a.reason.cmp(&b.reason))
    });
    out
}

pub fn print_causes(judged: &[Judged]) {
    println!("cause rank\tcells\tfixtures\tlane-runs\tcause");
    for (rank, cause) in causes(judged).iter().enumerate() {
        println!(
            "cause {}\t{}\t{}\t{}\t{}",
            rank + 1,
            cause.cells.len(),
            cause.fixtures.len(),
            cause.runs.len(),
            cause_key(&cause.reason)
        );
    }
}

fn parity(outcomes: &[Outcome], oracle: &BTreeMap<String, Row>) -> (Vec<String>, Vec<String>) {
    let mut gaps = Vec::new();
    let mut beats = Vec::new();
    for o in outcomes {
        let (Some(row), Some(run)) = (oracle.get(&o.entry.file), o.lanes.get(UNSAFE)) else {
            continue;
        };
        if row.perfect && !run.perfect() {
            gaps.push(o.entry.file.clone());
        }
        if run.perfect() && !row.perfect {
            beats.push(o.entry.file.clone());
        }
    }
    (gaps, beats)
}

pub fn print_parity(outcomes: &[Outcome], oracle: &BTreeMap<String, Row>, verbose: bool) {
    if oracle.is_empty() {
        println!("parity: no ORACLE; run with --webcrack to record webcrack's results");
        return;
    }
    let (gaps, beats) = parity(outcomes, oracle);
    for check in CHECKS {
        let n = outcomes
            .iter()
            .filter(|o| {
                let unsafe_failed = o.lanes.get(UNSAFE).is_some_and(|r| {
                    r.checks()
                        .iter()
                        .any(|(c, failure)| *c == check && failure.is_some())
                });
                oracle.get(&o.entry.file).is_some_and(|r| r.ok(check)) && unsafe_failed
            })
            .count();
        println!("parity-check {check}: webcrack passes and unsafe fails on {n} fixture(s)");
    }
    for file in gaps.iter().filter(|_| verbose) {
        println!("parity-gap {file}");
    }
    for file in &beats {
        println!("beats-webcrack {file}");
    }
    println!(
        "parity: {} fixture(s) webcrack makes perfect and --unsafe does not; untwist --unsafe beats webcrack on {}",
        gaps.len(),
        beats.len()
    );
}

type LaneRuns<'a> = BTreeMap<&'a str, BTreeSet<(String, &'static str)>>;

fn causes_by_cell<'a>(
    judged: &'a [Judged],
    keep: impl Fn(&Judged) -> bool,
) -> BTreeMap<&'a str, LaneRuns<'a>> {
    let mut out: BTreeMap<&str, LaneRuns> = BTreeMap::new();
    for j in judged.iter().filter(|j| keep(j)) {
        if let Verdict::ExpectedFail(reason) = &j.verdict {
            out.entry(j.cell.as_str())
                .or_default()
                .entry(cause_key(reason))
                .or_default()
                .insert((j.file.clone(), j.lane));
        }
    }
    out
}

fn listed(causes: Option<&LaneRuns>) -> String {
    let parts: Vec<String> = causes
        .map(|m| {
            m.iter()
                .map(|(key, runs)| format!("{key} ({})", runs.len()))
                .collect()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(", ")
    }
}

fn is_gap(j: &Judged, oracle: &BTreeMap<String, Row>) -> bool {
    j.lane == UNSAFE && oracle.get(&j.file).is_some_and(|r| r.perfect)
}

fn parity_lines(
    rows: &[CellRow],
    judged: &[Judged],
    oracle: &BTreeMap<String, Row>,
) -> Vec<String> {
    let gaps = causes_by_cell(judged, |j| is_gap(j, oracle));
    let mut lines = vec![
        "# parity gaps first: cells where webcrack is perfect on a fixture and --unsafe is not"
            .to_string(),
    ];
    lines.push("# parity-gap cell\tfixtures\tunsafe causes on those fixtures".to_string());
    for row in rows.iter().filter(|r| r.gaps > 0) {
        lines.push(format!(
            "{}\t{}\t{}",
            row.cell,
            row.gaps,
            listed(gaps.get(row.cell.as_str()))
        ));
    }
    lines
}

fn cell_lines(rows: &[CellRow], judged: &[Judged]) -> Vec<String> {
    let causes = causes_by_cell(judged, |_| true);
    let mut lines = vec![
        "# cell\tfixtures\tsafe\tunsafe\twebcrack\tparity-gaps\tcauses (lane runs)".to_string(),
    ];
    for row in rows {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}",
            row.cell,
            row.fixtures,
            ratio(row.passed[SAFE], row.fixtures),
            ratio(row.passed[UNSAFE], row.fixtures),
            webcrack_text(row),
            row.gaps,
            listed(causes.get(row.cell.as_str()))
        ));
    }
    lines
}

fn cause_lines(judged: &[Judged], oracle: &BTreeMap<String, Row>) -> Vec<String> {
    let mut gap_cells: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (cell, causes) in causes_by_cell(judged, |j| is_gap(j, oracle)) {
        for key in causes.keys() {
            gap_cells.entry(key).or_default().insert(cell);
        }
    }
    let mut lines =
        vec!["# cause\tcells\tfixtures\tlane-runs\tparity-gap-cells\treason".to_string()];
    for cause in causes(judged) {
        let key = cause_key(&cause.reason);
        lines.push(format!(
            "{key}\t{}\t{}\t{}\t{}\t{}",
            cause.cells.len(),
            cause.fixtures.len(),
            cause.runs.len(),
            gap_cells.get(key).map_or(0, BTreeSet::len),
            cause.reason.split_once(": ").map_or("", |(_, r)| r)
        ));
    }
    lines
}

pub fn write_baseline(
    path: &Path,
    rows: &[CellRow],
    judged: &[Judged],
    oracle: &BTreeMap<String, Row>,
) -> Result<()> {
    let mut lines = vec![
        "# obfuscator-io option matrix baseline, written by cargo xtask obf-options --baseline".to_string(),
        "# safe, unsafe and webcrack count fixtures that are perfect: they run, keep no obfuscator machinery, simplify every expression back and print what the original prints".to_string(),
    ];
    if !oracle.is_empty() {
        lines.extend(parity_lines(rows, judged, oracle));
    }
    lines.extend(cell_lines(rows, judged));
    lines.extend(cause_lines(judged, oracle));
    std::fs::write(path, lines.join("\n") + "\n").with_context(|| path.display().to_string())
}

fn signature(j: &Judged) -> String {
    let classes: Vec<&str> = j
        .detail
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|part| part.split('=').next().unwrap_or(part))
        .collect();
    let note = j
        .note
        .as_ref()
        .map(|n| note_classes(std::slice::from_ref(n)).join(""))
        .unwrap_or_else(|| "no note".to_string());
    match j.check {
        "machinery" | "simplified" => format!("{} {}; {note}", j.check, classes.join("+")),
        other => format!("{other}; {note}"),
    }
}

pub fn write_draft(path: &Path, judged: &[Judged], outcomes: &[Outcome]) -> Result<()> {
    let mut per_cell: BTreeMap<&str, usize> = BTreeMap::new();
    for o in outcomes {
        *per_cell.entry(o.entry.cell.as_str()).or_default() += 1;
    }
    let mut groups: BTreeMap<(String, &str, String), BTreeMap<String, BTreeSet<&str>>> =
        BTreeMap::new();
    for j in judged
        .iter()
        .filter(|j| matches!(j.verdict, Verdict::Fail(_)))
    {
        groups
            .entry((j.cell.clone(), j.check, signature(j)))
            .or_default()
            .entry(j.file.clone())
            .or_default()
            .insert(j.lane);
    }
    let mut lines = Vec::new();
    for ((cell, check, sig), files) in groups {
        let lanes_of = |set: &BTreeSet<&str>| {
            if set.len() == UNTWIST_LANES.len() {
                "*".to_string()
            } else {
                set.iter().copied().collect::<Vec<_>>().join(",")
            }
        };
        let same_lanes: BTreeSet<String> = files.values().map(lanes_of).collect();
        if files.len() == per_cell[cell.as_str()] && same_lanes.len() == 1 {
            let lanes = same_lanes.into_iter().next().unwrap_or_default();
            lines.push(format!("{cell}--*\t{lanes}\t{check}\txfail\tTODO: {sig}"));
            continue;
        }
        for (file, set) in files {
            lines.push(format!(
                "{file}\t{}\t{check}\txfail\tTODO: {sig}",
                lanes_of(&set)
            ));
        }
    }
    std::fs::write(path, lines.join("\n") + "\n").with_context(|| path.display().to_string())
}
