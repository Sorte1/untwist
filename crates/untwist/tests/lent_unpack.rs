use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_testing::{generated_bundle_present, js_files_in, real_world_fixture};

const MODES: [(bool, bool); 4] = [(false, true), (false, false), (true, true), (true, false)];

const UNPACK_SETS: [&str; 6] = [
    "fixtures/reference/unpack",
    "fixtures/unpack-gaps",
    "fixtures/reference/deobfuscate",
    "fixtures/unminify",
    "fixtures/jsx",
    "fixtures/deobfuscate-gaps",
];

fn mode(unsafe_transforms: bool, extended: bool) -> Options {
    Options {
        unsafe_transforms,
        extended,
        ..Options::default()
    }
}

fn notes(output: &Output) -> Vec<String> {
    output
        .notes
        .iter()
        .map(|note| {
            format!(
                "{} {:?} {} {:?}",
                note.pass, note.kind, note.message, note.span
            )
        })
        .collect()
}

fn stages(output: &Output) -> Vec<String> {
    output
        .stages
        .iter()
        .map(|stage| {
            format!(
                "{} {} {} {} {:?} {:?} {:?}",
                stage.stage,
                stage.round,
                stage.iterations,
                stage.changes,
                stage.per_pass,
                stage.fixpoint_per_pass,
                stage.stop
            )
        })
        .collect()
}

fn bundle(output: &Output) -> Option<Vec<String>> {
    let bundle = output.bundle.as_ref()?;
    let mut lines = vec![format!(
        "{} {:?} {:?}",
        bundle.kind, bundle.entry_id, bundle.chunk_ids
    )];
    lines.extend(bundle.modules.iter().map(|module| {
        format!(
            "{} {} {} {}",
            module.id, module.path, module.is_entry, module.code
        )
    }));
    Some(lines)
}

fn assert_same(label: &str, lent: &Output, reparsed: &Output) {
    assert_eq!(lent.codes, reparsed.codes, "{label}: codes");
    assert_eq!(bundle(lent), bundle(reparsed), "{label}: bundle");
    assert_eq!(notes(lent), notes(reparsed), "{label}: notes");
    assert_eq!(stages(lent), stages(reparsed), "{label}: stages");
    assert_eq!(
        (&lent.parse_diagnostics, lent.parse_errors),
        (&reparsed.parse_diagnostics, reparsed.parse_errors),
        "{label}: diagnostics"
    );
    assert_eq!(
        (&lent.panicked, &lent.depth_refused, lent.unpack_panicked),
        (
            &reparsed.panicked,
            &reparsed.depth_refused,
            reparsed.unpack_panicked
        ),
        "{label}: refusals"
    );
}

fn compare(inputs: &[(String, String)], options: &Options) -> (usize, usize) {
    let mut bundles = 0;
    for (label, source) in inputs {
        let pipeline = Pipeline::new(options.clone());
        let lent = pipeline.run(source);
        let reparsed = pipeline.run_reparsing_for_tests(&[source.as_str()]);
        assert_same(label, &lent, &reparsed);
        bundles += usize::from(lent.bundle.is_some());
    }
    (inputs.len(), bundles)
}

fn every_mode(inputs: &[(String, String)]) -> (usize, usize) {
    assert!(!inputs.is_empty(), "no inputs");
    MODES
        .iter()
        .fold((0, 0), |(runs, bundles), (unsafe_transforms, extended)| {
            let (more, found) = compare(inputs, &mode(*unsafe_transforms, *extended));
            (runs + more, bundles + found)
        })
}

fn game_js(index: usize) {
    let Some(source) = real_world_fixture("game.js") else {
        return;
    };
    let game = vec![("game.js".to_string(), source)];
    let (unsafe_transforms, extended) = MODES[index];
    assert_eq!(compare(&game, &mode(unsafe_transforms, extended)), (1, 0));
}

#[test]
fn game_js_default_unpacks_alike_lent_and_reparsed() {
    game_js(0);
}

#[test]
fn game_js_conservative_unpacks_alike_lent_and_reparsed() {
    game_js(1);
}

#[test]
fn game_js_unsafe_unpacks_alike_lent_and_reparsed() {
    game_js(2);
}

#[test]
fn game_js_unsafe_conservative_unpacks_alike_lent_and_reparsed() {
    game_js(3);
}

#[test]
fn the_high_preset_unpacks_alike_lent_and_reparsed() {
    let high = js_files_in("fixtures/reference/deobfuscate")
        .into_iter()
        .filter(|(label, _)| label.ends_with("obfuscator.io-high.js"))
        .collect::<Vec<_>>();
    assert_eq!(every_mode(&high).0, 4);
}

#[test]
fn the_corpora_unpack_alike_lent_and_reparsed() {
    for dir in ["fixtures/corpus-npm", "fixtures/behaviour"] {
        assert!(every_mode(&js_files_in(dir)).0 > 0);
    }
}

#[test]
fn the_diff_sets_unpack_alike_lent_and_reparsed() {
    let mut bundles = 0;
    for dir in UNPACK_SETS {
        bundles += every_mode(&js_files_in(dir)).1;
    }
    let mangle = Options {
        rename: RenameMode::Mangle,
        ..Options::default()
    };
    for dir in [
        "fixtures/unminify",
        "fixtures/reference/deobfuscate",
        "fixtures/reference/unpack",
    ] {
        bundles += compare(&js_files_in(dir), &mangle).1;
    }
    assert!(bundles > 0, "no diff-set input unpacked a bundle");
}

#[test]
fn the_generated_bundle_unpacks_alike_lent_and_reparsed() {
    if !generated_bundle_present() {
        return;
    }
    let options = Options {
        deobfuscate: false,
        unminify: false,
        ..Options::default()
    };
    assert_eq!(
        compare(&js_files_in("fixtures/generated"), &options),
        (1, 1)
    );
}

#[test]
fn several_inputs_unpack_alike_lent_and_reparsed() {
    let inputs = js_files_in("fixtures/reference/unpack");
    let sources = inputs
        .iter()
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>();
    let pipeline = Pipeline::new(Options::default());
    let lent = pipeline.run_many(&sources);
    let reparsed = pipeline.run_reparsing_for_tests(&sources);
    assert_same("all unpack fixtures", &lent, &reparsed);
    let plain = js_files_in("fixtures/unminify");
    let sources = plain
        .iter()
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>();
    let lent = pipeline.run_many(&sources);
    let reparsed = pipeline.run_reparsing_for_tests(&sources);
    assert_same("all unminify fixtures", &lent, &reparsed);
}
