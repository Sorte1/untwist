use std::num::NonZeroUsize;
use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Pipeline};
use untwist_core::semantic::{
    check_patches_for_tests, forget_failures, last_stale_for_tests, refresh_counts_for_tests,
    skip_one_deletion_for_tests,
};
use untwist_testing::{js_files_in, real_world_fixture};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

const DIFF_SETS: [&str; 6] = [
    "fixtures/unminify",
    "fixtures/reference/deobfuscate",
    "fixtures/reference/unpack",
    "fixtures/jsx",
    "fixtures/behaviour",
    "fixtures/corpus-npm",
];

const SLOW_IN_DEBUG: [&str; 2] = ["obfuscator.io-high.js", "obfuscator.io-multi-encoders.js"];

const NAMED: &str = "fixtures/reference/deobfuscate/obfuscator.io.js";

fn corpora(slow: bool) -> Vec<(String, String)> {
    DIFF_SETS
        .iter()
        .flat_map(|dir| js_files_in(dir))
        .filter(|(label, _)| SLOW_IN_DEBUG.iter().any(|name| label.ends_with(name)) == slow)
        .collect()
}

fn modes() -> [(&'static str, Options); 5] {
    let mode = |unsafe_transforms, extended| Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    };
    [
        ("default", mode(false, true)),
        ("conservative", mode(false, false)),
        ("unsafe", mode(true, true)),
        ("unsafe conservative", mode(true, false)),
        (
            "unsafe, no unpack, no jsx",
            Options {
                unpack: false,
                jsx: false,
                ..mode(true, true)
            },
        ),
    ]
}

fn patches_equal_fresh_builds(label: &str, source: &str) -> usize {
    let mut patched = 0;
    for (mode, options) in modes() {
        forget_failures();
        let before = refresh_counts_for_tests();
        check_patches_for_tests(true);
        let out = Pipeline::new(options).run(source);
        check_patches_for_tests(false);
        let after = refresh_counts_for_tests();
        assert_eq!(
            after.stale,
            before.stale,
            "{label} ({mode}): a patched scoping differs from a fresh build: {}",
            last_stale_for_tests()
        );
        assert!(
            out.panicked.is_empty(),
            "{label} ({mode}): a stage panicked"
        );
        assert_eq!(
            after.patched - before.patched,
            after.checked - before.checked,
            "{label} ({mode}): a patched scoping went unchecked"
        );
        patched += after.patched - before.patched;
    }
    patched
}

#[test]
fn every_patched_scoping_over_the_corpora_equals_a_fresh_build() {
    let _serial = serial();
    let files = corpora(false);
    assert!(files.len() >= 150, "{}", files.len());
    let patched: usize = files
        .iter()
        .map(|(label, source)| patches_equal_fresh_builds(label, source))
        .sum();
    assert!(patched >= 500, "only {patched} rounds were patched");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test scoping_patch"
)]
fn game_js_and_the_high_preset_patch_like_fresh_builds() {
    let _serial = serial();
    let mut files = corpora(true);
    assert_eq!(files.len(), 2);
    files.extend(real_world_fixture("game.js").map(|source| ("game.js".to_string(), source)));
    for (label, source) in files {
        let patched = patches_equal_fresh_builds(&label, &source);
        assert!(patched > 0, "{label}: no round was patched");
    }
}

#[test]
fn a_patch_that_skips_one_deletion_turns_s1_red_on_a_named_fixture() {
    let _serial = serial();
    let source = js_files_in("fixtures/reference/deobfuscate")
        .into_iter()
        .find(|(label, _)| label.ends_with("obfuscator.io.js"))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{NAMED}"));
    forget_failures();
    let before = refresh_counts_for_tests();
    skip_one_deletion_for_tests(true);
    check_patches_for_tests(true);
    let out = Pipeline::new(Options::default()).run(&source);
    check_patches_for_tests(false);
    skip_one_deletion_for_tests(false);
    let after = refresh_counts_for_tests();
    assert!(
        after.stale > before.stale,
        "{NAMED}: the skipped deletion went unseen"
    );
    assert!(!out.panicked.is_empty(), "{NAMED}: no stage stopped");
}

fn named_fixture() -> String {
    js_files_in("fixtures/reference/deobfuscate")
        .into_iter()
        .find(|(label, _)| label.ends_with("obfuscator.io.js"))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{NAMED}"))
}

fn patched_on_one_thread(sources: &[&str]) -> usize {
    forget_failures();
    let before = refresh_counts_for_tests();
    let options = Options {
        jobs: NonZeroUsize::new(1),
        ..Options::default()
    };
    let out = Pipeline::new(options).run_many(sources);
    assert!(out.panicked.is_empty(), "a stage panicked");
    refresh_counts_for_tests().patched - before.patched
}

#[test]
fn a_failed_patch_in_one_input_leaves_the_next_input_patching() {
    let _serial = serial();
    let source = named_fixture();
    let poisoning = format!("{{ function sloppy() {{}} }}\n{source}");
    let alone = patched_on_one_thread(&[&source]);
    assert!(alone > 0, "{NAMED}: no round was patched");
    let poisoned = patched_on_one_thread(&[&poisoning]);
    assert_eq!(
        patched_on_one_thread(&[&poisoning, &source]),
        poisoned + alone,
        "{NAMED}: an earlier input's failed patches kept this input from patching"
    );
}

fn lexical_patches(source: &str) -> usize {
    let count = |rename| {
        forget_failures();
        let before = refresh_counts_for_tests();
        check_patches_for_tests(true);
        let out = Pipeline::new(Options {
            rename,
            ..Options::default()
        })
        .run(source);
        check_patches_for_tests(false);
        let after = refresh_counts_for_tests();
        assert_eq!(after.stale, before.stale, "{}", last_stale_for_tests());
        assert!(out.panicked.is_empty(), "a stage panicked");
        after.patched - before.patched
    };
    let without = count(untwist::RenameMode::None);
    count(untwist::RenameMode::Simple).saturating_sub(without)
}

#[test]
fn the_lexical_round_before_rename_patches_a_let_made_const() {
    let _serial = serial();
    let source =
        "function f(a) {\n  let b = a + 1;\n  var c = b * 2;\n  return c;\n}\nconsole.log(f(1));\n";
    assert!(lexical_patches(source) > 0, "the lexical round was rebuilt");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 10.7 MB game.js; run with cargo test --release -p untwist --test scoping_patch"
)]
fn game_js_patches_its_lexical_round_before_rename() {
    let _serial = serial();
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    assert!(
        lexical_patches(&source) > 0,
        "game.js: the lexical round was rebuilt"
    );
}
