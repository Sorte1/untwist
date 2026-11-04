use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Oracle {
        file: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        flags: Vec<String>,
    },
    BenchBaseline,
    BenchCompare {
        filter: Option<String>,
    },
    DiffUnminify {
        files: Vec<PathBuf>,
    },
    DiffUnpack {
        files: Vec<PathBuf>,
    },
    DiffDeobfuscate {
        files: Vec<PathBuf>,
    },
    DiffMangle {
        files: Vec<PathBuf>,
    },
    DiffJsx {
        files: Vec<PathBuf>,
    },
    DiffBehaviour {
        files: Vec<PathBuf>,
    },
    DiffUnpackBehaviour(xtask::unpack_behaviour::Request),
    ObfMatrix(xtask::obf_matrix::Request),
    ObfOptions(xtask::obf_options::Request),
    CorpusExport(xtask::corpus_export::Request),
    Corpus,
    CorpusNpm {
        seed: Option<u64>,
    },
    DiffEval {
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 20_000)]
        cases: usize,
    },
    CorpusBundle {
        modules: Option<usize>,
        seed: Option<u64>,
        #[arg(long, default_value = "webpack")]
        kind: String,
    },
    DepthDiff(xtask::depth_diff::Request),
    FreezeRecords {
        out: PathBuf,
    },
    DepthOracle(xtask::depth_oracle::Request),
    #[command(hide = true)]
    DepthOracleChild(xtask::depth_oracle::ChildRequest),
    #[command(flatten)]
    Layers(xtask::layers::Command),
    Bump {
        version: String,
    },
    CheckVersion {
        tag: String,
    },
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

fn freeze_records(out: &Path) -> Result<()> {
    std::fs::create_dir_all(out).with_context(|| out.display().to_string())?;
    let out = out.canonicalize()?;
    let lines = xtask::freeze_records::run(&repo_root(), &out)?;
    println!(
        "froze {lines} unit records into {}; full records under {}",
        xtask::freeze_records::DATA,
        out.display()
    );
    Ok(())
}

fn reference_cli() -> Result<PathBuf> {
    xtask::reference::cli(&repo_root())
}

fn run_oracle(file: &Path, flags: &[String]) -> Result<String> {
    let out = Command::new("node")
        .arg(reference_cli()?)
        .args(flags)
        .arg(file)
        .output()
        .context("spawn node")?;
    if !out.status.success() {
        bail!(
            "the reference deobfuscator failed on {}: {}",
            file.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn fixture_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for dir in ["deobfuscate", "unpack"] {
        for entry in std::fs::read_dir(repo_root().join("fixtures/reference").join(dir))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "js") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn bench_baseline() -> Result<()> {
    let mut results = serde_json::Map::new();
    let mut files = fixture_files()?;
    files.extend(untwist_testing::real_world_path("game.js"));
    for file in files {
        let start = Instant::now();
        run_oracle(&file, &["--no-unpack".to_string()])?;
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let name = file
            .strip_prefix(repo_root())
            .unwrap_or(&file)
            .display()
            .to_string();
        results.insert(name, serde_json::json!(ms));
    }
    let path = repo_root().join("benches/baseline-reference.json");
    std::fs::create_dir_all(path.parent().context("bench dir")?)?;
    std::fs::write(&path, serde_json::to_string_pretty(&results)?)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn default_diff_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for dir in ["fixtures/unminify", "fixtures/reference/unpack"] {
        let dir = repo_root().join(dir);
        for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let path = entry?.path();
            if path.extension().is_some_and(|e| e == "js") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn print_outcome(outcome: &xtask::diff::FixtureOutcome) -> bool {
    let rel = outcome
        .file
        .strip_prefix(repo_root())
        .unwrap_or(&outcome.file)
        .display();
    match &outcome.outcome {
        xtask::diff::Outcome::Identical => {
            println!("identical    {rel}");
            false
        }
        xtask::diff::Outcome::Allowlisted => {
            println!("allowlisted  {rel}");
            false
        }
        xtask::diff::Outcome::StaleAllowlist => {
            println!("identical    {rel}");
            println!("warning: {rel} matches the reference deobfuscator; remove its stale allowlist entry");
            false
        }
        xtask::diff::Outcome::Failing(diff) => {
            println!("failing      {rel}");
            print!("{diff}");
            true
        }
    }
}

fn report_outcomes(outcomes: Vec<xtask::diff::FixtureOutcome>) -> Result<()> {
    let failing = outcomes.iter().filter(|o| print_outcome(o)).count();
    let allowlisted = outcomes
        .iter()
        .filter(|o| matches!(o.outcome, xtask::diff::Outcome::Allowlisted))
        .count();
    let identical = outcomes.len() - failing - allowlisted;
    println!(
        "{} fixtures, {identical} identical, {allowlisted} allowlisted, {failing} failing",
        outcomes.len()
    );
    if failing > 0 {
        bail!(
            "{failing} fixture(s) have unallowlisted differences from the reference deobfuscator"
        );
    }
    Ok(())
}

fn run_diff_unminify(files: Vec<PathBuf>) -> Result<()> {
    let files = if files.is_empty() {
        default_diff_files()?
    } else {
        files
    };
    let allowlist = xtask::diff::read_allowlist(&repo_root().join("xtask/diff-allowlist.txt"))?;
    let outcomes = xtask::diff::diff_unminify(&repo_root(), &files, &allowlist)?;
    report_outcomes(outcomes)
}

fn js_fixture_files_under(subdir: &str) -> Result<Vec<PathBuf>> {
    let dir = repo_root().join(subdir);
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "js") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn js_fixture_files_below(subdir: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut pending = vec![repo_root().join(subdir)];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "js") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn unpack_fixture_files() -> Result<Vec<PathBuf>> {
    js_fixture_files_under("fixtures/reference/unpack")
}

fn run_diff_unpack(files: Vec<PathBuf>) -> Result<()> {
    let files = if files.is_empty() {
        unpack_fixture_files()?
    } else {
        files
    };
    let allowlist =
        xtask::diff::read_allowlist(&repo_root().join("xtask/diff-unpack-allowlist.txt"))?;
    let outcomes = xtask::diff::diff_unpack(&repo_root(), &files, &allowlist)?;
    report_outcomes(outcomes)
}

fn deobfuscate_fixture_files() -> Result<Vec<PathBuf>> {
    let mut files = js_fixture_files_under("fixtures/reference/deobfuscate")?;
    files.extend(untwist_testing::real_world_path("game.js"));
    Ok(files)
}

fn run_diff_deobfuscate(files: Vec<PathBuf>) -> Result<()> {
    let files = if files.is_empty() {
        deobfuscate_fixture_files()?
    } else {
        files
    };
    let allowlist =
        xtask::diff::read_allowlist(&repo_root().join("xtask/diff-deobfuscate-allowlist.txt"))?;
    let outcomes = xtask::diff::diff_deobfuscate(&repo_root(), &files, &allowlist)?;
    report_outcomes(outcomes)
}

const MANGLE_CORPORA: [&str; 3] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
];

fn mangle_fixture_files() -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for corpus in MANGLE_CORPORA {
        files.extend(js_fixture_files_under(corpus)?);
    }
    Ok(files)
}

fn run_diff_mangle(files: Vec<PathBuf>) -> Result<()> {
    let files = if files.is_empty() {
        mangle_fixture_files()?
    } else {
        files
    };
    let allowlist =
        xtask::diff::read_allowlist(&repo_root().join("xtask/diff-mangle-allowlist.txt"))?;
    let outcomes = xtask::diff::diff_mangle(&repo_root(), &files, &allowlist)?;
    report_outcomes(outcomes)
}

fn jsx_fixture_files() -> Result<Vec<PathBuf>> {
    js_fixture_files_under("fixtures/jsx")
}

fn run_diff_jsx(files: Vec<PathBuf>) -> Result<()> {
    let files = if files.is_empty() {
        jsx_fixture_files()?
    } else {
        files
    };
    let allowlist = xtask::diff::read_allowlist(&repo_root().join("xtask/diff-jsx-allowlist.txt"))?;
    let outcomes = xtask::diff::diff_jsx(&repo_root(), &files, &allowlist)?;
    report_outcomes(outcomes)
}

fn behaviour_fixture_files() -> Result<Vec<PathBuf>> {
    let mut files = js_fixture_files_under("fixtures/reference/deobfuscate")?;
    files.extend(js_fixture_files_under("fixtures/behaviour")?);
    files.extend(js_fixture_files_below("fixtures/untranspile")?);
    Ok(files)
}

#[derive(Default)]
struct BehaviourTally {
    skipped: usize,
    allowed: usize,
    mismatched: usize,
    seen: std::collections::HashSet<String>,
}

fn print_lane_mismatch(rel: &str, mismatch: &xtask::behaviour::LaneMismatch, key: &str) {
    println!("mismatched   {rel} lane={}", mismatch.lane);
    println!("before:\n{}", mismatch.before);
    println!("after:\n{}", mismatch.after);
    println!("not in the allowlist: {key}");
}

fn tally_behaviour(
    file: &Path,
    behaviour: &xtask::behaviour::Behaviour,
    allowlist: &std::collections::HashSet<String>,
    tally: &mut BehaviourTally,
) {
    let rel = file
        .strip_prefix(repo_root())
        .unwrap_or(file)
        .display()
        .to_string();
    match behaviour {
        xtask::behaviour::Behaviour::Match => println!("behaviour ok {rel}"),
        xtask::behaviour::Behaviour::Skipped(reason) => {
            println!("skipped      {rel}: {reason}");
            tally.skipped += 1;
        }
        xtask::behaviour::Behaviour::Mismatch(mismatches) => {
            for mismatch in mismatches {
                let key = format!("{rel} {} {}", mismatch.lane, mismatch.digest());
                if allowlist.contains(&key) {
                    println!("allowlisted  {key}");
                    tally.allowed += 1;
                    tally.seen.insert(key);
                } else {
                    print_lane_mismatch(&rel, mismatch, &key);
                    tally.mismatched += 1;
                }
            }
        }
    }
}

fn run_diff_behaviour(files: Vec<PathBuf>) -> Result<()> {
    let whole_set = files.is_empty();
    let files = if whole_set {
        behaviour_fixture_files()?
    } else {
        files
    };
    let allowlist: std::collections::HashSet<String> =
        xtask::diff::read_allowlist(&repo_root().join("xtask/diff-behaviour-allowlist.txt"))?
            .into_iter()
            .collect();
    let outcomes = xtask::behaviour::diff_behaviour(&repo_root(), &files)?;
    if outcomes.is_empty() {
        return Ok(());
    }
    let mut tally = BehaviourTally::default();
    for (file, behaviour) in &outcomes {
        tally_behaviour(file, behaviour, &allowlist, &mut tally);
    }
    let mut stale: Vec<&String> = if whole_set {
        allowlist.difference(&tally.seen).collect()
    } else {
        Vec::new()
    };
    stale.sort();
    for key in &stale {
        println!("stale allowlist line (the mismatch is gone or changed): {key}");
    }
    println!(
        "{} fixtures, {} skipped, {} allowlisted lane mismatches, {} mismatched, {} stale",
        outcomes.len(),
        tally.skipped,
        tally.allowed,
        tally.mismatched,
        stale.len()
    );
    if tally.mismatched > 0 || !stale.is_empty() {
        bail!(
            "{} lane mismatch(es) outside xtask/diff-behaviour-allowlist.txt, {} stale line(s)",
            tally.mismatched,
            stale.len()
        );
    }
    Ok(())
}

fn today() -> Result<String> {
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
    let days = i64::try_from(since_epoch.as_secs() / 86_400)?;
    Ok(xtask::release::civil_date(days))
}

fn bump(version: &str) -> Result<()> {
    let root = repo_root();
    let version = xtask::release::bump(&root, version, &today()?)?;
    let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["update", "--workspace"])
        .current_dir(&root)
        .status()
        .context("spawn cargo update")?;
    if !status.success() {
        bail!("cargo update --workspace failed; Cargo.lock still has the old version");
    }
    println!("bumped to {version}; commit, then tag v{version} and push with --follow-tags");
    Ok(())
}

fn run_diff_eval(seed: u64, cases: usize) -> Result<()> {
    let Some(node) = untwist_testing::node::node() else {
        bail!("diff-eval needs node on PATH (or UNTWIST_NODE)");
    };
    let outcome = xtask::diff_eval::run(&node, seed, cases)?;
    if !outcome.mismatches.is_empty() {
        bail!(
            "{} cases answered differently from node",
            outcome.mismatches.len()
        );
    }
    Ok(())
}

fn main() -> Result<()> {
    match Args::parse().command {
        Cmd::Oracle { file, flags } => {
            print!("{}", run_oracle(&file, &flags)?);
            Ok(())
        }
        Cmd::BenchBaseline => bench_baseline(),
        Cmd::BenchCompare { filter } => {
            xtask::bench::bench_compare(&repo_root(), filter.as_deref())
        }
        Cmd::DiffUnminify { files } => run_diff_unminify(files),
        Cmd::DiffUnpack { files } => run_diff_unpack(files),
        Cmd::DiffDeobfuscate { files } => run_diff_deobfuscate(files),
        Cmd::DiffMangle { files } => run_diff_mangle(files),
        Cmd::DiffJsx { files } => run_diff_jsx(files),
        Cmd::DiffBehaviour { files } => run_diff_behaviour(files),
        Cmd::DiffUnpackBehaviour(request) => xtask::unpack_behaviour::run(&repo_root(), &request),
        Cmd::ObfMatrix(request) => xtask::obf_matrix::run(&repo_root(), &request),
        Cmd::ObfOptions(request) => xtask::obf_options::run(&repo_root(), &request),
        Cmd::CorpusExport(request) => xtask::corpus_export::run(&repo_root(), &request),
        Cmd::DiffEval { seed, cases } => run_diff_eval(seed, cases),
        Cmd::Corpus => {
            let count = xtask::corpus::generate(&repo_root())?;
            println!("wrote {count} corpus cases to fixtures/eval");
            Ok(())
        }
        Cmd::CorpusNpm { seed } => {
            let seed = seed.unwrap_or(xtask::corpus_npm::DEFAULT_SEED);
            let count = xtask::corpus_npm::generate(&repo_root(), seed)?;
            println!("wrote {count} cases to fixtures/corpus-npm");
            Ok(())
        }
        Cmd::CorpusBundle {
            modules,
            seed,
            kind,
        } => {
            let modules = modules.unwrap_or(xtask::corpus_bundle::DEFAULT_MODULES);
            let seed = seed.unwrap_or(xtask::corpus_bundle::DEFAULT_SEED);
            let bytes = xtask::corpus_bundle::generate_kind(&repo_root(), &kind, modules, seed)?;
            println!(
                "wrote {bytes} bytes to {}",
                xtask::corpus_bundle::output_for(&kind)
            );
            Ok(())
        }
        Cmd::DepthDiff(request) => xtask::depth_diff::run(&request),
        Cmd::FreezeRecords { out } => freeze_records(&out),
        Cmd::DepthOracle(request) => xtask::depth_oracle::run(&request),
        Cmd::DepthOracleChild(child) => xtask::depth_oracle::run_child(&child),
        Cmd::Layers(command) => command.run(&repo_root()),
        Cmd::Bump { version } => bump(&version),
        Cmd::CheckVersion { tag } => {
            xtask::release::check_version(&repo_root(), &tag)?;
            println!("{tag} matches every version");
            Ok(())
        }
    }
}
