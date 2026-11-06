use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Pipeline};
use untwist_core::pass::{cascade_off_for_tests, cascade_root_lists_for_tests, Note};
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const SETS: [&str; 13] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/behaviour",
    "fixtures/corpus-npm",
    "fixtures/untranspile/babel",
    "fixtures/untranspile/esbuild",
    "fixtures/untranspile/hand",
    "fixtures/untranspile/samples",
    "fixtures/untranspile/swc",
    "fixtures/untranspile/terser",
    "fixtures/untranspile/ts",
];

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

const MODES: [(bool, bool); 4] = [(false, true), (false, false), (true, true), (true, false)];

fn files(slow: bool) -> Vec<(String, String)> {
    SETS.iter()
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

fn options((unsafe_transforms, extended): (bool, bool)) -> Options {
    Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    }
}

fn run(source: &str, mode: (bool, bool), off: bool) -> (Vec<String>, Vec<Note>, u32) {
    cascade_off_for_tests(off);
    let out = Pipeline::new(options(mode)).run(source);
    cascade_off_for_tests(false);
    let iterations = out
        .stages
        .iter()
        .filter(|stage| stage.stage == "unminify")
        .map(|stage| stage.iterations)
        .sum();
    (out.codes, out.notes, iterations)
}

fn assert_cascade_equals_next_iterations(label: &str, source: &str) {
    for mode in MODES {
        let (codes, notes, iterations) = run(source, mode, false);
        let (plain_codes, plain_notes, plain_iterations) = run(source, mode, true);
        assert!(
            codes == plain_codes,
            "{label} {mode:?}: the walked replacement changes the output"
        );
        assert!(
            notes == plain_notes,
            "{label} {mode:?}: the walked replacement changes the notes"
        );
        assert!(
            iterations <= plain_iterations,
            "{label} {mode:?}: {iterations} > {plain_iterations}"
        );
    }
}

#[test]
fn the_corpora_equal_runs_that_leave_replacements_to_the_next_iteration() {
    let _serial = serial();
    let files = files(false);
    assert!(files.len() >= 400, "{}", files.len());
    for (label, source) in files {
        assert_cascade_equals_next_iterations(&label, &source);
    }
}

#[test]
fn a_walk_that_hands_the_replacement_to_list_hooks_diverges_on_a_named_source() {
    let _serial = serial();
    let source = "function f(x, y, z) { (x(), y) && (z(), x()); }\n";
    let (plain, _, _) = run(source, MODES[0], true);
    let (codes, _, _) = run(source, MODES[0], false);
    assert_eq!(codes, plain);
    cascade_root_lists_for_tests(true);
    let (mutant, _, _) = run(source, MODES[0], false);
    cascade_root_lists_for_tests(false);
    assert_ne!(mutant, plain);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test cascade_walk"
)]
fn game_js_and_the_high_preset_equal_runs_that_leave_replacements_to_the_next_iteration() {
    let _serial = serial();
    let slow = files(true);
    assert_eq!(slow.len(), 2);
    for (label, source) in slow {
        assert_cascade_equals_next_iterations(&label, &source);
    }
    if let Some(source) = real_world_fixture("game.js") {
        assert_cascade_equals_next_iterations("game.js", &source);
        let (_, _, iterations) = run(&source, MODES[0], false);
        assert!(
            iterations <= 5,
            "unminify took {iterations} iterations on game.js"
        );
    }
}
