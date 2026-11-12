use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::semantic::build_scoping;
use untwist_rename::human::looks_human_written;

const SHORT: usize = 2;

const GENERIC: [&str; 12] = [
    "v", "fn", "cls", "arr", "obj", "str", "num", "bool", "re", "result", "cb", "el",
];

#[derive(clap::Args)]
pub struct Request {
    pub files: Vec<PathBuf>,
}

#[derive(Default)]
pub struct Share {
    pub total: usize,
    pub machine: usize,
    pub generic: usize,
}

pub fn is_machine_named(name: &str) -> bool {
    !looks_human_written(name) || name.chars().count() <= SHORT
}

pub fn is_generic(name: &str) -> bool {
    let stem = name.trim_end_matches(|ch: char| ch.is_ascii_digit());
    GENERIC.contains(&stem)
}

pub fn share(source: &str) -> Share {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let mut out = Share::default();
    for name in sem.scoping.symbol_names() {
        out.total += 1;
        if is_machine_named(name) {
            out.machine += 1;
        } else if is_generic(name) {
            out.generic += 1;
        }
    }
    out
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 * 100.0 / total as f64
}

fn report(file: &Path) -> Result<String> {
    let source = std::fs::read_to_string(file).with_context(|| file.display().to_string())?;
    let found = share(&source);
    Ok(format!(
        "{}\tbindings {}\tmachine {} ({:.1}%)\tgeneric {} ({:.1}%)",
        file.display(),
        found.total,
        found.machine,
        percent(found.machine, found.total),
        found.generic,
        percent(found.generic, found.total),
    ))
}

pub fn run(request: &Request) -> Result<()> {
    for file in &request.files {
        println!("{}", report(file)?);
    }
    Ok(())
}
