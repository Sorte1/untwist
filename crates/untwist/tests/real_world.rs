use oxc_allocator::Allocator;
use untwist::{Options, Output, Pipeline, StopReason};
use untwist_core::parse::parse_program;
use untwist_testing::real_world_fixture;

const GAME_SELF_DEFENDING_STATEMENTS_REMOVED: u32 = 3;
const GAME_DEBUG_PROTECTIONS_REMOVED: u32 = 0;
const GAME_SELF_DEFENDING_MARKER: &str = "search(\"(((.+)+)+)+$\")";
const GAME_PROMOTED_PROLOGUE: &str = "(function () {\n  \"use strict\";\n";
const GAME_INERT_PROLOGUE: &str =
    "var b475796ed633d5fd0485;\nvar f9e2c8d63a4b7128b031;\n(function () {\n  (\"use strict\");\n";

fn assert_the_protections_stage_cleared_the_self_defending_block(out: &Output) {
    let protections = out
        .stages
        .iter()
        .find(|stage| stage.stage == "protections")
        .expect("protections stage reported");
    assert_eq!(
        protections.per_pass,
        vec![
            ("self-defending", GAME_SELF_DEFENDING_STATEMENTS_REMOVED),
            ("debug-protection", GAME_DEBUG_PROTECTIONS_REMOVED),
        ],
        "game.js carries one SingleCallController, its guarded variable and that variable's bare \
         call, and no debug protection at all"
    );
    assert!(
        !out.code().contains(GAME_SELF_DEFENDING_MARKER),
        "the self-defending callback survived"
    );
    assert!(
        !out.code().contains(GAME_PROMOTED_PROLOGUE),
        "the string the obfuscator put after its block became a directive, so sloppy code turned strict"
    );
    assert!(
        out.code().contains(GAME_INERT_PROLOGUE),
        "the string the removed block left at the head is not printed as an inert expression"
    );
}

fn assert_the_decode_peak_is_under_a_fifth_of_the_heap_budget(out: &Output, input_len: usize) {
    let peak = out.heap_peak;
    let budget = untwist_eval::budget::heap_budget(input_len);
    assert!(
        peak > 0,
        "game.js decodes, so its evaluator holds live charged bytes"
    );
    assert!(
        peak * 5 < budget,
        "game.js peaked at {peak} live bytes against a heap budget of {budget} B"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist --test real_world"
)]
fn game_js_unminifies_to_a_fixpoint_and_the_result_is_stable() {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    let pipeline = Pipeline::new(Options::default());

    let first = pipeline.run(&source);
    assert_eq!(first.parse_errors, 0, "{:?}", first.parse_diagnostics);
    for stage in &first.stages {
        assert_eq!(
            stage.stop,
            StopReason::Converged,
            "{} stopped early after {} iterations",
            stage.stage,
            stage.iterations
        );
    }

    assert_the_protections_stage_cleared_the_self_defending_block(&first);
    assert_the_decode_peak_is_under_a_fifth_of_the_heap_budget(&first, source.len());

    let alloc = Allocator::default();
    let reparsed = parse_program(&alloc, first.code());
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:?}",
        reparsed.diagnostics
    );

    let second = pipeline.run(first.code());
    for stage in &second.stages {
        assert_eq!(
            (stage.iterations, stage.changes),
            (1, 0),
            "{} is not idempotent",
            stage.stage
        );
    }
    assert_eq!(second.code(), first.code());
}
