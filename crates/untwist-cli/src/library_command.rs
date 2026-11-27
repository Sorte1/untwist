use std::ffi::OsString;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;

use crate::library_cache;

pub const COMMAND: &str = "libraries";

#[derive(Parser, Debug)]
#[command(
    name = "untwist libraries",
    about = "manage the cached library collection that --libraries cache (the default) reads; \
             only `update` uses the network"
)]
struct Libraries {
    #[command(subcommand)]
    action: Action,
}

#[derive(clap::Subcommand, Debug)]
enum Action {
    #[command(
        about = "fetch collection.json and every shard it names that the cache lacks, \
                       check each against its SHA-256, and remove the files no longer named"
    )]
    Update {
        #[arg(long, value_name = "URL", default_value = library_cache::DEFAULT_URL)]
        url: String,
    },
    #[command(about = "print the cached collection's version, shape epoch, source and size")]
    Status,
    #[command(about = "remove the cached collection and its shards")]
    Clear,
}

fn run(action: &Action) -> Result<()> {
    let dir = library_cache::cache_dir()
        .context("no cache directory: set XDG_CACHE_HOME, HOME or UNTWIST_LIBRARY_CACHE")?;
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    match action {
        Action::Update { url } => library_cache::update(&dir, url, &mut out, &mut err),
        Action::Status => library_cache::status(&dir, &mut out),
        Action::Clear => library_cache::clear(&dir, &mut out),
    }
}

pub fn dispatch() -> Option<ExitCode> {
    let mut args = std::env::args_os().skip(1);
    if args.next()? != COMMAND {
        return None;
    }
    let name = OsString::from(format!("untwist {COMMAND}"));
    let parsed = Libraries::parse_from(std::iter::once(name).chain(args));
    Some(match run(&parsed.action) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    })
}
