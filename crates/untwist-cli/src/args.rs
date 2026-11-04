use std::num::NonZeroUsize;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "untwist",
    version,
    about = "Deobfuscate, unminify and unpack bundled JavaScript"
)]
pub struct Args {
    #[arg(help = "JavaScript files to process; with none given, the program is read from stdin")]
    pub files: Vec<PathBuf>,
    #[arg(
        short,
        long,
        value_name = "DIR",
        help = "write the result into DIR, one file per unpacked module, instead of stdout"
    )]
    pub output: Option<PathBuf>,
    #[arg(
        short,
        long,
        help = "with --output, overwrite an existing directory and delete the files a previous run left there"
    )]
    pub force: bool,
    #[arg(
        long = "no-jsx",
        action = clap::ArgAction::SetFalse,
        help = "skip JSX reconstruction (the JSX passes are unsafe and run only with --unsafe)"
    )]
    pub jsx: bool,
    #[arg(
        long = "no-unpack",
        action = clap::ArgAction::SetFalse,
        help = "skip bundle unpacking and treat the input as a single module"
    )]
    pub unpack: bool,
    #[arg(
        long = "split-hoisted",
        help = "also split a scope-hoisted bundle (Rollup, Vite) into modules by a heuristic \
                over its top-level declarations; off by default, since such a bundle carries no \
                module boundaries of its own"
    )]
    pub split_hoisted: bool,
    #[arg(
        long = "no-deobfuscate",
        action = clap::ArgAction::SetFalse,
        help = "skip the deobfuscate stage: string arrays, decoders, control flow and protections"
    )]
    pub deobfuscate: bool,
    #[arg(
        long = "no-unminify",
        action = clap::ArgAction::SetFalse,
        help = "skip the unminify stage that restores readable statements and expressions"
    )]
    pub unminify: bool,
    #[arg(
        long = "unsafe",
        help = "enable the four passes tagged unsafe: deobfuscate's dead-code, which folds an if \
                whose test is a constant comparison and deletes the branch it proves unreachable; \
                unminify's template-literals; and the JSX stage's jsx and jsx-new, which \
                reconstruct React.createElement and jsx/jsxs calls"
    )]
    pub unsafe_transforms: bool,
    #[arg(
        long,
        value_enum,
        value_name = "MODE",
        help = "rename identifiers: none, mangle, simple or unique"
    )]
    pub rename: Option<RenameArg>,
    #[arg(
        short = 'm',
        long,
        conflicts_with = "rename",
        help = "alias for --rename mangle"
    )]
    pub mangle: bool,
    #[arg(
        long = "keep-human-names",
        action = clap::ArgAction::SetFalse,
        help = "in simple and unique, leave identifiers that already look human-written alone \
                (by default they are renamed too; no effect in mangle)"
    )]
    pub rename_all: bool,
    #[cfg(feature = "test-hooks")]
    #[arg(long = "conservative", hide = true)]
    pub conservative: bool,
    #[arg(
        long,
        help = "stop a stage after N passes over the program (default: run until nothing changes)"
    )]
    pub max_iterations: Option<u32>,
    #[arg(
        long,
        value_enum,
        value_name = "FORMAT",
        help = "print what every stage and pass did: text on stderr, or json on stdout beside the code"
    )]
    pub report: Option<ReportFormat>,
    #[arg(short, long, help = "shorthand for --report text")]
    pub verbose: bool,
    #[arg(
        long = "single-file",
        help = "write every unpacked module into one deobfuscated.js (or to stdout) for reading, \
                with a banner per module, instead of one file per module; bundle.json still \
                records the real paths. Without a bundle, several inputs are joined under \
                `// ==== input N ====` banners and a single input is written bare"
    )]
    pub single_file: bool,
    #[arg(
        long = "max-input-mb",
        value_name = "MB",
        help = "warn when the inputs together exceed MB megabytes, instead of comparing against \
                available system memory"
    )]
    pub max_input_mb: Option<u64>,
    #[arg(
        short = 'j',
        long,
        value_name = "N",
        help = "run up to N inputs at once, each in its own arena on its own thread (default: the \
                machine's parallelism, capped at the number of inputs; a single input never starts \
                a pool); outputs are merged in input order whatever N is"
    )]
    pub jobs: Option<NonZeroUsize>,
    #[arg(
        long,
        value_name = "N",
        value_parser = fuel_steps,
        help = "interpreter steps the string-array decoder may spend per decode unit before it \
                gives up and leaves the call sites alone (default 50000000, or 1000000000 with --unsafe; \
                unlimited lifts it)"
    )]
    pub fuel: Option<u64>,
    #[arg(
        long = "disable",
        value_name = "PASS",
        value_delimiter = ',',
        value_parser = pass_name,
        help = "turn off the named passes (repeat the flag or separate names with commas); \
                --list-passes prints every name"
    )]
    pub disable: Vec<&'static str>,
    #[arg(
        long = "list-passes",
        help = "print every pass the engine can run, with its stage, and exit"
    )]
    pub list_passes: bool,
}

fn pass_name(word: &str) -> Result<&'static str, String> {
    untwist::pass_named(word).ok_or_else(|| format!("unknown pass {word} (see --list-passes)"))
}

pub fn pass_list() -> String {
    let width = untwist::PASSES
        .iter()
        .map(|pass| pass.name.len())
        .max()
        .unwrap_or(0);
    let mut text = format!("{:width$}  STAGE\n", "PASS");
    for pass in untwist::PASSES {
        let tag = if pass.unsafe_only { "  (unsafe)" } else { "" };
        text.push_str(&format!("{:width$}  {}{tag}\n", pass.name, pass.stage));
    }
    text
}

fn fuel_steps(word: &str) -> Result<u64, String> {
    if word == "unlimited" {
        return Ok(u64::MAX);
    }
    match word.parse::<u64>() {
        Ok(0) => Err("must be at least 1".to_string()),
        Ok(steps) => Ok(steps),
        Err(_) => Err(format!("takes a number or unlimited, not {word}")),
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ReportFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum RenameArg {
    None,
    Mangle,
    Simple,
    Unique,
}

impl RenameArg {
    fn mode(self) -> untwist::RenameMode {
        match self {
            RenameArg::None => untwist::RenameMode::None,
            RenameArg::Mangle => untwist::RenameMode::Mangle,
            RenameArg::Simple => untwist::RenameMode::Simple,
            RenameArg::Unique => untwist::RenameMode::Unique,
        }
    }
}

impl Args {
    fn rename_mode(&self) -> untwist::RenameMode {
        if self.mangle {
            return untwist::RenameMode::Mangle;
        }
        self.rename.unwrap_or(RenameArg::None).mode()
    }

    pub fn options(&self) -> untwist::Options {
        untwist::Options {
            unminify: self.unminify,
            deobfuscate: self.deobfuscate,
            unpack: self.unpack,
            jsx: self.jsx,
            unsafe_transforms: self.unsafe_transforms,
            #[cfg(feature = "test-hooks")]
            extended: !self.conservative,
            max_iterations: self.max_iterations,
            mappings: Vec::new(),
            split_hoisted: self.split_hoisted,
            rename: self.rename_mode(),
            rename_all: self.rename_all,
            plugins: untwist::Plugins::default(),
            single_file: self.single_file,
            jobs: self.jobs,
            fuel: self.fuel,
            disabled_passes: self.disable.clone(),
            ..untwist::Options::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::Parser;

    #[cfg(feature = "test-hooks")]
    #[test]
    fn the_superset_passes_are_on_by_default() {
        assert!(untwist::Options::default().extended);
        let args = Args::parse_from(["untwist", "in.js"]);
        assert!(args.options().extended);
    }

    #[cfg(feature = "test-hooks")]
    #[test]
    fn the_conservative_flag_turns_the_superset_passes_off() {
        let args = Args::parse_from(["untwist", "--conservative", "in.js"]);
        assert!(!args.options().extended);
    }

    #[test]
    fn scope_hoist_splitting_is_off_unless_asked_for() {
        assert!(
            !Args::parse_from(["untwist", "in.js"])
                .options()
                .split_hoisted
        );
        let args = Args::parse_from(["untwist", "--split-hoisted", "in.js"]);
        assert!(args.options().split_hoisted);
    }
}
