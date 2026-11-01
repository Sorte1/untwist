use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::{print, PrintStyle};

pub fn normalize(code: &str) -> String {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, code);
    print(&parsed.program, PrintStyle::Compact)
}

const UNMINIFY_ONLY: [&str; 3] = ["--no-deobfuscate", "--no-unpack", "--no-jsx"];

fn run_plain(program: &Path, extra: &[&str], file: &Path) -> Result<String> {
    let out = Command::new(program)
        .args(extra)
        .arg(file)
        .output()
        .with_context(|| format!("spawn {}", program.display()))?;
    if !out.status.success() {
        bail!(
            "{} failed on {}: {}",
            program.display(),
            file.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn run_tool(program: &Path, extra: &[&str], file: &Path) -> Result<String> {
    let mut args: Vec<&str> = extra.to_vec();
    args.extend_from_slice(&UNMINIFY_ONLY);
    run_plain(program, &args, file)
}

pub enum Outcome {
    Identical,
    Allowlisted,
    StaleAllowlist,
    Failing(String),
}

pub struct FixtureOutcome {
    pub file: PathBuf,
    pub outcome: Outcome,
}

fn reference_cli(repo_root: &Path) -> Result<String> {
    crate::reference::cli(repo_root)?
        .to_str()
        .map(str::to_string)
        .context("reference cli path is not utf8")
}

fn run_reference(repo_root: &Path, extra: &[&str], file: &Path) -> Result<String> {
    let cli_path = reference_cli(repo_root)?;
    let mut args: Vec<&str> = vec![cli_path.as_str()];
    args.extend_from_slice(extra);
    run_tool(Path::new("node"), &args, file)
}

const CONSERVATIVE: &str = "--conservative";

fn run_untwist(untwist: &Path, file: &Path) -> Result<String> {
    run_tool(untwist, &[CONSERVATIVE], file)
}

fn relative_to(repo_root: &Path, file: &Path) -> String {
    file.strip_prefix(repo_root)
        .unwrap_or(file)
        .display()
        .to_string()
}

fn classify(expected: &str, actual: &str, allowlisted: bool) -> Outcome {
    if expected == actual {
        return if allowlisted {
            Outcome::StaleAllowlist
        } else {
            Outcome::Identical
        };
    }
    if allowlisted {
        return Outcome::Allowlisted;
    }
    let diff = similar::TextDiff::from_lines(expected, actual)
        .unified_diff()
        .header("reference", "untwist")
        .to_string();
    Outcome::Failing(diff)
}

fn compare_fixtures(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
    run: impl Fn(&Path, &Path) -> Result<(String, String)>,
) -> Result<Vec<FixtureOutcome>> {
    let untwist = repo_root.join("target/release/untwist");
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let mut outcomes = Vec::new();
    for file in files {
        let (expected, actual) = run(&untwist, file)?;
        let allowlisted = allowlist.iter().any(|a| a == &relative_to(repo_root, file));
        outcomes.push(FixtureOutcome {
            file: file.clone(),
            outcome: classify(&expected, &actual, allowlisted),
        });
    }
    Ok(outcomes)
}

pub fn diff_unminify(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
) -> Result<Vec<FixtureOutcome>> {
    compare_fixtures(repo_root, files, allowlist, |untwist, file| {
        Ok((
            normalize(&run_reference(repo_root, &[], file)?),
            normalize(&run_untwist(untwist, file)?),
        ))
    })
}

pub fn read_allowlist(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read allowlist {}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect())
}

pub fn normalize_module_path(path: &str) -> String {
    path.strip_prefix("./").unwrap_or(path).to_string()
}

struct ToolRun {
    ok: bool,
    stderr: String,
}

fn run_into(program: &Path, prefix: &[&str], file: &Path, dir: &Path) -> Result<ToolRun> {
    let out = Command::new(program)
        .args(prefix)
        .arg("-o")
        .arg(dir)
        .arg("-f")
        .arg(file)
        .output()
        .with_context(|| format!("spawn {}", program.display()))?;
    Ok(ToolRun {
        ok: out.status.success(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

struct BundleFacts {
    kind: String,
    entry: String,
    modules: BTreeMap<String, String>,
}

fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn read_bundle(dir: &Path) -> Result<Option<BundleFacts>> {
    let path = dir.join("bundle.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err).with_context(|| format!("read {}", path.display())),
    };
    let json: serde_json::Value = serde_json::from_str(&text).context("parse bundle.json")?;
    let mut modules = BTreeMap::new();
    if let Some(list) = json.get("modules").and_then(serde_json::Value::as_array) {
        for module in list {
            modules.insert(
                json_str(module, "id"),
                normalize_module_path(&json_str(module, "path")),
            );
        }
    }
    Ok(Some(BundleFacts {
        kind: json_str(&json, "type"),
        entry: json_str(&json, "entryId"),
        modules,
    }))
}

fn compare_module(
    id: &str,
    expected_dir: &Path,
    actual_dir: &Path,
    path: &str,
    report: &mut String,
) -> Result<()> {
    let expected_path = expected_dir.join(path);
    let actual_path = actual_dir.join(path);
    let expected = normalize(
        &std::fs::read_to_string(&expected_path)
            .with_context(|| format!("read {}", expected_path.display()))?,
    );
    let actual = normalize(
        &std::fs::read_to_string(&actual_path)
            .with_context(|| format!("read {}", actual_path.display()))?,
    );
    if expected == actual {
        return Ok(());
    }
    let left = format!("reference {id}");
    let right = format!("untwist {id}");
    report.push_str(
        &similar::TextDiff::from_lines(&expected, &actual)
            .unified_diff()
            .header(&left, &right)
            .to_string(),
    );
    Ok(())
}

fn compare_present_bundles(
    expected: &BundleFacts,
    actual: &BundleFacts,
    expected_dir: &Path,
    actual_dir: &Path,
) -> Result<Option<String>> {
    let mut report = String::new();
    if expected.kind != actual.kind {
        report.push_str(&format!(
            "type: reference {:?} untwist {:?}\n",
            expected.kind, actual.kind
        ));
    }
    if expected.entry != actual.entry {
        report.push_str(&format!(
            "entryId: reference {:?} untwist {:?}\n",
            expected.entry, actual.entry
        ));
    }
    for (id, path) in &expected.modules {
        match actual.modules.get(id) {
            None => report.push_str(&format!("module {id}: missing from untwist\n")),
            Some(actual_path) if actual_path != path => report.push_str(&format!(
                "module {id}: reference {path:?} untwist {actual_path:?}\n"
            )),
            Some(actual_path) => {
                compare_module(id, expected_dir, actual_dir, actual_path, &mut report)?;
            }
        }
    }
    for id in actual.modules.keys() {
        if !expected.modules.contains_key(id) {
            report.push_str(&format!(
                "module {id}: not produced by the reference deobfuscator\n"
            ));
        }
    }
    Ok((!report.is_empty()).then_some(report))
}

fn compare_bundles(
    fixture: &str,
    expected_dir: &Path,
    actual_dir: &Path,
) -> Result<Option<String>> {
    match (read_bundle(expected_dir)?, read_bundle(actual_dir)?) {
        (None, None) => Ok(None),
        (None, Some(_)) => Ok(Some(format!(
            "{fixture}: the reference deobfuscator produced no bundle.json (no bundle detected); untwist detected a bundle\n"
        ))),
        (Some(_), None) => Ok(Some(format!(
            "{fixture}: untwist produced no bundle.json (no bundle detected); the reference deobfuscator detected a bundle\n"
        ))),
        (Some(expected), Some(actual)) => {
            compare_present_bundles(&expected, &actual, expected_dir, actual_dir)
        }
    }
}

fn classify_report(report: String, allowlisted: bool) -> Outcome {
    if allowlisted {
        Outcome::Allowlisted
    } else {
        Outcome::Failing(report)
    }
}

pub fn outcome_for_dirs(
    fixture: &str,
    expected_dir: &Path,
    actual_dir: &Path,
    allowlisted: bool,
) -> Result<Outcome> {
    match compare_bundles(fixture, expected_dir, actual_dir)? {
        None if allowlisted => Ok(Outcome::StaleAllowlist),
        None => Ok(Outcome::Identical),
        Some(report) => Ok(classify_report(report, allowlisted)),
    }
}

pub fn diff_unpack(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
) -> Result<Vec<FixtureOutcome>> {
    let untwist = repo_root.join("target/release/untwist");
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let cli_path = reference_cli(repo_root)?;
    let mut outcomes = Vec::new();
    for file in files {
        let allowlisted = allowlist.iter().any(|a| a == &relative_to(repo_root, file));
        outcomes.push(FixtureOutcome {
            file: file.clone(),
            outcome: unpack_outcome(repo_root, &untwist, &cli_path, file, allowlisted)?,
        });
    }
    Ok(outcomes)
}

fn unpack_outcome(
    repo_root: &Path,
    untwist: &Path,
    cli_path: &str,
    file: &Path,
    allowlisted: bool,
) -> Result<Outcome> {
    let fixture = relative_to(repo_root, file);
    let expected_dir = tempfile::tempdir().context("expected temp dir")?;
    let actual_dir = tempfile::tempdir().context("actual temp dir")?;
    let expected_run = run_into(Path::new("node"), &[cli_path], file, expected_dir.path())?;
    let actual_run = run_into(untwist, &[CONSERVATIVE], file, actual_dir.path())?;
    if !expected_run.ok {
        return Ok(classify_report(
            format!(
                "the reference deobfuscator refused this fixture: {}",
                expected_run.stderr
            ),
            allowlisted,
        ));
    }
    if !actual_run.ok {
        return Ok(classify_report(
            format!("untwist failed: {}", actual_run.stderr),
            allowlisted,
        ));
    }
    outcome_for_dirs(
        &fixture,
        expected_dir.path(),
        actual_dir.path(),
        allowlisted,
    )
}

const DEOBFUSCATE_FLAGS: [&str; 4] = ["--unsafe", "--conservative", "--no-unpack", "--no-jsx"];

fn run_untwist_deobfuscate(untwist: &Path, file: &Path) -> Result<Result<String, String>> {
    let out = Command::new(untwist)
        .args(DEOBFUSCATE_FLAGS)
        .arg(file)
        .output()
        .with_context(|| format!("spawn {}", untwist.display()))?;
    if !out.status.success() {
        return Ok(Err(format!(
            "untwist failed on {}: {}",
            file.display(),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(Ok(String::from_utf8_lossy(&out.stdout).into_owned()))
}

pub fn diff_deobfuscate(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
) -> Result<Vec<FixtureOutcome>> {
    let untwist = repo_root.join("target/release/untwist");
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let mut outcomes = Vec::new();
    for file in files {
        let snapshot = file.with_extension("js.snap");
        let expected = std::fs::read_to_string(&snapshot)
            .with_context(|| format!("read {}", snapshot.display()))?;
        let expected = normalize(&expected);
        let allowlisted = allowlist.iter().any(|a| a == &relative_to(repo_root, file));
        let outcome = match run_untwist_deobfuscate(&untwist, file)? {
            Ok(actual) => classify(&expected, &normalize(&actual), allowlisted),
            Err(report) => Outcome::Failing(report),
        };
        outcomes.push(FixtureOutcome {
            file: file.clone(),
            outcome,
        });
    }
    Ok(outcomes)
}

pub fn diff_mangle(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
) -> Result<Vec<FixtureOutcome>> {
    compare_fixtures(repo_root, files, allowlist, |untwist, file| {
        Ok((
            normalize(&run_reference(repo_root, &["-m"], file)?),
            normalize(&run_tool(
                untwist,
                &[CONSERVATIVE, "--rename", "mangle"],
                file,
            )?),
        ))
    })
}

const JSX_REFERENCE_FLAGS: [&str; 2] = ["--no-unpack", "--no-deobfuscate"];
const JSX_FLAGS: [&str; 4] = ["--unsafe", CONSERVATIVE, "--no-unpack", "--no-deobfuscate"];

pub fn diff_jsx(
    repo_root: &Path,
    files: &[PathBuf],
    allowlist: &[String],
) -> Result<Vec<FixtureOutcome>> {
    let cli_path = reference_cli(repo_root)?;
    let mut reference_args: Vec<&str> = vec![cli_path.as_str()];
    reference_args.extend_from_slice(&JSX_REFERENCE_FLAGS);
    compare_fixtures(repo_root, files, allowlist, |untwist, file| {
        Ok((
            normalize(&run_plain(Path::new("node"), &reference_args, file)?),
            normalize(&run_plain(untwist, &JSX_FLAGS, file)?),
        ))
    })
}
