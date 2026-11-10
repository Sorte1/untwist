use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Pipeline};
use untwist_core::pass::{check_shared_spans_for_tests, take_shared_span_violations_for_tests};
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const MODES: [(&str, bool); 2] = [("default", false), ("unsafe", true)];

const SETS: [&str; 5] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/behaviour",
];

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

fn files(slow: bool) -> Vec<(String, String)> {
    SETS.iter()
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

fn violations_in(source: &str, unsafe_transforms: bool) -> Vec<String> {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    take_shared_span_violations_for_tests();
    check_shared_spans_for_tests(true);
    Pipeline::new(options).run(source);
    check_shared_spans_for_tests(false);
    take_shared_span_violations_for_tests()
}

fn assert_no_shared_spans(files: Vec<(String, String)>) {
    let mut failures = Vec::new();
    for (label, source) in files {
        for (mode, unsafe_transforms) in MODES {
            for violation in violations_in(&source, unsafe_transforms) {
                failures.push(format!("{label} ({mode}) {violation}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "an expression and its child share one span: {failures:#?}"
    );
}

#[test]
fn a_built_parent_expression_never_shares_the_span_of_its_child_over_the_fixtures() {
    let _serial = serial();
    let files = files(false);
    assert!(files.len() >= 150, "{}", files.len());
    assert_no_shared_spans(files);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test shared_spans"
)]
fn a_built_parent_expression_never_shares_the_span_of_its_child_over_game_js_and_the_high_preset() {
    let _serial = serial();
    let mut files = files(true);
    assert_eq!(files.len(), 2);
    files.extend(real_world_fixture("game.js").map(|source| ("game.js".to_string(), source)));
    assert_no_shared_spans(files);
}
