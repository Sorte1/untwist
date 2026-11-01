use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

pub const FIXTURES: &str = "fixtures/unpack-bundlers";
pub const RUNNER: &str = "tools/unpack-run.mjs";
const TIMEOUT: Duration = Duration::from_secs(20);
const UNPACK_ONLY: [&str; 3] = ["--no-deobfuscate", "--no-unminify", "--no-jsx"];
pub const MODES: [(&str, &[&str]); 2] = [("default", &[]), ("unpack-only", &UNPACK_ONLY)];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub bundler: String,
    pub loader: String,
}

#[derive(Debug)]
pub struct Fixture {
    pub kind: String,
    pub entry: Row,
    pub files: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    None,
    SwapPrints,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Observed {
    pub exit: Option<i32>,
    pub stdout: String,
}

#[derive(Debug)]
pub enum Verdict {
    Match,
    Skipped(String),
    Mismatch { expected: String, actual: String },
    CheckFailed(String),
    SplitLost(String),
}

#[derive(Debug)]
pub struct Outcome {
    pub fixture: String,
    pub mode: &'static str,
    pub bundle: String,
    pub verdict: Verdict,
}

pub fn parse_manifest(text: &str) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let fields: Vec<&str> = line.split('\t').collect();
        let [path, bundler, loader, _flags, _sha] = fields[..] else {
            bail!("MANIFEST line has {} fields: {line}", fields.len());
        };
        rows.push(Row {
            path: path.to_string(),
            bundler: bundler.to_string(),
            loader: loader.to_string(),
        });
    }
    Ok(rows)
}

fn kind_of(path: &str) -> String {
    path.split('/').next().unwrap_or_default().to_string()
}

fn fixture_dir(path: &str) -> Option<&str> {
    let parts = path.matches('/').count();
    (parts >= 2).then(|| &path[..path.rfind('/').unwrap_or(0)])
}

pub fn fixtures(rows: &[Row]) -> Vec<Fixture> {
    let entries = rows.iter().filter(|r| r.loader != "chunk");
    entries
        .map(|entry| {
            let files = match fixture_dir(&entry.path) {
                Some(dir) => rows
                    .iter()
                    .filter(|r| r.path.starts_with(&format!("{dir}/")))
                    .map(|r| r.path.clone())
                    .collect(),
                None => vec![entry.path.clone()],
            };
            Fixture {
                kind: kind_of(&entry.path),
                entry: entry.clone(),
                files,
            }
        })
        .collect()
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

fn observe(command: &mut Command) -> Result<Option<Observed>> {
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawn")?;
    let stdout = drain(child.stdout.take());
    loop {
        if let Some(status) = child.try_wait()? {
            let out = stdout.join().unwrap_or_default();
            return Ok(Some(Observed {
                exit: status.code(),
                stdout: String::from_utf8_lossy(&out).into_owned(),
            }));
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub struct Harness {
    pub node: PathBuf,
    pub untwist: PathBuf,
    pub root: PathBuf,
    pub work: PathBuf,
    pub mutation: Mutation,
    pub split_hoisted: bool,
}

impl Harness {
    fn run_entry(&self, loader: &str, entry: &Path) -> Result<Option<Observed>> {
        let runner = self.root.join(RUNNER);
        observe(
            Command::new(&self.node)
                .arg("--no-warnings")
                .arg(runner)
                .args(["--loader", loader])
                .arg(entry)
                .current_dir(entry.parent().unwrap_or(&self.root)),
        )
    }

    fn syntax_errors(&self, files: &[PathBuf]) -> Result<Vec<String>> {
        let mut errors = Vec::new();
        for file in files {
            let out = Command::new(&self.node)
                .arg("--no-warnings")
                .arg("--check")
                .arg(file)
                .output()
                .context("spawn node --check")?;
            if !out.status.success() {
                errors.push(format!(
                    "{}: {}",
                    file.display(),
                    String::from_utf8_lossy(&out.stderr)
                        .lines()
                        .take(5)
                        .collect::<Vec<_>>()
                        .join(" / ")
                ));
            }
        }
        Ok(errors)
    }

    fn untwist(&self, flags: &[&str], args: &[&Path]) -> Result<Vec<u8>> {
        let out = Command::new(&self.untwist)
            .args(self.split_hoisted.then_some("--split-hoisted"))
            .args(flags)
            .args(args)
            .output()
            .with_context(|| format!("spawn {}", self.untwist.display()))?;
        if !out.status.success() {
            bail!(
                "untwist {flags:?} {args:?} exited {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(out.stdout)
    }
}

fn js_files_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| dir.display().to_string())? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(js_files_under(&path)?);
        } else if path
            .extension()
            .is_some_and(|e| e == "js" || e == "mjs" || e == "cjs")
        {
            found.push(path);
        }
    }
    found.sort();
    Ok(found)
}

pub fn swap_first_prints(text: &str) -> Option<String> {
    let mut lines: Vec<&str> = text.lines().collect();
    let mut prints = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim_start().starts_with("console.log(") && l.trim_end().ends_with(';'))
        .map(|(i, _)| i);
    let first = prints.next()?;
    let second = prints.find(|&i| lines[i].trim() != lines[first].trim())?;
    lines.swap(first, second);
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

fn mutate(files: &[PathBuf]) -> Result<bool> {
    for file in files {
        let text = std::fs::read_to_string(file)?;
        if let Some(changed) = swap_first_prints(&text) {
            std::fs::write(file, changed)?;
            return Ok(true);
        }
    }
    Ok(false)
}

struct Output {
    loader: String,
    entry: PathBuf,
    files: Vec<PathBuf>,
    bundle: String,
}

fn bundle_label(out: &Path) -> Result<Option<String>> {
    let manifest = out.join("bundle.json");
    if !manifest.exists() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&manifest)?)?;
    let kind = value["type"].as_str().unwrap_or("?").to_string();
    let modules = value["modules"].as_array().map_or(0, Vec::len);
    Ok(Some(format!("{kind}:{modules}")))
}

fn unpacked(h: &Harness, fixture: &Fixture, flags: &[&str], dir: &Path) -> Result<Output> {
    let out = dir.join("out");
    let inputs: Vec<PathBuf> = fixture
        .files
        .iter()
        .map(|f| h.root.join(FIXTURES).join(f))
        .collect();
    let mut args: Vec<&Path> = vec![Path::new("-f"), Path::new("-o"), out.as_path()];
    args.extend(inputs.iter().map(PathBuf::as_path));
    h.untwist(flags, &args)?;
    if let Some(bundle) = bundle_label(&out)? {
        let files = js_files_under(&out)?
            .into_iter()
            .filter(|f| !f.ends_with("deobfuscated.js"))
            .collect();
        return Ok(Output {
            loader: "node".to_string(),
            entry: out.join("index.js"),
            files,
            bundle,
        });
    }
    let mirror = dir.join("mirror");
    let mut files = Vec::new();
    for (name, input) in fixture.files.iter().zip(&inputs) {
        let target = mirror.join(name);
        std::fs::create_dir_all(target.parent().context("mirror parent")?)?;
        std::fs::write(&target, h.untwist(flags, &[input.as_path()])?)?;
        files.push(target);
    }
    Ok(Output {
        loader: fixture.entry.loader.clone(),
        entry: mirror.join(&fixture.entry.path),
        files,
        bundle: "none".to_string(),
    })
}

fn render(observed: &Option<Observed>) -> String {
    match observed {
        Some(o) => format!("exit={:?}\n{}", o.exit, o.stdout),
        None => format!("did not terminate within {}s", TIMEOUT.as_secs()),
    }
}

fn check_mode(
    h: &Harness,
    fixture: &Fixture,
    expected: &Option<Observed>,
    mode: (&'static str, &[&str]),
) -> Result<Outcome> {
    let dir = h.work.join(mode.0).join(&fixture.entry.path);
    std::fs::create_dir_all(&dir)?;
    let output = unpacked(h, fixture, mode.1, &dir)?;
    let outcome = |verdict| Outcome {
        fixture: fixture.entry.path.clone(),
        mode: mode.0,
        bundle: output.bundle.clone(),
        verdict,
    };
    let errors = h.syntax_errors(&output.files)?;
    if !errors.is_empty() {
        return Ok(outcome(Verdict::CheckFailed(errors.join("\n"))));
    }
    if h.mutation == Mutation::SwapPrints && !mutate(&output.files)? {
        return Ok(outcome(Verdict::Skipped("nothing to mutate".to_string())));
    }
    let actual = h.run_entry(&output.loader, &output.entry)?;
    if &actual == expected {
        return Ok(outcome(Verdict::Match));
    }
    Ok(outcome(Verdict::Mismatch {
        expected: render(expected),
        actual: render(&actual),
    }))
}

pub fn check_fixture(h: &Harness, fixture: &Fixture) -> Result<Vec<Outcome>> {
    if fixture.entry.loader == "none" {
        return Ok(MODES
            .iter()
            .map(|&(mode, _)| Outcome {
                fixture: fixture.entry.path.clone(),
                mode,
                bundle: "-".to_string(),
                verdict: Verdict::Skipped("no node loader for this fixture".to_string()),
            })
            .collect());
    }
    let entry = h.root.join(FIXTURES).join(&fixture.entry.path);
    let expected = h.run_entry(&fixture.entry.loader, &entry)?;
    if expected.as_ref().is_none_or(|o| o.stdout.is_empty()) {
        bail!("{} prints nothing under its own loader", fixture.entry.path);
    }
    let mut outcomes = MODES
        .iter()
        .map(|&mode| check_mode(h, fixture, &expected, mode))
        .collect::<Result<Vec<_>>>()?;
    mark_lost_split(&mut outcomes);
    Ok(outcomes)
}

pub fn mark_lost_split(outcomes: &mut [Outcome]) {
    if let [default, unpack_only] = outcomes {
        if default.bundle != unpack_only.bundle {
            default.verdict = Verdict::SplitLost(unpack_only.bundle.clone());
        }
    }
}

pub fn load(root: &Path, kind: Option<&str>) -> Result<Vec<Fixture>> {
    let manifest = root.join(FIXTURES).join("MANIFEST");
    let text =
        std::fs::read_to_string(&manifest).with_context(|| manifest.display().to_string())?;
    let all = fixtures(&parse_manifest(&text)?);
    Ok(all
        .into_iter()
        .filter(|f| kind.is_none_or(|k| f.kind == k))
        .collect())
}

#[derive(clap::Args)]
pub struct Request {
    #[arg(long)]
    pub kind: Option<String>,
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long)]
    pub seed_fault: bool,
    #[arg(long)]
    pub split_hoisted: bool,
}

fn print_outcome(outcome: &Outcome) -> (usize, usize, usize) {
    let head = format!(
        "unpack {:<11} {:<60} {:<16}",
        outcome.mode, outcome.fixture, outcome.bundle
    );
    match &outcome.verdict {
        Verdict::Match => {
            println!("{head} ok");
            (1, 0, 0)
        }
        Verdict::Skipped(reason) => {
            println!("{head} skipped: {reason}");
            (0, 1, 0)
        }
        Verdict::Mismatch { expected, actual } => {
            println!("{head} MISMATCH\n--- bundle\n{expected}\n--- unpacked\n{actual}");
            (0, 0, 1)
        }
        Verdict::CheckFailed(errors) => {
            println!("{head} CHECK-FAILED\n{errors}");
            (0, 0, 1)
        }
        Verdict::SplitLost(unpack_only) => {
            println!("{head} SPLIT-LOST\n--- unpack-only split\n{unpack_only}");
            (0, 0, 1)
        }
    }
}

fn harness(root: &Path, request: &Request, work: &Path) -> Result<Option<Harness>> {
    let Some(node) = untwist_testing::node::node() else {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        if untwist_testing::node::skip_allowed(required.as_deref()) {
            println!("{}", untwist_testing::node::SKIP_MESSAGE);
            return Ok(None);
        }
        bail!("required: {}", untwist_testing::node::SKIP_MESSAGE);
    };
    let untwist = request
        .untwist
        .clone()
        .unwrap_or_else(|| root.join("target/release/untwist"));
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli --features test-hooks");
    }
    let mutation = if request.seed_fault {
        Mutation::SwapPrints
    } else {
        Mutation::None
    };
    Ok(Some(Harness {
        node,
        untwist,
        root: root.to_path_buf(),
        work: work.to_path_buf(),
        mutation,
        split_hoisted: request.split_hoisted,
    }))
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let fixtures = load(root, request.kind.as_deref())?;
    if fixtures.is_empty() {
        bail!(
            "no fixture of kind {:?} in {FIXTURES}/MANIFEST",
            request.kind
        );
    }
    std::fs::create_dir_all(root.join("target"))?;
    let work = tempfile::Builder::new()
        .prefix("unpack-behaviour-")
        .tempdir_in(root.join("target"))?;
    let Some(h) = harness(root, request, work.path())? else {
        return Ok(());
    };
    let (mut ok, mut skipped, mut failed, mut runs) = (0, 0, 0, 0);
    for fixture in &fixtures {
        for outcome in check_fixture(&h, fixture)? {
            let (o, s, f) = print_outcome(&outcome);
            (ok, skipped, failed, runs) = (ok + o, skipped + s, failed + f, runs + 1);
        }
    }
    println!(
        "diff-unpack-behaviour: {} fixtures, {runs} runs, {ok} ok, {skipped} skipped, {failed} failed",
        fixtures.len()
    );
    if failed > 0 {
        bail!("{failed} run(s) differ from the bundle or fail node --check");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "# path\tbundler\tloader\tflags\tsha256\n\
        esbuild/a.js\tesbuild@1\tnode\t-\tx\n\
        parcel/hoist/index.js\tparcel@2\tnode\t-\tx\n\
        parcel/hoist/lazy.1.js\tparcel@2\tchunk\t-\tx\n\
        turbopack/server/index.js\tderived\tnode\t-\tx\n\
        turbopack/server/server/chunks/ssr/r.js\tnext@16\tchunk\t-\tx\n\
        turbopack/browser.js\tnext@16\tnone\t-\tx\n";

    #[test]
    fn a_directory_fixture_takes_every_file_under_its_entry() {
        let all = fixtures(&parse_manifest(MANIFEST).unwrap());
        let shape: Vec<(&str, &str, usize)> = all
            .iter()
            .map(|f| (f.kind.as_str(), f.entry.path.as_str(), f.files.len()))
            .collect();
        assert_eq!(
            shape,
            [
                ("esbuild", "esbuild/a.js", 1),
                ("parcel", "parcel/hoist/index.js", 2),
                ("turbopack", "turbopack/server/index.js", 2),
                ("turbopack", "turbopack/browser.js", 1),
            ]
        );
    }

    #[test]
    fn a_short_manifest_line_is_an_error() {
        assert!(parse_manifest("a.js\tb\tnode\n").is_err());
    }

    #[test]
    fn the_seeded_fault_swaps_two_different_prints() {
        let text = "console.log(\"a\");\nx();\nconsole.log(\"a\");\nconsole.log(\"b\");\n";
        assert_eq!(
            swap_first_prints(text).as_deref(),
            Some("console.log(\"b\");\nx();\nconsole.log(\"a\");\nconsole.log(\"a\");\n")
        );
        assert_eq!(swap_first_prints("console.log(1);\n"), None);
    }

    fn outcome(mode: &'static str, bundle: &str) -> Outcome {
        Outcome {
            fixture: "esbuild/a.js".to_string(),
            mode,
            bundle: bundle.to_string(),
            verdict: Verdict::Match,
        }
    }

    #[test]
    fn a_default_run_that_splits_less_than_unpack_only_fails() {
        let mut outcomes = [
            outcome("default", "none"),
            outcome("unpack-only", "esbuild:5"),
        ];
        mark_lost_split(&mut outcomes);
        assert!(matches!(&outcomes[0].verdict, Verdict::SplitLost(label) if label == "esbuild:5"));
        assert!(matches!(outcomes[1].verdict, Verdict::Match));
    }

    #[test]
    fn equal_splits_in_both_modes_pass() {
        let mut outcomes = [
            outcome("default", "esbuild:5"),
            outcome("unpack-only", "esbuild:5"),
        ];
        mark_lost_split(&mut outcomes);
        assert!(outcomes.iter().all(|o| matches!(o.verdict, Verdict::Match)));
    }
}
