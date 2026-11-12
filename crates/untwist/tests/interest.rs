use oxc_allocator::Allocator;
use std::path::{Path, PathBuf};
use untwist_core::ast_util::respan::reset_synthetic_spans;
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, Interest, Note, PassCtor, PassGroup, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_rename::{rename_passes, RenameConfig, RenameMode};
use untwist_testing::js_files_in;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn every_production_pass<'a>() -> Vec<(String, Box<dyn untwist_core::pass::Pass<'a> + 'a>)> {
    let mut passes: Vec<_> = untwist::all_pass_ctors()
        .iter()
        .map(|ctor| {
            let pass = ctor();
            (pass.name().to_string(), pass)
        })
        .collect();
    for mode in [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique] {
        let config = RenameConfig {
            mode,
            ..RenameConfig::default()
        };
        for pass in rename_passes(config) {
            passes.push((pass.name().to_string(), pass));
        }
    }
    let shorthand = untwist_rename::shorthand::boxed();
    passes.push((shorthand.name().to_string(), shorthand));
    passes
}

#[test]
fn every_registered_pass_declares_a_non_exhaustive_interest() {
    let passes = every_production_pass();
    assert!(passes.len() >= 56, "{}", passes.len());
    for (name, pass) in &passes {
        let interest = pass.interest();
        assert_ne!(
            interest,
            Interest::ALL,
            "{name} keeps the exhaustive default"
        );
        assert_ne!(interest, Interest::NONE, "{name} declares no hook at all");
    }
}

const CORPUS_DIRS: [&str; 8] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/deobfuscate-gaps",
    "fixtures/unpack-gaps",
    "fixtures/corpus-npm",
    "fixtures/eval",
];

fn corpus() -> Vec<(String, String)> {
    CORPUS_DIRS
        .iter()
        .flat_map(|dir| js_files_in(dir))
        .collect()
}

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

fn is_slow_in_debug(label: &str) -> bool {
    SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name))
}

fn run_group<'a>(
    source: &'a str,
    group: &mut PassGroup<'a>,
    alloc: &'a Allocator,
) -> (String, Vec<(&'static str, u32)>, u32, Vec<Note>) {
    reset_synthetic_spans();
    let mut parsed = parse_program(alloc, source);
    let sem = build_scoping(&parsed.program);
    let (_sem, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, group);
    (
        print(&parsed.program, PrintStyle::Compact),
        report.per_pass,
        report.iterations,
        report.notes,
    )
}

fn opts() -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: Some(4),
        fuel: None,
        ..PassOptions::default()
    }
}

fn assert_interest_matches_exhaustive(label: &str, source: String, ctors: &[PassCtor<'static>]) {
    let source: &'static str = Box::leak(source.into_boxed_str());
    let alloc: &'static Allocator = Box::leak(Box::new(Allocator::default()));
    let mut interest = PassGroup::from_ctors(ctors, opts());
    let mut exhaustive =
        PassGroup::exhaustive_for_tests(ctors.iter().map(|ctor| ctor()).collect(), opts());
    let by_interest = run_group(source, &mut interest, alloc);
    let by_exhaustive = run_group(source, &mut exhaustive, alloc);
    assert_eq!(
        by_interest, by_exhaustive,
        "{label}: the interest group and the exhaustive group diverge"
    );
}

fn every_ctor<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = untwist::all_pass_ctors();
    ctors.push(untwist_rename::shorthand::boxed);
    ctors
}

#[test]
fn the_interest_group_matches_the_exhaustive_group_on_every_corpus_file() {
    let ctors = every_ctor();
    let files: Vec<(String, String)> = corpus()
        .into_iter()
        .filter(|(label, _)| !is_slow_in_debug(label))
        .collect();
    assert!(files.len() >= 90, "{}", files.len());
    for (label, source) in files {
        assert_interest_matches_exhaustive(&label, source, &ctors);
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the two rotation-search fixtures; run with cargo test --release -p untwist --test interest"
)]
fn the_interest_group_matches_the_exhaustive_group_on_the_slow_fixtures() {
    let ctors = every_ctor();
    for (label, source) in corpus()
        .into_iter()
        .filter(|(label, _)| is_slow_in_debug(label))
    {
        assert_interest_matches_exhaustive(&label, source, &ctors);
    }
}

fn rename_ctor_for(mode: RenameMode) -> Vec<Box<dyn untwist_core::pass::Pass<'static> + 'static>> {
    rename_passes(RenameConfig {
        mode,
        ..RenameConfig::default()
    })
}

#[test]
fn the_rename_passes_match_exhaustive_on_the_unminify_corpus() {
    for (label, source) in js_files_in("fixtures/unminify") {
        for mode in [RenameMode::Mangle, RenameMode::Simple, RenameMode::Unique] {
            let source: &'static str = Box::leak(source.clone().into_boxed_str());
            let alloc: &'static Allocator = Box::leak(Box::new(Allocator::default()));
            let mut interest = PassGroup::new(rename_ctor_for(mode), PassOptions::default());
            let mut exhaustive =
                PassGroup::exhaustive_for_tests(rename_ctor_for(mode), PassOptions::default());
            assert_eq!(
                run_group(source, &mut interest, alloc),
                run_group(source, &mut exhaustive, alloc),
                "{label} under {mode:?}"
            );
        }
    }
}

fn hook_names_in_impl_block(block: &str) -> Vec<String> {
    block
        .lines()
        .filter_map(|line| line.strip_prefix("    fn "))
        .filter(|rest| rest.starts_with("enter_") || rest.starts_with("exit_"))
        .filter_map(|rest| rest.split('(').next())
        .map(|hook| hook.to_ascii_uppercase())
        .collect()
}

fn interest_names_in_impl_block(block: &str) -> Vec<String> {
    let Some(start) = block.find("fn interest(&self) -> Interest {") else {
        return Vec::new();
    };
    let body = &block[start..];
    let end = body.find("\n    }\n").unwrap_or(body.len());
    body[..end]
        .split("Interest::")
        .skip(1)
        .filter_map(|rest| {
            rest.split(|ch: char| !ch.is_ascii_uppercase() && ch != '_')
                .next()
        })
        .map(str::to_string)
        .collect()
}

fn pass_impl_blocks(source: &str) -> Vec<&str> {
    source
        .match_indices("impl<'a> Pass<'a> for ")
        .map(|(start, _)| {
            let rest = &source[start..];
            let end = rest.find("\n}\n").map_or(rest.len(), |offset| offset + 3);
            &rest[..end]
        })
        .collect()
}

fn is_production_source(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
        && path.file_name().is_some_and(|name| name != "tests.rs")
}

fn pass_sources() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for crate_dir in [
        "crates/untwist-unminify/src",
        "crates/untwist-deobfuscate/src",
        "crates/untwist-jsx/src",
        "crates/untwist-rename/src",
    ] {
        let mut stack = vec![root().join(crate_dir)];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if is_production_source(&path) {
                    out.push(path);
                }
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_pass_impl_names_exactly_the_hooks_it_overrides() {
    let mut blocks = 0usize;
    for path in pass_sources() {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        for block in pass_impl_blocks(&source) {
            blocks += 1;
            let mut overridden = hook_names_in_impl_block(block);
            let mut declared = interest_names_in_impl_block(block);
            assert!(
                !declared.is_empty(),
                "{}: no fn interest found",
                path.display()
            );
            overridden.sort();
            declared.sort();
            assert_eq!(
                overridden,
                declared,
                "{}: the hooks the impl overrides and the interest it declares differ",
                path.display()
            );
        }
    }
    assert_eq!(
        blocks, 59,
        "the production pass census moved; re-count and update"
    );
}
