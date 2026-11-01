use std::sync::{Mutex, MutexGuard, PoisonError};

use oxc_allocator::Allocator;
use untwist::{Options, Output, Pipeline};
use untwist_core::parse::parse_program;
use untwist_core::pass::{full_rounds_for_tests, stale_settled_for_tests, Note, PassOptions};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::run_cleanup;
use untwist_testing::js_files_in;

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const SPLIT_XY: &str = "console.log(\"xy\".split(\"\").reverse().join(\"\"));\n";

const SPLIT_ABC: &str = "console.log(\"abc\".split(\"\").reverse().join(\"\"));\n";

const SETTLE_CHAIN: &str =
    "function f() {\n  var o = { k: \"YW\" };\n  return atob(o.k + \"Jj\");\n}\nconsole.log(f());\n";

type StageRow = (&'static str, u32, u32, u32, Vec<(&'static str, u32)>);

#[derive(Debug, PartialEq)]
struct Observed {
    codes: Vec<String>,
    notes: Vec<Note>,
    stages: Vec<StageRow>,
}

fn observed(out: &Output) -> Observed {
    Observed {
        codes: out.codes.clone(),
        notes: out.notes.clone(),
        stages: out
            .stages
            .iter()
            .map(|stage| {
                (
                    stage.stage,
                    stage.round,
                    stage.iterations,
                    stage.changes,
                    stage.per_pass.clone(),
                )
            })
            .collect(),
    }
}

fn cleanup_replays(out: &Output) -> usize {
    out.stages
        .iter()
        .filter(|stage| stage.stage == "cleanup")
        .flat_map(|stage| stage.iteration_elapsed.iter())
        .filter(|timing| timing.traverse().is_zero())
        .count()
}

fn run(source: &str, full: bool) -> Output {
    full_rounds_for_tests(full);
    let out = Pipeline::new(Options::default()).run(source);
    full_rounds_for_tests(false);
    out
}

fn fresh_cleanup(source: &str) -> (String, u32, Vec<Note>) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let sem = build_scoping(&parsed.program);
    let (_, report) = run_cleanup(&alloc, &mut parsed.program, sem, &PassOptions::default());
    let printed = print(&parsed.program, PrintStyle::Compact);
    (printed, report.total_changes, report.notes)
}

#[test]
fn a_reused_pass_options_cleans_a_second_program_as_a_fresh_run_does() {
    let _serial = serial();
    for (first, second) in [(SPLIT_XY, SPLIT_ABC), (SPLIT_ABC, SPLIT_XY)] {
        let alloc = Allocator::default();
        let opts = PassOptions::default();
        let mut parsed = parse_program(&alloc, first);
        let sem = build_scoping(&parsed.program);
        let (_, report) = run_cleanup(&alloc, &mut parsed.program, sem, &opts);
        assert!(report.total_changes > 0);
        parsed = parse_program(&alloc, second);
        let sem = build_scoping(&parsed.program);
        let (_, report) = run_cleanup(&alloc, &mut parsed.program, sem, &opts);
        let reused = (
            print(&parsed.program, PrintStyle::Compact),
            report.total_changes,
            report.notes,
        );
        assert_eq!(reused, fresh_cleanup(second));
    }
}

#[test]
fn a_reused_pipeline_runs_each_input_as_a_fresh_pipeline_does() {
    let _serial = serial();
    let mut inputs: Vec<String> = vec![SPLIT_XY.into(), SPLIT_ABC.into(), SETTLE_CHAIN.into()];
    inputs.extend(
        js_files_in("fixtures/corpus-npm")
            .into_iter()
            .map(|(_, source)| source),
    );
    let pipeline = Pipeline::new(Options::default());
    for source in inputs.iter().chain(inputs.iter().rev()) {
        let reused = observed(&pipeline.run(source));
        let fresh = observed(&Pipeline::new(Options::default()).run(source));
        assert!(reused == fresh, "{source}");
    }
    let sources: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let together = pipeline.run_many(&sources).codes;
    let apart: Vec<String> = sources
        .iter()
        .flat_map(|source| Pipeline::new(Options::default()).run(source).codes)
        .collect();
    assert_eq!(together, apart);
}

#[test]
fn a_slot_left_by_a_run_without_unminify_never_reaches_the_next_run() {
    let _serial = serial();
    let options = Options {
        unminify: false,
        ..Options::default()
    };
    for (first, second) in [(SPLIT_XY, SPLIT_ABC), (SPLIT_ABC, SPLIT_XY)] {
        let pipeline = Pipeline::new(options.clone());
        let left = pipeline.run(first);
        assert!(!left.code().contains("split"), "{}", left.code());
        let reused = observed(&pipeline.run(second));
        let fresh = observed(&Pipeline::new(options.clone()).run(second));
        assert!(reused == fresh, "{second}: {reused:#?}");
        assert!(!fresh.codes.join("\n").contains("split"));
    }
}

#[test]
fn the_settle_slot_replays_cleanup_only_as_a_full_run_would_end() {
    let _serial = serial();
    for source in [SPLIT_XY, SETTLE_CHAIN] {
        let slotted = run(source, false);
        let full = run(source, true);
        assert!(observed(&slotted) == observed(&full), "{source}");
        assert!(
            cleanup_replays(&slotted) > 0,
            "{source}: {:#?}",
            slotted.stages
        );
        assert_eq!(cleanup_replays(&full), 0, "{source}");
    }
    assert!(!run(SETTLE_CHAIN, false).code().contains("atob"));
}

#[test]
fn a_settle_slot_that_ignores_edits_diverges_on_the_settle_chain() {
    let _serial = serial();
    stale_settled_for_tests(true);
    let stale = run(SETTLE_CHAIN, false);
    stale_settled_for_tests(false);
    let full = run(SETTLE_CHAIN, true);
    assert!(observed(&stale) != observed(&full));
    assert!(stale.code().contains("atob"), "{}", stale.code());
}
