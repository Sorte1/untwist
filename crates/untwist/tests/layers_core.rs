use std::num::NonZeroUsize;

use untwist::{Options, Output, Pipeline, RenameMode, TestHooks};
use untwist_deobfuscate::layers::notes::{
    LAYER_CAP, LAYER_CYCLE, LAYER_FUEL, LAYER_NOTES, LAYER_PEELED, LAYER_TOO_DEEP, LAYER_TOO_LARGE,
    LAYER_UNPARSABLE,
};

const IDENTITY: &[(&str, &str)] = &[];

const CYCLE: &[(&str, &str)] = &[("a", "eval(\"b\")"), ("b", "eval(\"a\")")];

const QUINE: &[(&str, &str)] = &[("q", "eval(\"q\")")];

const UNPARSABLE_SECOND: &[(&str, &str)] = &[("a", "eval(\"bad\")"), ("bad", "f(((")];

const PADDED_CHAIN: &[(&str, &str)] = &[
    (
        "k1",
        "eval(\"k2\"); /* padding padding padding padding padding padding padding padding pad */",
    ),
    (
        "k2",
        "eval(\"k3\"); /* padding padding padding padding padding padding padding padding pad */",
    ),
    (
        "k3",
        "eval(\"k4\"); /* padding padding padding padding padding padding padding padding pad */",
    ),
    (
        "k4",
        "eval(\"k5\"); /* padding padding padding padding padding padding padding padding pad */",
    ),
    (
        "k5",
        "eval(\"k6\"); /* padding padding padding padding padding padding padding padding pad */",
    ),
];

fn options(table: &'static [(&'static str, &'static str)]) -> Options {
    Options {
        unpack: false,
        jsx: false,
        test_hooks: TestHooks {
            layer_stub: Some(table),
            ..TestHooks::default()
        },
        ..Options::default()
    }
}

fn disabled(options: Options) -> Options {
    Options {
        disabled_passes: vec!["layers"],
        ..options
    }
}

fn wrap(payload: &str, layers: usize) -> String {
    (0..layers).fold(payload.to_string(), |text, _| format!("eval({text:?});"))
}

fn peels(out: &Output) -> usize {
    out.stages
        .iter()
        .filter(|stage| stage.stage == "layers")
        .count()
}

fn notes_with(out: &Output, text: &str) -> Vec<String> {
    out.notes
        .iter()
        .filter(|note| note.message.starts_with(text))
        .map(|note| note.message.clone())
        .collect()
}

fn refused_with(out: &Output, text: &str, index: u32) -> bool {
    let named = format!("{text}; layer {index}, W-eval");
    out.notes.iter().any(|note| note.message == named)
}

fn nested_arrays(units: usize) -> String {
    let depth = units / 2;
    format!("console.log({}0{})", "[".repeat(depth), "]".repeat(depth))
}

#[test]
fn a_seventeen_layer_chain_stops_at_sixteen_with_the_cap() {
    let source = wrap("console.log(1)", 17);
    let out = Pipeline::new(options(IDENTITY)).run(&source);
    assert_eq!(peels(&out), 16);
    assert_eq!(notes_with(&out, LAYER_PEELED).len(), 16);
    assert!(refused_with(&out, LAYER_CAP, 17), "{:?}", out.notes);
    assert_eq!(out.code().trim(), "eval(\"console.log(1)\");");
}

#[test]
fn sixteen_layers_peel_to_the_payload() {
    let source = wrap("console.log(1)", 16);
    let out = Pipeline::new(options(IDENTITY)).run(&source);
    assert_eq!(peels(&out), 16);
    assert!(notes_with(&out, LAYER_CAP).is_empty());
    assert_eq!(out.code().trim(), "console.log(1);");
}

#[test]
fn a_two_text_cycle_stops_with_the_cycle_note() {
    let out = Pipeline::new(options(CYCLE)).run("eval(\"a\")");
    assert_eq!(peels(&out), 1);
    assert!(refused_with(&out, LAYER_CYCLE, 2), "{:?}", out.notes);
    assert_eq!(out.code().trim(), "eval(\"b\");");
    let quine = Pipeline::new(options(QUINE)).run("eval(\"q\")\n");
    assert_eq!(peels(&quine), 0);
    assert!(refused_with(&quine, LAYER_CYCLE, 1), "{:?}", quine.notes);
}

#[test]
fn a_layer_deeper_than_the_inputs_reservation_is_refused() {
    let (deep, fits) = if cfg!(debug_assertions) {
        (150, 120)
    } else {
        (300, 240)
    };
    let refused = Pipeline::new(options(IDENTITY)).run(&wrap(&nested_arrays(deep), 1));
    assert_eq!(peels(&refused), 0);
    assert!(
        refused_with(&refused, LAYER_TOO_DEEP, 1),
        "{:?}",
        refused.notes
    );
    let peeled = Pipeline::new(options(IDENTITY)).run(&wrap(&nested_arrays(fits), 1));
    assert_eq!(peels(&peeled), 1, "{:?}", peeled.notes);
}

fn layered_inputs() -> Vec<String> {
    (0..4)
        .map(|index| {
            let payload = format!(
                "var value{index} = {index}; function twice{index}(x) {{ var y = x + value{index}; return y + y; }} console.log(twice{index}({index}));"
            );
            wrap(&payload, 1 + index % 2)
        })
        .collect()
}

fn unique(jobs: usize) -> Options {
    Options {
        jobs: NonZeroUsize::new(jobs),
        rename: RenameMode::Unique,
        ..options(IDENTITY)
    }
}

#[test]
fn four_layered_inputs_under_unique_rename_match_their_sequential_outputs() {
    let inputs = layered_inputs();
    let sources: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let parallel = Pipeline::new(unique(4)).run_many(&sources);
    let sequential = Pipeline::new(unique(1)).run_many(&sources);
    assert_eq!(parallel.codes, sequential.codes);
    assert_eq!(peels(&parallel), 6);
    for code in &parallel.codes {
        assert!(!code.contains("eval("), "{code}");
    }
}

#[test]
fn a_layer_over_the_text_budget_is_refused() {
    let payload = format!("console.log(\"{}\")", "a".repeat(1_500));
    let small_heap = Options {
        test_hooks: TestHooks {
            layer_stub: Some(IDENTITY),
            heap_budget: Some(4_000),
            ..TestHooks::default()
        },
        ..options(IDENTITY)
    };
    let source = wrap(&payload, 1);
    let out = Pipeline::new(small_heap.clone()).run(&source);
    assert!(refused_with(&out, LAYER_TOO_LARGE, 1), "{:?}", out.notes);
    let kept = Pipeline::new(disabled(small_heap)).run(&source);
    assert_eq!(out.code(), kept.code());
}

#[test]
fn a_layer_over_its_fuel_is_refused() {
    let fueled = |fuel| Options {
        fuel: Some(fuel),
        ..options(IDENTITY)
    };
    let source = wrap(&format!("console.log(\"{}\")", "a".repeat(150)), 1);
    let out = Pipeline::new(fueled(100)).run(&source);
    assert!(refused_with(&out, LAYER_FUEL, 1), "{:?}", out.notes);
    let enough = Pipeline::new(fueled(1_000)).run(&source);
    assert_eq!(peels(&enough), 1);
}

#[test]
fn all_layers_of_an_input_share_four_times_the_fuel() {
    let out = Pipeline::new(Options {
        fuel: Some(100),
        ..options(PADDED_CHAIN)
    })
    .run("eval(\"k1\")");
    assert_eq!(peels(&out), 4, "{:?}", out.notes);
    assert!(refused_with(&out, LAYER_FUEL, 5), "{:?}", out.notes);
}

#[test]
fn a_refused_layer_prints_as_the_run_with_layers_disabled() {
    let cases = [
        (wrap("f(((", 1), LAYER_UNPARSABLE),
        ("eval(\"f(\" + \"((\")".to_string(), LAYER_UNPARSABLE),
        (wrap(&nested_arrays(800), 1), LAYER_TOO_DEEP),
    ];
    for (source, note) in cases {
        let out = Pipeline::new(options(IDENTITY)).run(&source);
        let kept = Pipeline::new(disabled(options(IDENTITY))).run(&source);
        assert_eq!(peels(&out), 0, "{source}");
        assert!(refused_with(&out, note, 1), "{source}: {:?}", out.notes);
        assert_eq!(out.code(), kept.code(), "{source}");
        assert_eq!(out.stages.len(), kept.stages.len(), "{source}");
    }
}

#[test]
fn an_unparsable_second_layer_keeps_the_first_peel() {
    let out = Pipeline::new(options(UNPARSABLE_SECOND)).run("eval(\"a\")");
    assert_eq!(peels(&out), 1);
    assert!(refused_with(&out, LAYER_UNPARSABLE, 2), "{:?}", out.notes);
    assert_eq!(out.code().trim(), "eval(\"bad\");");
}

#[test]
fn a_layer_the_deobfuscate_stage_folds_to_a_string_peels_after_it() {
    let out = Pipeline::new(options(IDENTITY)).run("eval(\"console.\" + \"log(1)\")");
    assert_eq!(peels(&out), 1, "{:?}", out.notes);
    assert_eq!(out.code().trim(), "console.log(1);");
}

#[test]
fn layers_peel_only_in_the_extended_deobfuscating_modes() {
    let source = wrap("console.log(1)", 1);
    let conservative = Options {
        extended: false,
        ..options(IDENTITY)
    };
    let plain = Options {
        deobfuscate: false,
        ..options(IDENTITY)
    };
    for options in [conservative, plain, disabled(options(IDENTITY))] {
        let out = Pipeline::new(options).run(&source);
        assert_eq!(peels(&out), 0);
        assert!(out.notes.iter().all(|note| note.pass != "layers"));
    }
}

#[test]
fn without_the_stub_the_real_decoder_peels_both_layers() {
    let source = wrap("console.log(1)", 2);
    let out = Pipeline::new(Options {
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(&source);
    assert_eq!(peels(&out), 2, "{:?}", out.notes);
    assert_eq!(out.code().trim(), "console.log(1);");
}

#[test]
fn every_layer_note_constant_is_catalogued_exactly_once() {
    untwist_testing::notes::every_constant_is_catalogued_exactly_once(
        include_str!("../../untwist-deobfuscate/src/layers/notes.rs"),
        &[LAYER_NOTES],
    );
    untwist_testing::notes::no_note_is_a_substring_of_another(&[LAYER_NOTES]);
}
