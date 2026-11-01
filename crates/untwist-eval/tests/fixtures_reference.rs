mod common;

use untwist_eval::budget::Budget;
use untwist_eval::decode::{Decoded, Evaluator, InterpEvaluator};

struct Fixture {
    file: &'static str,
    array_fn: &'static str,
    decoders: &'static [&'static str],
    rotator_index: Option<usize>,
    must_contain: &'static [&'static str],
}

const FIXTURES: [Fixture; 5] = [
    Fixture {
        file: "obfuscator.io-string-array-undefined.js",
        array_fn: "_0x3bd1",
        decoders: &["_0x2ca7"],
        rotator_index: None,
        must_contain: &["log"],
    },
    Fixture {
        file: "obfuscator.io.js",
        array_fn: "c",
        decoders: &["d"],
        rotator_index: Some(0),
        must_contain: &["log", "Hello World!"],
    },
    Fixture {
        file: "obfuscator.io-var-functions.js",
        array_fn: "c",
        decoders: &["d"],
        rotator_index: Some(0),
        must_contain: &["log", "Hello World!"],
    },
    Fixture {
        file: "obfuscator.io-rotator-unary.js",
        array_fn: "a",
        decoders: &["b"],
        rotator_index: Some(0),
        must_contain: &["log"],
    },
    Fixture {
        file: "obfuscator.io-multi-encoders.js",
        array_fn: "a",
        decoders: &["b", "c", "d"],
        rotator_index: Some(0),
        must_contain: &["log"],
    },
];

fn decoded_strings(fixture: &Fixture) -> Vec<String> {
    let source = common::read_fixture(fixture.file);
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(
            program,
            fixture.array_fn,
            fixture.decoders,
            fixture.rotator_index,
        );
        let calls = common::decoder_calls_anywhere(program, fixture.decoders);
        let mut evaluator = InterpEvaluator::default();
        let values = evaluator
            .decode(&decoder_program, &calls)
            .unwrap_or_else(|bail| panic!("{}: decode bailed: {bail}", fixture.file));
        values
            .iter()
            .filter_map(|value| match value {
                Decoded::Str(text) => Some(text.to_lossy_string()),
                _ => None,
            })
            .collect()
    })
}

#[test]
fn the_in_scope_reference_fixtures_decode_their_known_plaintext() {
    for fixture in &FIXTURES {
        let decoded = decoded_strings(fixture);
        for needle in fixture.must_contain {
            assert!(
                decoded.iter().any(|value| value == needle),
                "{} did not decode {needle:?}; got {decoded:?}",
                fixture.file
            );
        }
    }
}

const HIGH_PRESET_ROTATION_INDEX: usize = 338;

#[test]
fn the_high_preset_rotation_is_found_expected_partial_pending_2b_wrapper_inlining() {
    let source = common::read_fixture("obfuscator.io-high.js");
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "_0x4d6b", &["_0x4c3e"], Some(0));
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let mut session = evaluator
            .instantiate(&decoder_program)
            .expect("obfuscator.io-high.js: instantiation must succeed");
        let found = session
            .apply_rotation(&decoder_program)
            .expect("obfuscator.io-high.js: the rotation search must complete");
        assert_eq!(
            found,
            Some(HIGH_PRESET_ROTATION_INDEX),
            "expected-partial: obfuscator.io-high.js's rotation must be found at index {HIGH_PRESET_ROTATION_INDEX}; \
             its real call sites route through decoder wrappers, which the \
             inline-decoder-wrappers pass handles, so no decoded plaintext is asserted here"
        );
    });
}
