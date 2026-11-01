use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result};

const COMPARE_FLAGS: [&str; 4] = ["--unsafe", "--conservative", "--no-unpack", "--no-jsx"];
const DEFAULT_MODE_FLAGS: [&str; 3] = ["--unsafe", "--no-unpack", "--no-jsx"];
const RUNS_PER_FIXTURE: u32 = 20;
const GAME_JS: &str = "fixtures/real-world/game.js";
const GAME_RUNS: u32 = 3;

fn one_run(untwist: &Path, flags: &[&str], file: &Path) -> Result<f64> {
    let start = Instant::now();
    let out = Command::new(untwist)
        .args(flags)
        .arg(file)
        .output()
        .with_context(|| format!("run {}", untwist.display()))?;
    if !out.status.success() {
        anyhow::bail!(
            "{} failed on {}: {}",
            untwist.display(),
            file.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(start.elapsed().as_secs_f64() * 1000.0)
}

fn mean_of_runs(untwist: &Path, flags: &[&str], file: &Path, runs: u32) -> Result<f64> {
    let mut total = 0.0;
    for _ in 0..runs {
        total += one_run(untwist, flags, file)?;
    }
    Ok(total / f64::from(runs))
}

fn median_of_runs(untwist: &Path, flags: &[&str], file: &Path, runs: u32) -> Result<f64> {
    let mut samples = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        samples.push(one_run(untwist, flags, file)?);
    }
    samples.sort_by(f64::total_cmp);
    Ok(samples.get(samples.len() / 2).copied().unwrap_or(0.0))
}

fn time_untwist(untwist: &Path, name: &str, file: &Path) -> Result<Vec<(String, f64)>> {
    if name != GAME_JS {
        let ms = mean_of_runs(untwist, &COMPARE_FLAGS, file, RUNS_PER_FIXTURE)?;
        return Ok(vec![(name.to_string(), ms)]);
    }
    let conservative = median_of_runs(untwist, &COMPARE_FLAGS, file, GAME_RUNS)?;
    let default_mode = median_of_runs(untwist, &DEFAULT_MODE_FLAGS, file, GAME_RUNS)?;
    Ok(vec![
        (
            format!("{name} (--conservative, median of {GAME_RUNS})"),
            conservative,
        ),
        (
            format!("{name} (default, median of {GAME_RUNS})"),
            default_mode,
        ),
    ])
}

pub fn bench_compare(repo_root: &Path, filter: Option<&str>) -> Result<()> {
    let baseline_path = repo_root.join("benches/baseline-reference.json");
    let baseline_text = std::fs::read_to_string(&baseline_path)
        .context("run `cargo xtask bench-baseline` first")?;
    let baseline: serde_json::Value =
        serde_json::from_str(&baseline_text).context("parse benches/baseline-reference.json")?;
    let untwist = repo_root.join("target/release/untwist");
    if !untwist.exists() {
        anyhow::bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    println!(
        "{:<60} {:>12} {:>12} {:>8}",
        "fixture", "reference ms", "untwist ms", "speedup"
    );
    for (name, reference_ms) in baseline.as_object().context("baseline is not an object")? {
        if filter.is_some_and(|needle| !name.contains(needle)) {
            continue;
        }
        let theirs = reference_ms
            .as_f64()
            .with_context(|| format!("{name}: baseline value is not a number"))?;
        for (label, ours) in time_untwist(&untwist, name, &repo_root.join(name))? {
            println!(
                "{:<60} {:>12.1} {:>12.2} {:>7.1}x",
                label,
                theirs,
                ours,
                theirs / ours
            );
        }
    }
    Ok(())
}
