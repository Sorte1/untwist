use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Pipeline};
use untwist_core::pass::Note;
use untwist_eval::test_hooks::keep_unit_outcomes;
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

const LANES: [bool; 2] = [false, true];

fn run(source: &str, unsafe_transforms: bool, keeping: bool) -> (Vec<String>, Vec<Note>, u64) {
    keep_unit_outcomes(keeping);
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    let out = Pipeline::new(options).run(source);
    keep_unit_outcomes(true);
    (out.codes, out.notes, out.stats.decode.steps)
}

fn steps_equal_to_fresh_units(label: &str, source: &str) -> (u64, u64) {
    let mut spent = (0, 0);
    for unsafe_transforms in LANES {
        let (codes, notes, kept) = run(source, unsafe_transforms, true);
        let (fresh_codes, fresh_notes, fresh) = run(source, unsafe_transforms, false);
        assert!(
            codes == fresh_codes,
            "{label} unsafe {unsafe_transforms}: a kept unit changes the output"
        );
        assert!(
            notes == fresh_notes,
            "{label} unsafe {unsafe_transforms}: a kept unit changes the notes"
        );
        assert!(kept <= fresh, "{label}: {kept} > {fresh}");
        spent = (spent.0 + kept, spent.1 + fresh);
    }
    spent
}

fn deobfuscate_fixtures(slow: bool) -> Vec<(String, String)> {
    ["fixtures/reference/deobfuscate", "fixtures/behaviour"]
        .iter()
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

#[test]
fn kept_units_equal_fresh_units_over_the_deobfuscate_fixtures() {
    let _serial = serial();
    let files = deobfuscate_fixtures(false);
    assert!(files.len() >= 100, "{}", files.len());
    for (label, source) in files {
        steps_equal_to_fresh_units(&label, &source);
    }
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset corpus and the 10.7 MB game.js; run with cargo test --release -p \
              untwist --test unit_outcomes"
)]
fn kept_units_equal_fresh_units_on_the_high_preset_and_game_js() {
    let _serial = serial();
    let mut files = deobfuscate_fixtures(true);
    files.extend(
        js_files_in("fixtures/obfuscator-io")
            .into_iter()
            .filter(|(label, _)| label.contains("preset-high--")),
    );
    assert_eq!(files.len(), 12);
    let (kept, fresh) = files
        .iter()
        .map(|(label, source)| steps_equal_to_fresh_units(label, source))
        .fold((0, 0), |sum, (kept, fresh)| (sum.0 + kept, sum.1 + fresh));
    assert!(kept < fresh, "no unit was kept: {kept} steps of {fresh}");
    if let Some(source) = real_world_fixture("game.js") {
        steps_equal_to_fresh_units("game.js", &source);
    }
}
