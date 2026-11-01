use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist::{Options, Pipeline, RenameMode};
use untwist_core::parse::parse_program;
use untwist_core::pass::{run_to_fixpoint, PassGroup, PassOptions};
use untwist_core::semantic::{build_scoping, Semantics};
use untwist_rename::fresh::{plans_checked, scopings_checked, STALE};
use untwist_rename::{rename_passes, RenameConfig};

const MODES: [RenameMode; 3] = [RenameMode::Simple, RenameMode::Mangle, RenameMode::Unique];

const CORPORA: [&str; 4] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/corpus-npm",
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = CORPORA
        .iter()
        .flat_map(|dir| {
            std::fs::read_dir(repo_root().join(dir))
                .into_iter()
                .flatten()
        })
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "js"))
        .collect();
    files.sort();
    files
}

fn rename_only(mode: RenameMode) -> Options {
    Options {
        deobfuscate: false,
        unminify: false,
        unpack: false,
        jsx: false,
        rename: mode,
        ..Options::default()
    }
}

fn every_stage(mode: RenameMode) -> Options {
    Options {
        unsafe_transforms: true,
        rename: mode,
        ..Options::default()
    }
}

fn assert_fresh_through_the_pipeline(path: &Path, options: Options) {
    let (scopings, plans) = (scopings_checked(), plans_checked());
    let source = std::fs::read_to_string(path).expect("the fixture must be readable");
    let out = Pipeline::new(options).run(&source);
    let shown = path.display();
    assert!(out.panicked.is_empty(), "{shown}: a rename stage panicked");
    assert!(
        scopings_checked() > scopings,
        "{shown}: no scoping was checked"
    );
    assert!(plans_checked() > plans, "{shown}: no plan was replayed");
}

#[test]
fn every_mode_reuses_a_fresh_scoping_over_the_rename_fixtures_and_the_corpus() {
    for path in corpus_files() {
        for mode in MODES {
            assert_fresh_through_the_pipeline(&path, rename_only(mode));
            assert_fresh_through_the_pipeline(&path, every_stage(mode));
        }
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist --test simple_scoping"
)]
fn every_mode_reuses_a_fresh_scoping_on_game_js() {
    let Some(path) = untwist_testing::real_world_path("game.js") else {
        return;
    };
    for mode in MODES {
        assert_fresh_through_the_pipeline(&path, rename_only(mode));
        assert_fresh_through_the_pipeline(&path, every_stage(mode));
    }
}

fn config(mode: RenameMode) -> RenameConfig {
    RenameConfig {
        mode,
        rename_all: true,
        extended: true,
    }
}

fn rename_with<'a>(
    alloc: &'a Allocator,
    program: &mut Program<'a>,
    sem: Semantics,
    mode: RenameMode,
) {
    let mut group = PassGroup::new(rename_passes(config(mode)), PassOptions::default());
    let _ = run_to_fixpoint(alloc, program, sem, &mut group);
}

fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

fn assert_stale_is_caught(source: &str, edit: for<'a> fn(&'a Allocator, &mut Program<'a>)) {
    for mode in MODES {
        let alloc = Allocator::default();
        let mut parsed = parse_program(&alloc, source);
        let stale = build_scoping(&parsed.program);
        edit(&alloc, &mut parsed.program);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            rename_with(&alloc, &mut parsed.program, stale, mode);
        }));
        let Err(payload) = outcome else {
            panic!("{mode:?}: a scoping from before the edit was not caught");
        };
        assert!(
            panic_text(&*payload).contains(STALE),
            "{mode:?}: {}",
            panic_text(&*payload)
        );
    }
}

#[test]
fn a_scoping_from_before_a_rename_is_caught() {
    assert_stale_is_caught(
        "function outerName(firstArgument) { var secondValue = firstArgument + 1; return secondValue; }\nouterName(2);\n",
        |alloc, program| {
            let sem = build_scoping(program);
            rename_with(alloc, program, sem, RenameMode::Mangle);
        },
    );
}

#[test]
fn a_scoping_from_before_a_removed_declaration_is_caught() {
    assert_stale_is_caught(
        "var kept = 1;\nfunction dropped() { return kept; }\n",
        |_, program| {
            program.body.pop();
        },
    );
}
