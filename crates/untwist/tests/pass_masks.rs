use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Output, Pipeline, StopReason};
use untwist_core::pass::{probe_variants_for_tests, take_variant_violations_for_tests, Note};
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const MODES: [(&str, bool, bool); 4] = [
    ("default", false, true),
    ("conservative", false, false),
    ("unsafe", true, true),
    ("unsafe conservative", true, false),
];

fn options(unsafe_transforms: bool, extended: bool) -> Options {
    Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    }
}

type StageRow = (
    &'static str,
    u32,
    u32,
    u32,
    Vec<(&'static str, u32)>,
    Vec<(&'static str, u32)>,
    StopReason,
);

#[derive(Debug, PartialEq)]
struct Observed {
    codes: Vec<String>,
    notes: Vec<Note>,
    stages: Vec<StageRow>,
    panicked: Vec<usize>,
    depth_refused: Vec<usize>,
}

fn observed(out: Output) -> Observed {
    let stages = out
        .stages
        .iter()
        .map(|stage| {
            (
                stage.stage,
                stage.round,
                stage.iterations,
                stage.changes,
                stage.per_pass.clone(),
                stage.fixpoint_per_pass.clone(),
                stage.stop,
            )
        })
        .collect();
    Observed {
        codes: out.codes,
        notes: out.notes,
        stages,
        panicked: out.panicked,
        depth_refused: out.depth_refused,
    }
}

fn assert_declared_variants_hold(label: &str, source: &str) {
    for (mode, unsafe_transforms, extended) in MODES {
        take_variant_violations_for_tests();
        probe_variants_for_tests(true);
        let probed = Pipeline::new(options(unsafe_transforms, extended)).run(source);
        probe_variants_for_tests(false);
        let violations = take_variant_violations_for_tests();
        assert!(
            violations.is_empty(),
            "{label} ({mode}): a pass acted outside its declared variants: {violations:#?}"
        );
        let dispatched = Pipeline::new(options(unsafe_transforms, extended)).run(source);
        assert!(
            observed(probed) == observed(dispatched),
            "{label} ({mode}): dispatch by variant diverges from calling every pass on every node"
        );
    }
}

const DIFF_SETS: [&str; 5] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/behaviour",
];

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

fn diff_sets(slow: bool) -> Vec<(String, String)> {
    DIFF_SETS
        .iter()
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

#[test]
fn every_declared_variant_set_covers_what_its_pass_acts_on_over_the_diff_sets() {
    let _serial = serial();
    let files = diff_sets(false);
    assert!(files.len() >= 150, "{}", files.len());
    for (label, source) in files {
        assert_declared_variants_hold(&label, &source);
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test pass_masks"
)]
fn every_declared_variant_set_covers_what_its_pass_acts_on_over_game_js_and_the_high_preset() {
    let _serial = serial();
    let mut files = diff_sets(true);
    assert_eq!(files.len(), 2);
    files.extend(real_world_fixture("game.js").map(|source| ("game.js".to_string(), source)));
    for (label, source) in files {
        assert_declared_variants_hold(&label, &source);
    }
}
