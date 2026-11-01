use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

pub const DATA: &str = "crates/untwist-eval/tests/data/compile_records.sha256";
const FREEZE_DIR: &str = "UNTWIST_FREEZE_RECORDS";

const EVAL_TESTS: [&str; 17] = [
    "arguments_frames",
    "compile_deep",
    "compile_deep_statements",
    "compile_depth",
    "compile_differential",
    "compile_entries",
    "compile_entry_sessions",
    "compile_expressions",
    "compile_flow",
    "compile_forms",
    "compile_forms_deep",
    "compile_held_values",
    "compile_refusals",
    "compile_statement_fuzz",
    "compile_statements",
    "fuel_exact",
    "konst",
];

struct Step {
    release: bool,
    package: &'static str,
    tests: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

const STEPS: [Step; 3] = [
    Step {
        release: false,
        package: "untwist-eval",
        tests: &EVAL_TESTS,
        env: &[],
    },
    Step {
        release: true,
        package: "untwist-eval",
        tests: &EVAL_TESTS,
        env: &[],
    },
    Step {
        release: true,
        package: "untwist-deobfuscate",
        tests: &["compile_differential"],
        env: &[("UNTWIST_REQUIRE_CORPUS", "1")],
    },
];

fn run_step(root: &Path, out: &Path, step: &Step) -> Result<()> {
    let mut command = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    command
        .current_dir(root)
        .args(["test", "-j", "8", "-p", step.package]);
    if step.release {
        command.arg("--release");
    }
    for test in step.tests {
        command.args(["--test", test]);
    }
    command.env(FREEZE_DIR, out).envs(step.env.iter().copied());
    let status = command.status().context("cargo test")?;
    if !status.success() {
        bail!(
            "{} (release {}) failed: {status}",
            step.package,
            step.release
        );
    }
    Ok(())
}

fn ordered(line: &str) -> (String, u64) {
    let mut fields = line.split(' ');
    let key = fields.next().unwrap_or_default().to_string();
    let index = fields.next().and_then(|index| index.parse().ok());
    (key, index.unwrap_or(u64::MAX))
}

fn assemble(out: &Path) -> Result<Vec<String>> {
    let mut lines = Vec::new();
    for entry in std::fs::read_dir(out.join("lines")).context("the frozen lines")? {
        let text = std::fs::read_to_string(entry?.path())?;
        lines.extend(text.lines().map(str::to_string));
    }
    lines.sort_by_key(|line| ordered(line));
    Ok(lines)
}

pub fn run(root: &Path, out: &Path) -> Result<usize> {
    if out.join("lines").exists() {
        bail!(
            "{} already holds frozen lines; pass an empty directory",
            out.display()
        );
    }
    for step in &STEPS {
        run_step(root, out, step)?;
    }
    let lines = assemble(out)?;
    let mut text = lines.join("\n");
    text.push('\n');
    std::fs::write(root.join(DATA), text).with_context(|| DATA.to_string())?;
    Ok(lines.len())
}
