pub mod inventory;

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use untwist_testing::node::{node, normalise_stderr, skip_allowed, syntax_check, SKIP_MESSAGE};

pub const FIXTURES: &str = "fixtures/layers";
pub const RUNNER: &str = "tools/layers/run.cjs";
pub const ALLOWLIST: &str = "fixtures/layers/ALLOWLIST";
const TIMEOUT: Duration = Duration::from_secs(20);
const SAFE: [&str; 2] = ["--no-unpack", "--no-jsx"];
const UNSAFE: [&str; 3] = ["--unsafe", "--no-unpack", "--no-jsx"];
const CONSERVATIVE: [&str; 4] = ["--unsafe", "--conservative", "--no-unpack", "--no-jsx"];
pub const LANES: [(&str, &[&str]); 3] = [
    ("safe", &SAFE),
    ("unsafe", &UNSAFE),
    ("conservative", &CONSERVATIVE),
];

#[derive(clap::Subcommand)]
pub enum Command {
    DiffLayers(DiffRequest),
    CorpusLayers(crate::corpus_layers::Request),
    LayerInventory(inventory::Request),
}

impl Command {
    pub fn run(&self, root: &Path) -> Result<()> {
        match self {
            Command::DiffLayers(request) => diff_layers(root, request),
            Command::CorpusLayers(request) => crate::corpus_layers::run(root, request),
            Command::LayerInventory(request) => inventory::run(request),
        }
    }
}

#[derive(clap::Args, Debug)]
pub struct DiffRequest {
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub seed_fault: bool,
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    pub files: Vec<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Observed {
    pub exit: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum Verdict {
    Match,
    Skipped(String),
    Mismatch { expected: String, actual: String },
    CheckFailed(String),
}

impl Verdict {
    fn label(&self) -> &'static str {
        match self {
            Verdict::Match => "ok",
            Verdict::Skipped(_) => "SKIPPED",
            Verdict::Mismatch { .. } => "MISMATCH",
            Verdict::CheckFailed(_) => "CHECK-FAILED",
        }
    }
}

struct Lane<'a> {
    node: &'a Path,
    runner: &'a Path,
    untwist: &'a Path,
    out: &'a Path,
    seed_fault: bool,
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buffer);
        }
        buffer
    })
}

pub fn run_script(node: &Path, runner: &Path, file: &Path) -> Result<Option<Observed>> {
    let started = Instant::now();
    let mut child = Process::new(node)
        .arg(runner)
        .arg(file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {}", node.display()))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    loop {
        if let Some(status) = child.try_wait()? {
            let out = stdout.join().unwrap_or_default();
            let err = stderr.join().unwrap_or_default();
            return Ok(Some(Observed {
                exit: status.code(),
                stdout: String::from_utf8_lossy(&out).into_owned(),
                stderr: normalise_stderr(&String::from_utf8_lossy(&err), &[file]),
            }));
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn render(observed: &Observed) -> String {
    format!(
        "exit={:?}\nstdout={}\nstderr={}",
        observed.exit, observed.stdout, observed.stderr
    )
}

pub fn swap_two_prints(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let (first, second) = two_prints(&lines).or_else(|| last_two(&lines))?;
    let mut swapped = lines.clone();
    swapped.swap(first, second);
    Some(swapped.join("\n") + "\n")
}

fn two_prints(lines: &[&str]) -> Option<(usize, usize)> {
    let prints: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].trim_start().starts_with("console.log("))
        .collect();
    let first = *prints.first()?;
    let second = prints
        .iter()
        .copied()
        .find(|&i| lines[i].trim() != lines[first].trim())?;
    Some((first, second))
}

fn last_two(lines: &[&str]) -> Option<(usize, usize)> {
    let filled: Vec<usize> = (0..lines.len())
        .filter(|&i| !lines[i].trim().is_empty())
        .collect();
    let second = *filled.last()?;
    let first = filled
        .iter()
        .rev()
        .copied()
        .find(|&i| lines[i].trim() != lines[second].trim())?;
    Some((first, second))
}

fn untwist_output(untwist: &Path, flags: &[&str], file: &Path) -> Result<String, String> {
    let out = Process::new(untwist)
        .args(flags)
        .arg(file)
        .output()
        .map_err(|err| format!("spawn {}: {err}", untwist.display()))?;
    if !out.status.success() {
        return Err(format!(
            "untwist exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn lane_text(lane: &Lane, flags: &[&str], file: &Path) -> Result<String, Verdict> {
    let text = untwist_output(lane.untwist, flags, file).map_err(Verdict::CheckFailed)?;
    if !lane.seed_fault {
        return Ok(text);
    }
    swap_two_prints(&text).ok_or_else(|| Verdict::Skipped("nothing to swap".to_string()))
}

fn run_lane(
    lane: &Lane,
    (name, flags): (&str, &[&str]),
    file: &Path,
    a: &Observed,
) -> Result<Verdict> {
    let text = match lane_text(lane, flags, file) {
        Ok(text) => text,
        Err(verdict) => return Ok(verdict),
    };
    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("fixture");
    let parent = file
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let path = lane.out.join(format!("{parent}-{stem}.{name}.js"));
    std::fs::write(&path, &text).with_context(|| format!("write {}", path.display()))?;
    if let Err(message) = syntax_check(lane.node, &path)? {
        return Ok(Verdict::CheckFailed(format!(
            "node --check: {}",
            message.trim()
        )));
    }
    let Some(b) = run_script(lane.node, lane.runner, &path)? else {
        return Ok(Verdict::Mismatch {
            expected: render(a),
            actual: format!("did not terminate within {TIMEOUT:?}"),
        });
    };
    if &b == a {
        return Ok(Verdict::Match);
    }
    Ok(Verdict::Mismatch {
        expected: render(a),
        actual: render(&b),
    })
}

pub fn fixture_files(root: &Path, kind: Option<&str>) -> Result<Vec<PathBuf>> {
    let base = root.join(FIXTURES);
    let mut files = Vec::new();
    collect(&base, &mut files)?;
    files.retain(|path| {
        kind.is_none_or(|kind| {
            path.strip_prefix(&base)
                .is_ok_and(|rel| rel.starts_with(kind))
        })
    });
    files.sort();
    if files.is_empty() {
        bail!("no fixtures under {} for kind {kind:?}", base.display());
    }
    Ok(files)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "js") {
            out.push(path);
        }
    }
    Ok(())
}

pub fn read_allowlist(root: &Path) -> Result<BTreeSet<String>> {
    let path = root.join(ALLOWLIST);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect())
}

#[derive(Default)]
struct Tally {
    runs: usize,
    ok: usize,
    skipped: usize,
    failed: usize,
    allowed: usize,
    seen: BTreeSet<String>,
}

fn record(tally: &mut Tally, allowlist: &BTreeSet<String>, key: String, verdict: &Verdict) {
    tally.runs += 1;
    let allowed = allowlist.contains(&key);
    match verdict {
        Verdict::Match => tally.ok += 1,
        Verdict::Skipped(_) => tally.skipped += 1,
        _ if allowed => {
            tally.allowed += 1;
            tally.seen.insert(key);
        }
        _ => tally.failed += 1,
    }
}

fn report(key: &str, verdict: &Verdict) {
    println!("layer {key} {}", verdict.label());
    match verdict {
        Verdict::Skipped(reason) | Verdict::CheckFailed(reason) => println!("  {reason}"),
        Verdict::Mismatch { expected, actual } => {
            println!(
                "  input:\n{}\n  output:\n{}",
                indent(expected),
                indent(actual)
            );
        }
        Verdict::Match => {}
    }
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tools(root: &Path, request: &DiffRequest) -> Result<Option<(PathBuf, PathBuf)>> {
    let Some(node) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        if skip_allowed(required.as_deref()) {
            println!("{SKIP_MESSAGE}");
            return Ok(None);
        }
        bail!("required: {SKIP_MESSAGE}");
    };
    let untwist = request
        .untwist
        .clone()
        .unwrap_or_else(|| root.join("target/release/untwist"));
    if !untwist.exists() {
        bail!(
            "build first: cargo build --release -p untwist-cli --features test-hooks ({} is absent)",
            untwist.display()
        );
    }
    Ok(Some((node, untwist)))
}

pub fn diff_layers(root: &Path, request: &DiffRequest) -> Result<()> {
    let Some((node, untwist)) = tools(root, request)? else {
        return Ok(());
    };
    let whole_set = request.files.is_empty() && request.kind.is_none() && !request.seed_fault;
    let files = if request.files.is_empty() {
        fixture_files(root, request.kind.as_deref())?
    } else {
        request.files.clone()
    };
    let allowlist = if request.seed_fault {
        BTreeSet::new()
    } else {
        read_allowlist(root)?
    };
    let out = tempfile::Builder::new()
        .prefix("diff-layers-")
        .tempdir_in(root.join("target"))
        .context("create the output dir under target/")?;
    let runner = root.join(RUNNER);
    let lane = Lane {
        node: &node,
        runner: &runner,
        untwist: &untwist,
        out: out.path(),
        seed_fault: request.seed_fault,
    };
    let mut tally = Tally::default();
    for file in &files {
        diff_fixture(root, &lane, file, &allowlist, &mut tally)?;
    }
    summarise(&files, &tally, &allowlist, whole_set)
}

fn diff_fixture(
    root: &Path,
    lane: &Lane,
    file: &Path,
    allowlist: &BTreeSet<String>,
    tally: &mut Tally,
) -> Result<()> {
    let file =
        std::fs::canonicalize(file).with_context(|| format!("canonicalize {}", file.display()))?;
    let base = std::fs::canonicalize(root.join(FIXTURES))?;
    let rel = file
        .strip_prefix(&base)
        .unwrap_or(&file)
        .display()
        .to_string();
    let Some(a) = run_script(lane.node, lane.runner, &file)? else {
        for (name, _) in LANES {
            let key = format!("{rel} {name}");
            let verdict = Verdict::Skipped("the input did not terminate".to_string());
            report(&key, &verdict);
            record(tally, allowlist, key, &verdict);
        }
        return Ok(());
    };
    for (name, flags) in LANES {
        let verdict = run_lane(lane, (name, flags), &file, &a)?;
        let key = format!("{rel} {name}");
        report(&key, &verdict);
        record(tally, allowlist, key, &verdict);
    }
    Ok(())
}

fn summarise(
    files: &[PathBuf],
    tally: &Tally,
    allowlist: &BTreeSet<String>,
    whole_set: bool,
) -> Result<()> {
    let stale: Vec<&String> = if whole_set {
        allowlist.difference(&tally.seen).collect()
    } else {
        Vec::new()
    };
    for key in &stale {
        println!("stale allowlist line: {key}");
    }
    println!(
        "diff-layers: {} fixtures, {} runs, {} ok, {} skipped, {} failed, {} allowlisted, {} stale",
        files.len(),
        tally.runs,
        tally.ok,
        tally.skipped,
        tally.failed,
        tally.allowed,
        stale.len()
    );
    if tally.failed > 0 || !stale.is_empty() {
        bail!(
            "{} run(s) failed, {} stale allowlist line(s)",
            tally.failed,
            stale.len()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::swap_two_prints;

    #[test]
    fn the_seeded_fault_swaps_two_different_prints_or_the_last_two_lines() {
        let text = "var a = 1;\nconsole.log(1);\nconsole.log(1);\nconsole.log(2);\n";
        assert_eq!(
            swap_two_prints(text).as_deref(),
            Some("var a = 1;\nconsole.log(2);\nconsole.log(1);\nconsole.log(1);\n")
        );
        assert_eq!(
            swap_two_prints("var a = 1;\nf(a);\n").as_deref(),
            Some("f(a);\nvar a = 1;\n")
        );
        assert_eq!(swap_two_prints("console.log(1);\n"), None);
    }
}
