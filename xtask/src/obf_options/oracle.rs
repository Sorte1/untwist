use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};

use super::lanes::{behaviour_of, Behaviour, LaneRun, CHECKS};
use super::machinery::{measure, MACHINERY, SIMPLIFY};
use super::manifest::{Entry, DIR};
use crate::obf_matrix::cell::{read_lossy, wait_with_timeout, Runner};
use crate::obf_matrix::pool;

pub const RUNNER: &str = "tools/webcrack/run.mjs";
pub const FILE: &str = "ORACLE";
const CHUNK_TIMEOUT: Duration = Duration::from_secs(1800);
const OK: &str = "ok";
const FAIL: &str = "fail";
const SKIP: &str = "skip";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub perfect: bool,
    pub checks: BTreeMap<String, String>,
    pub detail: String,
}

impl Row {
    pub fn ok(&self, check: &str) -> bool {
        self.checks.get(check).is_some_and(|s| s != FAIL)
    }
}

fn version(root: &Path) -> String {
    let path = root.join("tools/webcrack/node_modules/webcrack/package.json");
    let text = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|v| v["version"].as_str().map(String::from))
        .unwrap_or_else(|| "unknown".to_string())
}

fn spawn_chunk(root: &Path, node: &Path, list: &Path) -> Result<String> {
    let mut child = Command::new(node)
        .arg("--no-warnings")
        .arg(root.join(RUNNER))
        .arg(list)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn webcrack")?;
    let mut pipe = child.stdout.take().context("webcrack stdout")?;
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = pipe.read_to_string(&mut text);
        text
    });
    let _ = wait_with_timeout(&mut child, CHUNK_TIMEOUT)?;
    Ok(reader.join().unwrap_or_default())
}

fn outcomes(stdout: &str) -> BTreeMap<String, Option<String>> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|v| {
            let input = v["input"].as_str()?.to_string();
            let error = (!v["ok"].as_bool().unwrap_or(false))
                .then(|| v["error"].as_str().unwrap_or("failed").to_string());
            Some((input, error))
        })
        .collect()
}

pub fn run(
    root: &Path,
    runner: &Runner,
    entries: &[Entry],
    expected: &BTreeMap<String, String>,
    work: &Path,
    jobs: usize,
) -> Result<BTreeMap<String, LaneRun>> {
    let Some(node) = &runner.node else {
        bail!("the webcrack oracle needs node");
    };
    if !root.join("tools/webcrack/node_modules/webcrack").is_dir() {
        bail!("install the oracle first: npm ci --prefix tools/webcrack");
    }
    let out_dir = work.join("webcrack");
    std::fs::create_dir_all(&out_dir)?;
    let output_of = |e: &Entry| out_dir.join(&e.file);
    let chunks: Vec<PathBuf> = entries
        .chunks(entries.len().div_ceil(jobs.max(1)).max(1))
        .enumerate()
        .map(|(index, chunk)| {
            let list = work.join(format!("webcrack-{index}.tsv"));
            let lines: Vec<String> = chunk
                .iter()
                .map(|e| format!("{}\t{}", e.path(root).display(), output_of(e).display()))
                .collect();
            std::fs::write(&list, lines.join("\n")).map(|_| list)
        })
        .collect::<std::io::Result<_>>()?;
    let mut errors = BTreeMap::new();
    for stdout in pool(chunks, jobs, |list| spawn_chunk(root, node, &list)) {
        errors.extend(outcomes(&stdout?));
    }
    let judged = pool(entries.to_vec(), jobs, |entry| {
        let input = entry.path(root).display().to_string();
        let output = output_of(&entry);
        let (exit, notes) = match errors.get(&input) {
            Some(None) => (Some(0), Vec::new()),
            Some(Some(error)) => (Some(1), vec![format!("webcrack: {error}")]),
            None => (
                None,
                vec!["webcrack: no result (timed out or crashed)".to_string()],
            ),
        };
        let found = measure(&read_lossy(&output));
        let want = expected.get(entry.program()).map_or("", String::as_str);
        let behaviour = behaviour_of(runner, &entry, &found, exit, &output, want);
        (entry.file.clone(), exit, found, behaviour, notes)
    });
    let mut out = BTreeMap::new();
    for (file, exit, found, behaviour, notes) in judged {
        out.insert(
            file,
            LaneRun {
                exit,
                found,
                behaviour: behaviour?,
                notes,
                spans: Vec::new(),
            },
        );
    }
    Ok(out)
}

pub fn row_of(run: &LaneRun) -> Row {
    let checks = run.checks();
    let mut statuses = BTreeMap::new();
    for check in CHECKS {
        let status = match checks.iter().find(|(c, _)| *c == check) {
            None => SKIP,
            Some((_, None)) => OK,
            Some((_, Some(_))) => FAIL,
        };
        statuses.insert(check.to_string(), status.to_string());
    }
    let mut detail = Vec::new();
    for classes in [&MACHINERY[..], &SIMPLIFY[..]] {
        if !run.found.present(classes).is_empty() {
            detail.push(run.found.render(classes));
        }
    }
    if let Behaviour::Differs(d) = &run.behaviour {
        detail.push(d.clone());
    }
    detail.extend(run.notes.first().cloned());
    Row {
        perfect: run.perfect(),
        checks: statuses,
        detail: detail.join("; ").replace('\t', " "),
    }
}

pub fn write(root: &Path, rows: &BTreeMap<String, Row>) -> Result<()> {
    let mut lines = vec![
        format!("# file\tperfect\t{}\tdetail", CHECKS.join("\t")),
        format!(
            "# webcrack@{} from tools/webcrack, run by cargo xtask obf-options --webcrack on {DIR} only; judged like the untwist lanes",
            version(root)
        ),
    ];
    for (file, row) in rows {
        let statuses: Vec<&str> = CHECKS
            .iter()
            .map(|c| row.checks.get(*c).map_or(SKIP, String::as_str))
            .collect();
        lines.push(format!(
            "{file}\t{}\t{}\t{}",
            if row.perfect { OK } else { FAIL },
            statuses.join("\t"),
            row.detail
        ));
    }
    let path = root.join(DIR).join(FILE);
    std::fs::write(&path, lines.join("\n") + "\n").with_context(|| path.display().to_string())
}

pub fn load(root: &Path) -> Result<BTreeMap<String, Row>> {
    let path = root.join(DIR).join(FILE);
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let text = std::fs::read_to_string(&path).with_context(|| path.display().to_string())?;
    let mut rows = BTreeMap::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 + CHECKS.len() {
            bail!("{FILE}: short line {line:?}");
        }
        let checks = CHECKS
            .iter()
            .zip(&fields[2..])
            .map(|(c, s)| (c.to_string(), s.to_string()))
            .collect();
        rows.insert(
            fields[0].to_string(),
            Row {
                perfect: fields[1] == OK,
                checks,
                detail: fields.get(2 + CHECKS.len()).unwrap_or(&"").to_string(),
            },
        );
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_lines_become_outcomes() {
        let got = outcomes(
            "{\"input\":\"/a.js\",\"ok\":true}\n{\"input\":\"/b.js\",\"ok\":false,\"error\":\"boom\"}\nnoise\n",
        );
        assert_eq!(got.get("/a.js"), Some(&None));
        assert_eq!(got.get("/b.js"), Some(&Some("boom".to_string())));
    }

    #[test]
    fn a_skipped_node_check_is_not_a_failure() {
        let run = LaneRun {
            exit: Some(0),
            found: measure("console.log(1);"),
            behaviour: Behaviour::Skipped("residue-only: hostile".into()),
            notes: Vec::new(),
            spans: Vec::new(),
        };
        let row = row_of(&run);
        assert!(row.perfect);
        assert!(row.ok("node"));
        assert_eq!(row.checks["node"], SKIP);
    }
}
