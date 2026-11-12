use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use untwist_testing::node::{
    node, normalise_stderr, run as run_node, skip_allowed, Observed, SKIP_MESSAGE,
};

const CONSERVATIVE_FLAGS: [&str; 4] = ["--unsafe", "--conservative", "--no-unpack", "--no-jsx"];
const DEFAULT_FLAGS: [&str; 3] = ["--unsafe", "--no-unpack", "--no-jsx"];
const SAFE_FLAGS: [&str; 2] = ["--no-unpack", "--no-jsx"];
const UNIQUE_FLAGS: [&str; 5] = ["--unsafe", "--no-unpack", "--no-jsx", "--rename", "unique"];
const SIMPLE_FLAGS: [&str; 5] = ["--unsafe", "--no-unpack", "--no-jsx", "--rename", "simple"];
const SMART_FLAGS: [&str; 5] = ["--unsafe", "--no-unpack", "--no-jsx", "--rename", "smart"];
const SAFE_SMART_FLAGS: [&str; 4] = ["--no-unpack", "--no-jsx", "--rename", "smart"];
const LANES: [(&str, &[&str]); 7] = [
    ("conservative", &CONSERVATIVE_FLAGS),
    ("default", &DEFAULT_FLAGS),
    ("safe", &SAFE_FLAGS),
    ("rename-unique", &UNIQUE_FLAGS),
    ("rename-simple", &SIMPLE_FLAGS),
    ("rename-smart", &SMART_FLAGS),
    ("safe-rename-smart", &SAFE_SMART_FLAGS),
];

pub struct LaneMismatch {
    pub lane: &'static str,
    pub before: String,
    pub after: String,
}

impl LaneMismatch {
    pub fn digest(&self) -> String {
        let digest = Sha256::digest(format!("{}\n--\n{}", self.before, self.after).as_bytes());
        digest
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

pub enum Behaviour {
    Match,
    Mismatch(Vec<LaneMismatch>),
    Skipped(&'static str),
}

fn run(node: &Path, file: &Path) -> Result<Option<Observed>> {
    run_node(node, file).with_context(|| format!("spawn {}", node.display()))
}

fn deobfuscate(untwist: &Path, file: &Path, flags: &[&str]) -> Result<String> {
    let out = Command::new(untwist)
        .args(flags)
        .arg(file)
        .output()
        .with_context(|| format!("spawn {}", untwist.display()))?;
    if !out.status.success() {
        bail!(
            "untwist failed on {}: {}",
            file.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn render(exit: Option<i32>, stdout: &str, stderr: &str) -> String {
    format!("exit={exit:?}\nstdout={stdout}\nstderr={stderr}")
}

fn output_path(tmp: &Path, file: &Path, mode: &str) -> Result<PathBuf> {
    let name = file
        .file_name()
        .with_context(|| format!("fixture file name for {}", file.display()))?;
    Ok(tmp.join(format!("{}.{mode}.js", Path::new(name).display())))
}

fn matches(before: &Observed, before_stderr: &str, after: &Observed, after_stderr: &str) -> bool {
    before.exit == after.exit && before.stdout == after.stdout && before_stderr == after_stderr
}

struct RunContext<'a> {
    node: &'a Path,
    untwist: &'a Path,
    tmp: &'a Path,
    file: &'a Path,
}

fn run_mode(
    ctx: &RunContext,
    (mode, flags): (&'static str, &[&str]),
    before: &Observed,
    before_stderr: &str,
) -> Result<Option<LaneMismatch>> {
    let deobfuscated = deobfuscate(ctx.untwist, ctx.file, flags)?;
    let path = output_path(ctx.tmp, ctx.file, mode)?;
    std::fs::write(&path, &deobfuscated).with_context(|| format!("write {}", path.display()))?;
    let before_text = render(before.exit, &before.stdout, before_stderr);
    let Some(after) = run(ctx.node, &path)? else {
        return Ok(Some(LaneMismatch {
            lane: mode,
            before: before_text,
            after: "did not terminate within 10s".to_string(),
        }));
    };
    let after_stderr = normalise_stderr(&after.stderr, &[path.as_path()]);
    if matches(before, before_stderr, &after, &after_stderr) {
        return Ok(None);
    }
    Ok(Some(LaneMismatch {
        lane: mode,
        before: before_text,
        after: render(after.exit, &after.stdout, &after_stderr),
    }))
}

fn diff_one(node: &Path, untwist: &Path, tmp: &Path, file: &Path) -> Result<Behaviour> {
    let canonical =
        std::fs::canonicalize(file).with_context(|| format!("canonicalize {}", file.display()))?;
    let file = canonical.as_path();
    let Some(before) = run(node, file)? else {
        return Ok(Behaviour::Skipped("input did not terminate"));
    };
    if before.exit != Some(0) && before.stdout.is_empty() {
        return Ok(Behaviour::Skipped("input throws before printing anything"));
    }
    let before_stderr = normalise_stderr(&before.stderr, &[file]);
    let ctx = RunContext {
        node,
        untwist,
        tmp,
        file,
    };
    let mut mismatches = Vec::new();
    for lane in LANES {
        if let Some(mismatch) = run_mode(&ctx, lane, &before, &before_stderr)? {
            mismatches.push(mismatch);
        }
    }
    if mismatches.is_empty() {
        return Ok(Behaviour::Match);
    }
    Ok(Behaviour::Mismatch(mismatches))
}

pub fn diff_behaviour(repo_root: &Path, files: &[PathBuf]) -> Result<Vec<(PathBuf, Behaviour)>> {
    let Some(node_path) = node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        if skip_allowed(required.as_deref()) {
            println!("{SKIP_MESSAGE}");
            return Ok(Vec::new());
        }
        bail!("required: {SKIP_MESSAGE}");
    };
    let untwist = repo_root.join("target/release/untwist");
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let tmp = tempfile::tempdir().context("create temp dir")?;
    let mut outcomes = Vec::new();
    for file in files {
        let behaviour = diff_one(&node_path, &untwist, tmp.path(), file)?;
        outcomes.push((file.clone(), behaviour));
    }
    Ok(outcomes)
}
