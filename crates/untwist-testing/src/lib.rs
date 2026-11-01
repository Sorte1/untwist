pub mod capped_child;
pub mod depth_classes;
pub mod depth_diff;
pub mod depth_edges;
pub mod depth_grid;
pub mod depth_mix;
pub mod depth_names;
pub mod depth_oracle;
pub mod depth_reference;
pub mod depth_soup;
pub mod node;
pub mod node_rows;
pub mod notes;

use untwist_core::parse::{parse_program, parse_with, Parsed, SourceType};
use untwist_core::pass::{run_to_fixpoint, PassCtor, PassGroup, PassOptions, RunReport};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;

pub use insta;
pub use oxc_allocator::Allocator;
pub use untwist_core::depth;

pub fn transform<'a>(alloc: &'a Allocator, source: &'a str, ctors: &[PassCtor<'a>]) -> String {
    transform_with(
        alloc,
        source,
        ctors,
        PassOptions {
            unsafe_transforms: true,
            ..PassOptions::default()
        },
    )
}

pub fn transform_with<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
    opts: PassOptions,
) -> String {
    transform_report(alloc, source, ctors, opts).0
}

pub fn transform_as<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    source_type: SourceType,
    ctors: &[PassCtor<'a>],
) -> String {
    let opts = PassOptions {
        unsafe_transforms: true,
        ..PassOptions::default()
    };
    run_group(
        alloc,
        parse_with(alloc, source, source_type),
        PassGroup::from_ctors(ctors, opts),
    )
    .0
}

pub fn transform_report<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    ctors: &[PassCtor<'a>],
    opts: PassOptions,
) -> (String, RunReport) {
    transform_report_with(alloc, source, PassGroup::from_ctors(ctors, opts))
}

pub fn transform_report_with<'a>(
    alloc: &'a Allocator,
    source: &'a str,
    group: PassGroup<'a>,
) -> (String, RunReport) {
    run_group(alloc, parse_program(alloc, source), group)
}

fn run_group<'a>(
    alloc: &'a Allocator,
    mut parsed: Parsed<'a>,
    mut group: PassGroup<'a>,
) -> (String, RunReport) {
    assert!(
        parsed.diagnostics.is_empty(),
        "test input must parse cleanly: {:?}",
        parsed.diagnostics
    );
    let sem = build_scoping(&parsed.program);
    let (_sem, report) = run_to_fixpoint(alloc, &mut parsed.program, sem, &mut group);
    assert!(
        report.converged(),
        "passes did not converge: stop={:?} per_pass={:?}",
        report.stop,
        report.per_pass
    );
    (print(&parsed.program, PrintStyle::Pretty), report)
}

#[macro_export]
macro_rules! assert_transform {
    ($source:expr, [$($ctor:expr),+ $(,)?], @$expected:literal) => {{
        let alloc = $crate::Allocator::default();
        let source: &str = $source;
        let ctors: &[$crate::PassCtorAlias<'_>] = &[$($ctor),+];
        $crate::insta::assert_snapshot!($crate::transform(&alloc, source, ctors), @$expected)
    }};
}

pub type PassCtorAlias<'a> = PassCtor<'a>;

fn workspace_path(relative: &str, name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join(relative).join(name))
        .unwrap_or_default()
}

pub fn real_world_path(name: &str) -> Option<std::path::PathBuf> {
    let path = workspace_path("fixtures/real-world", name);
    if path.is_file() {
        return Some(path);
    }
    eprintln!("skipping fixtures/real-world/{name}: the file is not present");
    None
}

pub fn real_world_fixture(name: &str) -> Option<String> {
    let path = real_world_path(name)?;
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("unreadable fixture {}: {error}", path.display()));
    Some(text)
}

pub fn read_fixture(dir: &str, name: &str) -> String {
    let path = workspace_path(&format!("fixtures/reference/{dir}"), name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {}", path.display()))
}

pub fn read_gap_fixture(name: &str) -> String {
    let path = workspace_path("fixtures/deobfuscate-gaps", name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {}", path.display()))
}

pub fn read_unpack_gap_fixture(name: &str) -> String {
    let path = workspace_path("fixtures/unpack-gaps", name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing fixture {}", path.display()))
}

pub fn big_bundle_required() -> bool {
    std::env::var("UNTWIST_REQUIRE_BIG_BUNDLE").is_ok_and(|value| !value.is_empty())
}

pub fn generated_bundle_present() -> bool {
    let present = workspace_path("fixtures/generated", "webpack-5mb.js").is_file();
    assert!(
        present || !big_bundle_required(),
        "UNTWIST_REQUIRE_BIG_BUNDLE is set but fixtures/generated/webpack-5mb.js is missing; \
         run cargo run -p xtask -- corpus-bundle"
    );
    if !present {
        eprintln!("skipping fixtures/generated: run `cargo run -p xtask -- corpus-bundle` first");
    }
    present
}

pub fn js_files_in(dir: &str) -> Vec<(String, String)> {
    let dir = workspace_path(dir, "");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {}: {err}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
            (path.display().to_string(), text)
        })
        .collect()
}
