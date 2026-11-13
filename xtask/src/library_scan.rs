use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::Result;
use oxc_allocator::Allocator;
use untwist_core::parse::{has_error, parse_program};
use untwist_core::semantic::build_scoping;
use untwist_rename::library::table::{current, Table};
use untwist_rename::library::{match_with, Hit};

#[derive(clap::Args, Debug)]
pub struct Request {
    pub files: Vec<PathBuf>,
    #[arg(long)]
    pub table: Vec<PathBuf>,
    #[arg(long)]
    pub names: bool,
    #[arg(long)]
    pub hits: bool,
    #[arg(long)]
    pub line: Option<usize>,
}

fn load(request: &Request) -> Result<Option<Table>> {
    if request.table.is_empty() {
        return Ok(None);
    }
    let mut tables = Vec::new();
    for path in &request.table {
        let text = crate::library_gz::read(path)?;
        tables.push(
            Table::parse(&text)
                .ok_or_else(|| anyhow::anyhow!("{} is not a library table", path.display()))?,
        );
    }
    Ok(Some(Table::merge(tables)))
}

fn line_of(text: &str, offset: u32) -> usize {
    text.as_bytes()[..offset as usize]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

fn describe(hit: &Hit) -> String {
    format!("{} {}: {}", hit.label, hit.range(), hit.module_text())
}

pub fn run(request: &Request) -> Result<()> {
    let custom = load(request)?;
    let builtin = current();
    let table = custom.as_ref().unwrap_or(&builtin);
    for file in &request.files {
        let text = std::fs::read_to_string(file)?;
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, &text);
        if has_error(&parsed.diagnostics) {
            println!("{}: parse error", file.display());
            continue;
        }
        let sem = build_scoping(&parsed.program);
        if let Some(line) = request.line {
            for record in untwist_rename::library::unit_records(&parsed.program, &sem.scoping) {
                if line_of(&text, record.span.start) == line {
                    println!(
                        "  unit {}:{:016x} {:032x} binding={:?} inner={:?} outer={:?} specific={}",
                        record.loose.0,
                        record.loose.1,
                        record.exact,
                        record.binding,
                        record.inner,
                        record.outer,
                        record.specific
                    );
                }
            }
        }
        let started = std::time::Instant::now();
        let found = match_with(table, &parsed.program, &sem.scoping);
        let elapsed = started.elapsed();
        let mut modules: BTreeMap<String, usize> = BTreeMap::new();
        for hit in found.hits.iter().filter(|hit| hit.outermost) {
            *modules.entry(describe(hit)).or_default() += 1;
        }
        println!(
            "{}: {} hits, {} outermost modules, {} names, {:.0} ms",
            file.display(),
            found.hits.len(),
            modules.len(),
            found.names.len(),
            elapsed.as_secs_f64() * 1000.0
        );
        if request.hits {
            for hit in &found.hits {
                println!(
                    "  line {} {:?} {:?} outermost={} {}",
                    line_of(&text, hit.span.start),
                    hit.role,
                    hit.kind,
                    hit.outermost,
                    describe(hit)
                );
            }
        }
        if request.names {
            let mut names: Vec<(String, String, usize)> = found
                .names
                .iter()
                .map(|(symbol, name)| {
                    let span = sem.scoping.symbol_span(*symbol);
                    (
                        sem.scoping.symbol_name(*symbol).to_string(),
                        name.clone(),
                        line_of(&text, span.start),
                    )
                })
                .collect();
            names.sort_by_key(|(_, _, line)| *line);
            for (old, new, line) in names {
                println!("  {line}: {old} -> {new}");
            }
            for (symbol, offered) in &found.conflicts {
                let line = line_of(&text, sem.scoping.symbol_span(*symbol).start);
                println!(
                    "  conflict {line}: {} offered {offered:?}",
                    sem.scoping.symbol_name(*symbol)
                );
            }
        }
    }
    Ok(())
}
