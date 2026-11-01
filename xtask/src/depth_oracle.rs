use std::process::Command;

use anyhow::{bail, Context, Result};
use untwist_testing::depth_grid::grid_len;
use untwist_testing::depth_oracle::{child, drive, ChildRun, Source, STACK_TUNABLE};

pub const CHILD_COMMAND: &str = "depth-oracle-child";
pub const DEFAULT_CHUNK: u64 = 20_000;

#[derive(clap::Args)]
pub struct Request {
    #[arg(long, default_value = "grid")]
    pub class: String,
    #[arg(long, default_value_t = 1)]
    pub seed: u64,
    #[arg(long, default_value_t = 0)]
    pub start: u64,
    #[arg(long)]
    pub end: Option<u64>,
    #[arg(long, default_value_t = DEFAULT_CHUNK)]
    pub chunk: u64,
}

#[derive(clap::Args)]
pub struct ChildRequest {
    pub class: String,
    pub seed: u64,
    pub start: u64,
    pub end: u64,
}

fn source_of(name: &str) -> Result<Source> {
    match Source::from_name(name) {
        Some(source) => Ok(source),
        None => bail!("unknown class {name}: grid, gridrand, soup, soup2, rand, sweep, sample, multi, grammar, mutate, mix or names"),
    }
}

fn spawn(name: &str, seed: u64, start: u64, end: u64) -> Result<ChildRun> {
    let exe = std::env::current_exe().context("the xtask binary")?;
    let output = Command::new(exe)
        .args([CHILD_COMMAND, name])
        .args([seed, start, end].map(|value| value.to_string()))
        .env(STACK_TUNABLE.0, STACK_TUNABLE.1)
        .output()
        .context("spawn the oracle child")?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let first = stderr.lines().next().unwrap_or("");
    Ok(ChildRun {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        status: format!("{:?} {first}", output.status),
    })
}

pub fn run(request: &Request) -> Result<()> {
    if cfg!(debug_assertions) {
        bail!("the stack oracle measures release frames; run it with `cargo run --release -p xtask -- depth-oracle`");
    }
    let source = source_of(&request.class)?;
    let end = request.end.unwrap_or(match source {
        Source::Class(untwist_testing::depth_grid::Kind::Grid) => grid_len(),
        _ => 200_000,
    });
    let mut failed = None;
    let tally = drive(
        source,
        request.seed,
        (request.start, end),
        request.chunk,
        |from, to| {
            println!("depth-oracle: {} {from}..{to}", request.class);
            spawn(&request.class, request.seed, from, to).unwrap_or_else(|error| {
                failed = Some(error.to_string());
                ChildRun {
                    stdout: "E".to_owned(),
                    status: String::new(),
                }
            })
        },
    );
    if let Some(error) = failed {
        bail!("{error}");
    }
    for failure in &tally.failures {
        println!("{failure}");
    }
    println!(
        "SUMMARY class={} seed={} range={}..{end} {}",
        request.class,
        request.seed,
        request.start,
        tally.summary()
    );
    if !tally.clean() {
        bail!("the stack oracle found {} failures", tally.failures.len());
    }
    Ok(())
}

pub fn run_child(request: &ChildRequest) -> Result<()> {
    let source = source_of(&request.class)?;
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    child(source, request.seed, request.start, request.end, &mut out);
    Ok(())
}
