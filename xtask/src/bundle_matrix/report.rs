use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};

use crate::obf_matrix::expect::{glob, Mark, Rule, Verdict};

use super::checks::CHECKS;
use super::manifest::Cell;

pub struct Judged {
    pub cell: Cell,
    pub lane: &'static str,
    pub check: &'static str,
    pub verdict: Verdict,
    pub detail: Option<String>,
    pub bundle: Option<(String, usize)>,
}

#[derive(Default)]
pub struct Tally {
    pub ok: usize,
    pub failed: usize,
    pub expected: usize,
    pub unexpected_pass: usize,
    pub allowed: usize,
}

pub fn tally(judged: &[Judged]) -> Tally {
    let mut t = Tally::default();
    for j in judged {
        match j.verdict {
            Verdict::Pass => t.ok += 1,
            Verdict::Fail(_) => t.failed += 1,
            Verdict::ExpectedFail(_) => t.expected += 1,
            Verdict::UnexpectedPass(_) => t.unexpected_pass += 1,
            Verdict::Allowed(_) => t.allowed += 1,
        }
    }
    t
}

fn label(j: &Judged) -> String {
    j.bundle
        .as_ref()
        .map_or("none".to_string(), |(kind, n)| format!("{kind}:{n}"))
}

pub fn print_verdicts(judged: &[Judged], verbose: bool) -> usize {
    let mut failures = 0;
    for j in judged {
        let shown = match &j.verdict {
            Verdict::Pass => continue,
            Verdict::Fail(d) => d.clone(),
            Verdict::UnexpectedPass(r) => format!("[{r}] now passes; drop or narrow the rule"),
            Verdict::ExpectedFail(r) | Verdict::Allowed(r) if verbose => {
                format!("[{r}] {}", j.detail.as_deref().unwrap_or(""))
            }
            _ => continue,
        };
        failures += usize::from(j.verdict.is_failure());
        println!(
            "cell {} {} {} {} {} {shown}",
            j.cell.id(),
            j.lane,
            j.check,
            label(j),
            j.verdict.tag()
        );
    }
    failures
}

type Row<'a> = (BTreeSet<String>, BTreeSet<String>, BTreeMap<&'a str, usize>);

pub fn print_summary(judged: &[Judged]) {
    let mut rows: BTreeMap<(String, &str), Row> = BTreeMap::new();
    for j in judged {
        let row = rows.entry((j.cell.bundler(), j.lane)).or_default();
        row.0.insert(j.cell.id());
        if j.detail.is_some() {
            row.1.insert(j.cell.id());
            *row.2.entry(j.check).or_default() += 1;
        }
    }
    println!(
        "summary bundler\tlane\tcells\tclean-cells\t{}",
        CHECKS.join("\t")
    );
    for ((bundler, lane), (cells, failing, by_check)) in &rows {
        let counts: Vec<String> = CHECKS
            .iter()
            .map(|c| by_check.get(c).copied().unwrap_or(0).to_string())
            .collect();
        println!(
            "summary {bundler}\t{lane}\t{}\t{}\t{}",
            cells.len(),
            cells.len() - failing.len(),
            counts.join("\t")
        );
    }
}

fn covering<'a>(rules: &'a [Rule], j: &Judged) -> Option<&'a Rule> {
    rules
        .iter()
        .find(|r| r.check == j.check && glob(&r.fixture, &j.cell.id()) && glob(&r.cell, j.lane))
}

fn line(rule: &Rule) -> String {
    let mark = if rule.mark == Mark::ExpectedFail {
        "xfail"
    } else {
        "allow"
    };
    format!(
        "{}\t{}\t{}\t{mark}\t{}",
        rule.fixture, rule.cell, rule.check, rule.reason
    )
}

pub fn write_suggestion(path: &Path, rules: &[Rule], judged: &[Judged]) -> Result<()> {
    let mut kept: BTreeSet<usize> = BTreeSet::new();
    let mut fresh = Vec::new();
    for j in judged.iter().filter(|j| j.detail.is_some()) {
        match covering(rules, j) {
            Some(rule) => {
                kept.insert(
                    rules
                        .iter()
                        .position(|r| std::ptr::eq(r, rule))
                        .unwrap_or(0),
                );
            }
            None => fresh.push(format!(
                "{}\t{}\t{}\txfail\tTODO {}",
                j.cell.id(),
                j.lane,
                j.check,
                j.detail.as_deref().unwrap_or("")
            )),
        }
    }
    let mut out = vec!["# app/cell\tlane\tcheck\txfail|allow\treason".to_string()];
    out.extend(kept.iter().map(|&i| line(&rules[i])));
    out.extend(fresh);
    std::fs::write(path, out.join("\n") + "\n").with_context(|| path.display().to_string())
}
