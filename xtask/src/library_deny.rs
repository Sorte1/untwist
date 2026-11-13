use std::collections::BTreeSet;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use oxc_allocator::Allocator;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::table::{exact_key, Table};
use untwist_rename::library::{match_with, unit_records};

#[derive(clap::Args, Debug)]
pub struct Request {
    #[arg(long)]
    pub table: Vec<PathBuf>,
    #[arg(long)]
    pub pairs: PathBuf,
}

fn load(paths: &[PathBuf]) -> Result<Table> {
    let mut tables = Vec::new();
    for path in paths {
        let text = crate::library_gz::read(path)?;
        tables.push(Table::parse(&text).with_context(|| format!("{} is not a library table", path.display()))?);
    }
    Ok(Table::merge(tables))
}

pub fn run(request: &Request) -> Result<()> {
    let table = load(&request.table)?;
    let pairs = std::fs::read_to_string(&request.pairs)?;
    let mut out = std::io::stdout().lock();
    for line in pairs.lines().filter(|line| !line.is_empty()) {
        let (file, truth) = line.split_once('\t').unwrap_or((line, ""));
        let truth: BTreeSet<&str> = truth.split(',').filter(|name| !name.is_empty()).collect();
        let Ok(text) = std::fs::read_to_string(file) else { continue };
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        if has_error(&parsed.diagnostics) {
            continue;
        }
        let sem = build_scoping(&parsed.program);
        let found = match_with(&table, &parsed.program, &sem.scoping);
        let wrong: Vec<_> = found.hits.iter().filter(|hit| !truth.contains(hit.label.as_str())).collect();
        if wrong.is_empty() {
            continue;
        }
        let records = unit_records(&parsed.program, &sem.scoping);
        for hit in wrong {
            for record in records.iter().filter(|record| record.span == hit.span) {
                writeln!(out, "{:016x}\t{}\t{}", exact_key(record.exact), hit.label, file)?;
            }
        }
    }
    Ok(())
}
