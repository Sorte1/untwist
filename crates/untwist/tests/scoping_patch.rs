use std::num::NonZeroUsize;
use std::sync::{Mutex, MutexGuard, PoisonError};

use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_core::pass::Note;
use untwist_core::semantic::{
    check_patches_for_tests, force_rebuilds_for_tests, forget_failures,
    keep_one_dead_binding_for_tests, last_stale_for_tests, misplace_one_scope_for_tests,
    refresh_counts_for_tests, skip_one_deletion_for_tests, trust_id_order_for_tests, RefreshCounts,
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

type StageRow = (&'static str, u32, u32, u32, Vec<(&'static str, u32)>);

#[derive(Debug, PartialEq)]
struct Observed {
    codes: Vec<String>,
    notes: Vec<Note>,
    stages: Vec<StageRow>,
    panicked: Vec<usize>,
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
            )
        })
        .collect();
    Observed {
        codes: out.codes,
        notes: out.notes,
        stages,
        panicked: out.panicked,
    }
}

fn run_counted(source: &str, options: Options, forced: bool) -> (Observed, RefreshCounts) {
    forget_failures();
    let before = refresh_counts_for_tests();
    force_rebuilds_for_tests(forced);
    let out = Pipeline::new(options).run(source);
    force_rebuilds_for_tests(false);
    let after = refresh_counts_for_tests();
    let counts = RefreshCounts {
        patched: after.patched - before.patched,
        rebuilt: after.rebuilt - before.rebuilt,
        built: after.built - before.built,
        checked: after.checked - before.checked,
        stale: after.stale - before.stale,
    };
    (observed(out), counts)
}

fn rename_modes() -> [(&'static str, Options); 3] {
    let mode = |rename| Options {
        rename,
        ..Options::default()
    };
    [
        ("rename simple", mode(RenameMode::Simple)),
        ("rename mangle", mode(RenameMode::Mangle)),
        ("rename unique", mode(RenameMode::Unique)),
    ]
}

fn patches_equal_forced_rebuilds(label: &str, source: &str, renames: bool) -> usize {
    let mut patched = 0;
    let modes = modes()
        .into_iter()
        .chain(renames.then(rename_modes).into_iter().flatten());
    for (mode, options) in modes {
        let (rebuilt, forced) = run_counted(source, options.clone(), true);
        assert_eq!(
            forced.patched, 0,
            "{label} ({mode}): a forced round patched"
        );
        let (kept, counts) = run_counted(source, options, false);
        assert!(
            kept == rebuilt,
            "{label} ({mode}): the patched rounds diverge from forced rebuilds"
        );
        patched += counts.patched;
    }
    patched
}

#[test]
fn every_corpus_output_with_patches_equals_forced_rebuilds() {
    let _serial = serial();
    let files = corpora(false);
    assert!(files.len() >= 150, "{}", files.len());
    let patched: usize = files
        .iter()
        .map(|(label, source)| patches_equal_forced_rebuilds(label, source, true))
        .sum();
    assert!(patched >= 500, "only {patched} rounds were patched");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the high preset, the multi-encoder fixture and the 10.7 MB game.js; run with cargo \
              test --release -p untwist --test scoping_patch"
)]
fn game_js_and_the_high_preset_with_patches_equal_forced_rebuilds() {
    let _serial = serial();
    let mut files = corpora(true);
    files.extend(real_world_fixture("game.js").map(|source| ("game.js".to_string(), source)));
    for (label, source) in files {
        let renames = label == "game.js";
        let patched = patches_equal_forced_rebuilds(&label, &source, renames);
        assert!(patched > 0, "{label}: no round was patched");
    }
}

const REBUILDS_ON_GAME_JS: usize = 4;

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "the 10.7 MB game.js; run with cargo test --release -p untwist --test scoping_patch"
)]
fn game_js_rebuilds_its_scoping_at_most_four_times() {
    let _serial = serial();
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    for (mode, options) in modes() {
        let (_, counts) = run_counted(&source, options, false);
        println!(
            "game.js ({mode}): {} built, {} rebuilt, {} patched",
            counts.built, counts.rebuilt, counts.patched
        );
        assert!(
            counts.rebuilt <= REBUILDS_ON_GAME_JS,
            "game.js ({mode}): {} rebuilds",
            counts.rebuilt
        );
    }
}

fn stale_under(mutant: fn(bool), source: &str) -> bool {
    forget_failures();
    let before = refresh_counts_for_tests();
    mutant(true);
    check_patches_for_tests(true);
    let out = Pipeline::new(Options::default()).run(source);
    check_patches_for_tests(false);
    mutant(false);
    let after = refresh_counts_for_tests();
    after.stale > before.stale && !out.panicked.is_empty()
}

const REPARENTED: &str = "if (a) {\n  b();\n} else {\n  if (c) {\n    d();\n  }\n}\n";

#[test]
fn a_patch_that_leaves_a_scope_under_its_old_parent_turns_s1_red() {
    let _serial = serial();
    assert!(
        !stale_under(|_| {}, REPARENTED),
        "the clean patch went stale"
    );
    assert!(
        stale_under(misplace_one_scope_for_tests, REPARENTED),
        "the misplaced scope went unseen"
    );
}

const UNLINKED: &str = "fixtures/unminify/rename-derived-names.js";

#[test]
fn a_patch_that_leaves_a_removed_binding_linked_turns_s1_red_on_a_named_fixture() {
    let _serial = serial();
    let source = js_files_in("fixtures/unminify")
        .into_iter()
        .find(|(label, _)| label.ends_with("rename-derived-names.js"))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{UNLINKED}"));
    assert!(
        !stale_under(|_| {}, &source),
        "{UNLINKED}: the clean patch went stale"
    );
    assert!(
        stale_under(keep_one_dead_binding_for_tests, &source),
        "{UNLINKED}: the linked binding went unseen"
    );
}

const ID_ORDERED: &str = "fixtures/unminify/for-loops.js";

#[test]
fn a_rename_plan_read_in_id_order_from_a_patched_scoping_turns_c7_red_on_a_named_fixture() {
    let _serial = serial();
    let source = js_files_in("fixtures/unminify")
        .into_iter()
        .find(|(label, _)| label.ends_with("for-loops.js"))
        .map(|(_, source)| source)
        .unwrap_or_else(|| panic!("{ID_ORDERED}"));
    let options = Options {
        rename: RenameMode::Mangle,
        ..Options::default()
    };
    let (rebuilt, _) = run_counted(&source, options.clone(), true);
    let (kept, _) = run_counted(&source, options.clone(), false);
    assert!(kept == rebuilt, "{ID_ORDERED}: the patched rounds diverge");
    trust_id_order_for_tests(true);
    let (trusted, _) = run_counted(&source, options, false);
    trust_id_order_for_tests(false);
    assert!(
        trusted != rebuilt,
        "{ID_ORDERED}: the id-ordered rename plan went unseen"
    );
}
