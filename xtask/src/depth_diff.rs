use std::time::Instant;

use anyhow::{bail, Result};
use untwist_testing::depth_diff::{
    fixture_paths, run_class, run_depth_classes, run_fixtures, run_soup_stream, Class, Tally,
    EDGE_SEED, MIX_SEED, SOUP_SEED,
};

pub const DEFAULT_MIX: u64 = 340_000;
pub const DEFAULT_EDGES: u64 = 1_000_000;
pub const DEFAULT_SOUP: usize = 1_000_000;

#[derive(clap::Args)]
pub struct Request {
    #[arg(long, default_value_t = DEFAULT_MIX)]
    pub mix: u64,
    #[arg(long, default_value_t = DEFAULT_EDGES)]
    pub edges: u64,
    #[arg(long, default_value_t = DEFAULT_SOUP)]
    pub soup: usize,
    #[arg(long, default_value_t = 8)]
    pub threads: usize,
    #[arg(long)]
    pub only: Option<String>,
}

fn report(name: &str, tally: &Tally, started: Instant, extra: &str) {
    println!(
        "depth-diff {name}: {}{extra} in {:.1?}",
        tally.summary(),
        started.elapsed()
    );
    for example in &tally.examples {
        println!("{example}");
    }
}

fn wanted(request: &Request, name: &str) -> bool {
    request.only.as_deref().is_none_or(|only| only == name)
}

fn generators(request: &Request, total: &mut Tally) {
    let classes = [
        ("multi", Class::Multi, MIX_SEED, request.mix),
        ("grammar", Class::Grammar, MIX_SEED, request.mix),
        ("mutate", Class::Mutate, MIX_SEED, request.mix),
        ("edges", Class::Edges, EDGE_SEED, request.edges),
    ];
    for (name, class, seed, count) in classes {
        if !wanted(request, name) {
            continue;
        }
        let started = Instant::now();
        let tally = run_class(class, seed, (0, count), request.threads);
        report(name, &tally, started, "");
        total.merge(tally);
    }
}

pub fn run(request: &Request) -> Result<()> {
    if cfg!(debug_assertions) {
        bail!("run the differential in release: `cargo run --release -p xtask -- depth-diff`");
    }
    let mut total = Tally::default();
    if wanted(request, "soup") {
        let started = Instant::now();
        let tally = run_soup_stream(SOUP_SEED, request.soup);
        report("soup", &tally, started, "");
        total.merge(tally);
    }
    generators(request, &mut total);
    if wanted(request, "classes") {
        let started = Instant::now();
        let tally = run_depth_classes();
        report("classes", &tally, started, "");
        total.merge(tally);
    }
    if wanted(request, "fixtures") {
        let started = Instant::now();
        let paths = fixture_paths();
        let (tally, unreadable) = run_fixtures(&paths);
        let extra = format!(" files={} unreadable={unreadable}", paths.len());
        report("fixtures", &tally, started, &extra);
        total.merge(tally);
    }
    println!("depth-diff SUMMARY {}", total.summary());
    if total.mismatches > 0 {
        bail!("{} differential mismatches", total.mismatches);
    }
    Ok(())
}
