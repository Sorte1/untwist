use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::spec::{Spec, SPEC};
use crate::obf_matrix::cell::wait_with_timeout;
use crate::obf_matrix::pool;

pub const TOOL: &str = "tools/obf-fuzz/fuzz.mjs";
pub const PINS: &str = "tools/obfuscated/node_modules/javascript-obfuscator-5";
const SAMPLE_BUDGET: Duration = Duration::from_secs(60);
const CHUNK: usize = 25;

fn node_command(root: &Path, node: &Path) -> Command {
    let mut command = Command::new(node);
    command
        .arg("--no-warnings")
        .arg(root.join(TOOL))
        .current_dir(root)
        .stdin(Stdio::null());
    command
}

fn finish(mut command: Command, limit: Duration, what: &str) -> Result<String> {
    let dir = tempfile::tempdir()?;
    let out = dir.path().join("out.txt");
    let err = dir.path().join("err.txt");
    let mut child = command
        .stdout(std::fs::File::create(&out)?)
        .stderr(std::fs::File::create(&err)?)
        .spawn()
        .with_context(|| format!("spawn node for {what}"))?;
    let exit = wait_with_timeout(&mut child, limit)?;
    let stdout = std::fs::read_to_string(&out).unwrap_or_default();
    if exit != Some(0) {
        let stderr = std::fs::read_to_string(&err).unwrap_or_default();
        bail!(
            "{what}: node exit {exit:?}: {}",
            stderr.lines().take(5).collect::<Vec<_>>().join(" | ")
        );
    }
    Ok(stdout)
}

pub fn programs(root: &Path, node: &Path, seed: &str, count: usize) -> Result<String> {
    let mut command = node_command(root, node);
    command.args(["programs", "--seed", seed, "--count", &count.to_string()]);
    finish(command, SAMPLE_BUDGET, "obf-fuzz programs")
}

fn shards(count: usize) -> Vec<(usize, usize)> {
    (0..count)
        .step_by(CHUNK)
        .map(|start| (start, CHUNK.min(count - start)))
        .collect()
}

pub fn generate(
    root: &Path,
    node: &Path,
    (seed, count): (&str, usize),
    jobs: usize,
    out: &Path,
) -> Result<Vec<PathBuf>> {
    if !root.join(PINS).is_dir() {
        bail!("install the pinned obfuscators first: npm ci --prefix tools/obfuscated");
    }
    std::fs::create_dir_all(out)?;
    let results = pool(shards(count), jobs, |(start, n)| {
        let mut command = node_command(root, node);
        command
            .args(["generate", "--seed", seed])
            .args(["--start", &start.to_string(), "--count", &n.to_string()])
            .arg("--out")
            .arg(out);
        finish(command, SAMPLE_BUDGET * n as u32, "obf-fuzz generate")
    });
    for result in results {
        result?;
    }
    Ok((0..count)
        .map(|index| out.join(format!("s{seed}-{index}")))
        .collect())
}

pub fn rebuild(root: &Path, node: &Path, raw: &Value, dir: &Path) -> Result<Spec> {
    let input = dir.with_extension("in.json");
    std::fs::write(&input, serde_json::to_string(raw)?)?;
    let mut command = node_command(root, node);
    command
        .arg("rebuild")
        .arg("--spec")
        .arg(&input)
        .arg("--out")
        .arg(dir);
    finish(command, SAMPLE_BUDGET, "obf-fuzz rebuild")?;
    let _ = std::fs::remove_file(&input);
    if !dir.join(SPEC).is_file() {
        bail!("obf-fuzz rebuild wrote no {SPEC} in {}", dir.display());
    }
    Spec::load(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    #[test]
    fn shards_cover_every_index_once() {
        assert_eq!(shards(60), vec![(0, 25), (25, 25), (50, 10)]);
        assert!(shards(0).is_empty());
    }

    #[test]
    fn the_same_seed_draws_the_same_programs_and_another_seed_does_not() {
        let Some(node) = untwist_testing::node::node() else {
            return;
        };
        let root = repo();
        let first = programs(&root, &node, "11", 30).unwrap();
        let again = programs(&root, &node, "11", 30).unwrap();
        let other = programs(&root, &node, "12", 30).unwrap();
        assert_eq!(first, again);
        assert_ne!(first, other);
        assert_eq!(first.lines().count(), 30);
        let all: Vec<Value> = first
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(all
            .iter()
            .all(|p| p["features"].as_array().is_some_and(|f| !f.is_empty())));
        let forms: std::collections::BTreeSet<&str> =
            all.iter().filter_map(|p| p["form"].as_str()).collect();
        assert!(forms.len() >= 3, "{forms:?}");
    }
}
