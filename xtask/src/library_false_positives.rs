use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::{bail, Result};
use oxc_allocator::Allocator;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::match_program;

#[derive(clap::Args, Debug)]
pub struct Request {
    #[arg(long)]
    pub untwist: Option<PathBuf>,
    #[arg(long, default_value_t = 8)]
    pub jobs: usize,
    #[arg(long)]
    pub samples: Vec<PathBuf>,
    #[arg(long)]
    pub skip_fixtures: bool,
}

const SKIPPED_FOLDERS: [&str; 2] = ["libraries", "generated"];
const LANES: [(&str, &[&str]); 2] = [("default", &[]), ("unsafe", &["--unsafe"])];
const SNIPPET: usize = 90;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if !SKIPPED_FOLDERS.contains(&name) {
                collect(&path, out);
            }
        } else if path
            .extension()
            .is_some_and(|ext| ext == "js" || ext == "mjs" || ext == "cjs")
        {
            out.push(path);
        }
    }
}

fn line_of(text: &str, offset: u32) -> (usize, String) {
    let offset = (offset as usize).min(text.len());
    let line = text.as_bytes()[..offset]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1;
    let snippet: String = text[offset..]
        .chars()
        .take_while(|&ch| ch != '\n')
        .take(SNIPPET)
        .collect();
    (line, snippet)
}

fn scan(text: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, text);
    if has_error(&parsed.diagnostics) {
        return Vec::new();
    }
    let sem = build_scoping(&parsed.program);
    let found = match_program(&parsed.program, &sem.scoping);
    found
        .hits
        .iter()
        .filter(|hit| hit.outermost && hit.reported)
        .map(|hit| {
            let (line, snippet) = line_of(text, hit.span.start);
            format!(
                "{} {}: {} at line {line}: {snippet}",
                hit.label,
                hit.range(),
                hit.module_text()
            )
        })
        .collect()
}

fn process(untwist: &Path, file: &Path) -> Vec<String> {
    let mut report = Vec::new();
    for (lane, flags) in LANES {
        let Ok(output) = Command::new(untwist)
            .args(flags)
            .args(["--no-unpack", "--timeout", "300"])
            .arg(file)
            .stderr(std::process::Stdio::null())
            .output()
        else {
            continue;
        };
        let text = String::from_utf8_lossy(&output.stdout);
        for hit in scan(&text) {
            report.push(format!("{} [{lane}] {hit}", file.display()));
        }
    }
    report
}

pub fn run(root: &Path, request: &Request) -> Result<()> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let untwist = request
        .untwist
        .clone()
        .unwrap_or_else(|| target.join("release/untwist"));
    if !untwist.exists() {
        bail!("build first: cargo build --release -p untwist-cli");
    }
    let mut files = Vec::new();
    if !request.skip_fixtures {
        collect(&root.join("fixtures"), &mut files);
    }
    files.extend(request.samples.iter().cloned());
    let next = AtomicUsize::new(0);
    let found: Mutex<Vec<String>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..request.jobs.max(1) {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(file) = files.get(index) else {
                    break;
                };
                let hits = process(&untwist, file);
                found.lock().expect("found lock").extend(hits);
            });
        }
    });
    let mut found = found.into_inner().expect("found lock");
    found.sort();
    for line in &found {
        println!("{line}");
    }
    eprintln!("{} files, {} library matches", files.len(), found.len());
    Ok(())
}
