use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::obf_matrix::cell::{Cell, Observation};
use crate::obf_matrix::checks::Subject;
use crate::obf_matrix::notes::{parse_ids, render_ids};
use crate::obf_matrix::{
    expect, judge_all, observe_all, print_summary, runner, warning_summary, Tally,
};

pub const BASELINE: &str = "fixtures/corpus-export/baseline.tsv";
pub const EXPECT: &str = "fixtures/corpus-export/EXPECT";
const EXPORT: &str = "export";
const GAME_JS: &str = "fixtures/real-world/game.js";
const GAME_OLD: &str = "game_old.js";
const USER_SAMPLE: usize = 10;
const HEADER: &str = "# sha256\tname\tcell\texit\tparses\tcalls\tarrays\trotators\tmodules\twarnings\terrors\tclasses\twall_ms\trss_kb";
const RSS_SLACK_KB: u64 = 65_536;
const WALL_SLACK_MS: u64 = 10_000;

#[derive(clap::Args)]
pub struct Request {
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long)]
    pub export: Option<PathBuf>,
    #[arg(long)]
    pub baseline: Option<PathBuf>,
    #[arg(long)]
    pub user: bool,
    #[arg(long)]
    pub bless: bool,
    #[arg(long, default_value_t = 2)]
    pub jobs_game: usize,
    #[arg(long, default_value_t = 6)]
    pub jobs_loader: usize,
    #[arg(long)]
    pub only: Option<String>,
    #[arg(long)]
    pub summary: Option<PathBuf>,
    #[arg(long)]
    pub metrics: Option<PathBuf>,
}

struct Input {
    name: String,
    path: PathBuf,
    sha: String,
    game: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub sha: String,
    pub name: String,
    pub cell: String,
    pub exit: String,
    pub parses: bool,
    pub calls: usize,
    pub arrays: usize,
    pub rotators: usize,
    pub modules: usize,
    pub warnings: usize,
    pub errors: usize,
    pub classes: String,
    pub wall_ms: u64,
    pub rss_kb: u64,
}

impl Row {
    fn of(input: &Input, cell: Cell, o: &Observation) -> Row {
        let residue = o.residue();
        Row {
            sha: input.sha.clone(),
            name: input.name.clone(),
            cell: cell.label(),
            exit: o.exit.map_or("timeout".to_string(), |c| c.to_string()),
            parses: o.parses(),
            calls: residue.decoder_calls,
            arrays: residue.string_arrays,
            rotators: residue.rotators,
            modules: o.module_set.len(),
            warnings: o.warnings.total(),
            errors: o.warnings.errors,
            classes: render_ids(&o.warnings.classes),
            wall_ms: o.wall_ms,
            rss_kb: o.rss_kb.unwrap_or(0),
        }
    }

    fn render(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.sha,
            self.name,
            self.cell,
            self.exit,
            self.parses,
            self.calls,
            self.arrays,
            self.rotators,
            self.modules,
            self.warnings,
            self.errors,
            self.classes,
            self.wall_ms,
            self.rss_kb
        )
    }

    fn parse(line: &str) -> Result<Row> {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() != 14 {
            bail!("baseline row has {} fields: {line}", f.len());
        }
        let n = |i: usize| -> Result<usize> {
            f[i].parse().with_context(|| format!("field {i} of {line}"))
        };
        Ok(Row {
            sha: f[0].to_string(),
            name: f[1].to_string(),
            cell: f[2].to_string(),
            exit: f[3].to_string(),
            parses: f[4] == "true",
            calls: n(5)?,
            arrays: n(6)?,
            rotators: n(7)?,
            modules: n(8)?,
            warnings: n(9)?,
            errors: n(10)?,
            classes: f[11].to_string(),
            wall_ms: n(12)? as u64,
            rss_kb: n(13)? as u64,
        })
    }

    fn key(&self) -> (String, String) {
        (self.sha.clone(), self.cell.clone())
    }
}

pub fn regressions(now: &Row, base: &Row) -> Vec<String> {
    let mut out = Vec::new();
    let mut worse = |what: &str, a: String, b: String| out.push(format!("{what} {b} -> {a}"));
    if base.exit == "0" && now.exit != "0" {
        worse("exit", now.exit.clone(), base.exit.clone());
    }
    if base.parses && !now.parses {
        worse("parses", "false".into(), "true".into());
    }
    for (what, a, b) in [
        ("decoder calls", now.calls, base.calls),
        ("string arrays", now.arrays, base.arrays),
        ("rotators", now.rotators, base.rotators),
        ("warnings", now.warnings, base.warnings),
        ("errors", now.errors, base.errors),
    ] {
        if a > b {
            worse(what, a.to_string(), b.to_string());
        }
    }
    if now.modules < base.modules {
        worse("modules", now.modules.to_string(), base.modules.to_string());
    }
    let base_ids = parse_ids(&base.classes);
    for (id, n) in parse_ids(&now.classes) {
        let had = base_ids.get(&id).copied().unwrap_or(0);
        if n > had {
            worse(
                &format!("warning class {id}"),
                n.to_string(),
                had.to_string(),
            );
        }
    }
    if base.rss_kb > 0 && now.rss_kb > base.rss_kb + base.rss_kb / 2 + RSS_SLACK_KB {
        worse(
            "peak rss kB",
            now.rss_kb.to_string(),
            base.rss_kb.to_string(),
        );
    }
    if now.wall_ms > base.wall_ms * 2 + WALL_SLACK_MS {
        worse("wall ms", now.wall_ms.to_string(), base.wall_ms.to_string());
    }
    out
}

fn improved(now: &Row, base: &Row) -> bool {
    now.calls < base.calls
        || now.arrays < base.arrays
        || now.rotators < base.rotators
        || now.warnings < base.warnings
        || now.errors < base.errors
        || now.modules > base.modules
        || (now.exit == "0" && base.exit != "0")
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).with_context(|| path.display().to_string())?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn export_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "js") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn sample(files: Vec<PathBuf>) -> Vec<PathBuf> {
    let step = (files.len() / USER_SAMPLE).max(1);
    files.into_iter().step_by(step).take(USER_SAMPLE).collect()
}

fn inputs(root: &Path, request: &Request, export: &Path) -> Result<Vec<Input>> {
    let mut paths = Vec::new();
    if request.user {
        let beside = export.parent().unwrap_or(root).join(GAME_OLD);
        paths.extend(
            [root.join(GAME_JS), beside]
                .into_iter()
                .filter(|p| p.is_file()),
        );
        paths.extend(sample(export_files(export)?));
    } else {
        paths.extend(export_files(export)?);
    }
    let mut out = Vec::new();
    for path in paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if request
            .only
            .as_deref()
            .is_some_and(|only| !name.contains(only))
        {
            continue;
        }
        out.push(Input {
            sha: sha256_file(&path)?,
            game: !name.starts_with("loader-"),
            name,
            path,
        });
    }
    Ok(out)
}

fn load_baseline(path: &Path) -> Result<BTreeMap<(String, String), Row>> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| Row::parse(l).map(|r| (r.key(), r)))
        .collect()
}

fn write_baseline(path: &Path, base: &BTreeMap<(String, String), Row>, now: &[Row]) -> Result<()> {
    let mut merged = base.clone();
    for row in now {
        merged.insert(row.key(), row.clone());
    }
    let mut text = String::from(HEADER);
    text.push('\n');
    for row in merged.values() {
        text.push_str(&row.render());
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text).with_context(|| path.display().to_string())
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let started = Instant::now();
    let export = request.export.clone().unwrap_or_else(|| root.join(EXPORT));
    if !export.is_dir() {
        println!("corpus-export: skipped, {} is absent", export.display());
        return Ok(());
    }
    let runner = runner(root, request.untwist.as_ref(), false)?;
    let rules = expect::load(&root.join(EXPECT))?;
    let baseline_path = request
        .baseline
        .clone()
        .unwrap_or_else(|| root.join(BASELINE));
    let baseline = load_baseline(&baseline_path)?;
    let all = inputs(root, request, &export)?;
    std::fs::create_dir_all(root.join("target"))?;
    let work = tempfile::Builder::new()
        .prefix("corpus-export-")
        .tempdir_in(root.join("target"))?;
    let split = |game: bool| -> Vec<(String, PathBuf)> {
        all.iter()
            .filter(|i| i.game == game)
            .map(|i| (i.name.clone(), i.path.clone()))
            .collect()
    };
    let mut observed = observe_all(
        &runner,
        &split(false),
        request.jobs_loader,
        &work.path().join("l"),
        false,
    );
    observed.extend(observe_all(
        &runner,
        &split(true),
        request.jobs_game,
        &work.path().join("g"),
        false,
    ));
    let mut tally = Tally::default();
    let mut rows = Vec::new();
    let (mut regressed, mut new_rows, mut better) = (0, 0, BTreeSet::new());
    for input in &all {
        let Some(cells) = observed.get(&input.name) else {
            tally.failed += 1;
            println!("file {} has no observed cell", input.name);
            continue;
        };
        let subject = Subject {
            cells,
            expected: None,
            realistic: false,
            bundle: false,
        };
        judge_all(&rules, &input.name, &subject, &mut tally);
        for (cell, o) in cells {
            let row = Row::of(input, *cell, o);
            match baseline.get(&row.key()) {
                None => {
                    new_rows += 1;
                    if !request.bless {
                        println!("row {} {} NEW (not in the baseline)", input.name, row.cell);
                    }
                }
                Some(base) => {
                    for what in regressions(&row, base) {
                        regressed += 1;
                        println!("row {} {} REGRESSED {what}", input.name, row.cell);
                    }
                    if improved(&row, base) {
                        better.insert(input.name.clone());
                    }
                }
            }
            rows.push(row);
        }
    }
    let summary = warning_summary(&observed);
    print_summary(&summary);
    if let Some(path) = &request.summary {
        let text: String = summary
            .iter()
            .map(|(class, files, cells, notes)| format!("{files}\t{cells}\t{notes}\t{class}\n"))
            .collect();
        std::fs::write(path, format!("# files\tcells\tnotes\tclass\n{text}"))?;
    }
    if let Some(path) = &request.metrics {
        let text: String = rows.iter().map(|r| r.render() + "\n").collect();
        std::fs::write(path, format!("{HEADER}\n{text}"))?;
    }
    for (reason, cells) in &tally.red_by_reason {
        println!("expected-fail {reason}: {} check(s)", cells.len());
    }
    let warned = rows.iter().filter(|r| r.warnings + r.errors > 0).count();
    println!(
        "corpus-export: {} files, {} cells, {} cells with warnings or errors, {} checks ok, {} expected-fail, {} failed, {} unexpected-pass, {} regressed, {} new, {} improved file(s), wall {:.1} s",
        all.len(),
        rows.len(),
        warned,
        tally.ok,
        tally.expected,
        tally.failed,
        tally.unexpected_pass,
        regressed,
        new_rows,
        better.len(),
        started.elapsed().as_secs_f64()
    );
    if request.bless {
        if regressed > 0 || tally.failures() > 0 {
            bail!(
                "not blessed: {regressed} regression(s), {} failed check(s)",
                tally.failures()
            );
        }
        write_baseline(&baseline_path, &baseline, &rows)?;
        println!(
            "corpus-export: baseline written to {}",
            baseline_path.display()
        );
        return Ok(());
    }
    if regressed > 0 || new_rows > 0 || tally.failures() > 0 {
        bail!(
            "{regressed} regression(s), {new_rows} row(s) missing from the baseline, {} failed check(s)",
            tally.failures()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Row {
        Row {
            sha: "s".into(),
            name: "n".into(),
            cell: "safe/none/stdout".into(),
            exit: "0".into(),
            parses: true,
            calls: 10,
            arrays: 1,
            rotators: 1,
            modules: 0,
            warnings: 3,
            errors: 0,
            classes: "aaaa0000=3".into(),
            wall_ms: 1000,
            rss_kb: 100_000,
        }
    }

    #[test]
    fn a_row_round_trips() {
        assert_eq!(Row::parse(&row().render()).unwrap(), row());
    }

    #[test]
    fn equal_rows_do_not_regress_and_more_residue_does() {
        assert!(regressions(&row(), &row()).is_empty());
        let worse = Row { calls: 11, ..row() };
        assert_eq!(regressions(&worse, &row()).len(), 1);
        let new_class = Row {
            classes: "aaaa0000=2,bbbb1111=1".into(),
            ..row()
        };
        assert_eq!(regressions(&new_class, &row()).len(), 1);
        let better = Row {
            warnings: 0,
            classes: "-".into(),
            ..row()
        };
        assert!(regressions(&better, &row()).is_empty());
        assert!(improved(&better, &row()));
    }

    #[test]
    fn noise_in_time_and_memory_is_tolerated() {
        let slower = Row {
            wall_ms: 2500,
            rss_kb: 140_000,
            ..row()
        };
        assert!(regressions(&slower, &row()).is_empty());
        let much = Row {
            wall_ms: 40_000,
            rss_kb: 400_000,
            ..row()
        };
        assert_eq!(regressions(&much, &row()).len(), 2);
    }

    #[test]
    fn the_user_sample_spreads_over_the_list() {
        let files: Vec<PathBuf> = (0..100)
            .map(|i| PathBuf::from(format!("{i:03}.js")))
            .collect();
        let picked = sample(files);
        assert_eq!(picked.len(), USER_SAMPLE);
        assert_eq!(picked[1], PathBuf::from("010.js"));
    }
}
