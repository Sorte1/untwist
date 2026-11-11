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
        help = "with --output, overwrite the files this run writes and delete the files a previous run left there"
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
        help = "use the unsafe lane: assume built-ins are not patched, and enable the three \
                passes tagged unsafe: the transpile stage's template-literals, and the JSX \
                stage's jsx and jsx-new, which reconstruct React.createElement and jsx/jsxs calls"
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
                gives up and leaves the call sites alone (default 50000000 plus 500000 per distinct \
                decoder call, at most 1000000000; 1000000000 with --unsafe; unlimited lifts it)"
    )]
    pub fuel: Option<u64>,
    #[arg(
        long = "max-memory",
        value_name = "SIZE",
        default_value = crate::budget::DEFAULT_MAX_MEMORY,
        value_parser = untwist::budget_memory_size,
        help = "memory the inputs being processed may use (such as 512MiB or 8G, or unlimited); \
                the texts the run holds are not counted. At three quarters of it an input stops \
                its passes and keeps the output so far (note budget-memory); at the full size that \
                input is stopped and passed through unchanged"
    )]
    pub max_memory: u64,
    #[arg(
        long,
        value_name = "SECS",
        value_parser = untwist::budget_timeout_secs,
        help = "wall-clock seconds processing the inputs may take (default: no limit); writing \
                the output and --fetch-wasm are not limited by it. At the limit every input stops \
                its passes and keeps the output so far (note budget-time); an input still running \
                a while later is passed through unchanged"
    )]
    pub timeout: Option<std::time::Duration>,
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
        long = "wasm-check",
        value_enum,
        value_name = "MODE",
        default_value = "off",
        help = "scan the output for embedded wasm modules: off (the default) only says when it \
                may hold one, auto extracts a byte array or base64 text that holds a module \
                with imports or exports, on reparses every output to look (slow on large files); \
                --wasm, --wasm-dir and --fetch-wasm work in every mode"
    )]
    pub wasm_check: WasmCheckArg,
    #[arg(
        long = "wasm-dir",
        value_name = "DIR",
        help = "take wasm modules the input loads from elsewhere out of DIR, matched by file name \
                (hashed names such as 3f2a91.module.wasm included); their imports and exports \
                name the glue when they match it"
    )]
    pub wasm_dir: Option<PathBuf>,
    #[arg(
        long = "wasm",
        value_name = "NAME=PATH",
        help = "use the wasm file at PATH for the module the report lists as NAME, over any \
                --wasm-dir match (repeatable)"
    )]
    pub wasm: Vec<String>,
    #[arg(
        long = "fetch-wasm",
        help = "download wasm modules the input loads from an absolute URL and that no --wasm or \
                --wasm-dir file supplies; without it nothing is ever fetched"
    )]
    pub fetch_wasm: bool,
    #[arg(
        long = "fetch-wasm-local",
        requires = "fetch_wasm",
        help = "with --fetch-wasm, also fetch from loopback, link-local and private addresses, \
                which are refused otherwise; for input that loads wasm from your own network"
    )]
    pub fetch_wasm_local: bool,
    #[arg(
        long = "wasm-base",
        value_name = "URL",
        requires = "fetch_wasm",
        help = "with --fetch-wasm, resolve relative wasm URLs against URL (the page or script \
                the input came from)"
    )]
    pub wasm_base: Option<String>,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum WasmCheckArg {
    Off,
    On,
    Auto,
}

impl WasmCheckArg {
    fn check(self) -> untwist::WasmCheck {
        match self {
            WasmCheckArg::Off => untwist::WasmCheck::Off,
            WasmCheckArg::On => untwist::WasmCheck::On,
            WasmCheckArg::Auto => untwist::WasmCheck::Auto,
        }
    }
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

    pub fn limits(&self) -> crate::budget::Limits {
        crate::budget::Limits {
            max_memory: (self.max_memory != u64::MAX).then_some(self.max_memory),
            timeout: self.timeout,
        }
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
            max_memory: self.limits().soft_memory(),
            timeout: self.timeout,
            disabled_passes: self.disable.clone(),
            wasm_check: self.wasm_check.check(),
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

    fn unsafe_help() -> String {
        let command = <Args as clap::CommandFactory>::command();
        let unsafe_flag = command
            .get_arguments()
            .find(|arg| arg.get_long() == Some("unsafe"))
            .expect("the --unsafe flag");
        unsafe_flag.get_help().expect("help text").to_string()
    }

    fn names_pass(help: &str, name: &str) -> bool {
        help.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .any(|word| word == name)
    }

    #[test]
    fn the_unsafe_help_names_exactly_the_passes_tagged_unsafe() {
        let help = unsafe_help();
        for pass in untwist::PASSES {
            assert_eq!(
                names_pass(&help, pass.name),
                pass.unsafe_only,
                "{}: {help}",
                pass.name
            );
        }
        let tagged = untwist::PASSES
            .iter()
            .filter(|pass| pass.unsafe_only)
            .count();
        assert_eq!(tagged, 3, "{help}");
        assert!(help.contains("three passes tagged unsafe"), "{help}");
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
