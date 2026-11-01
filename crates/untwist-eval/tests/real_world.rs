mod common;

use oxc_ast::ast::Program;
use sha2::{Digest, Sha256};
use untwist_eval::budget::Budget;
use untwist_eval::decode::{DecoderProgram, InterpEvaluator, Session};
use untwist_eval::rotation::RotationMemo;
use untwist_eval::value::Value;

const GAME_ARRAY_FN: &str = "iïîïïíî";
const GAME_DECODER: &str = "iíìîiîï";
const GAME_ROTATOR_STATEMENT: usize = 0;
const GAME_ARRAY_LENGTH: usize = 21_916;
const GAME_ROTATION: usize = 142;
const GAME_ROTATED_UNITS: usize = 1_113_759;
const GAME_ROTATED_SHA256: &str =
    "97fe4890c1e55f8f42b0809308ff7dd69386053c6c20b9dc864a62d1522e2f37";
const GAME_DECODED_CALL_SITES: usize = 41_485;
const GAME_BUDGET: u64 = 2_000_000;

fn rotated_array_digest(elements: &[Value<'_>]) -> (usize, String) {
    let mut hasher = Sha256::new();
    let mut units = 0usize;
    for element in elements {
        let Value::Str(text) = element else {
            panic!("string array element is not a string: {element:?}");
        };
        hasher.update((text.len() as u32).to_le_bytes());
        for unit in text.units() {
            hasher.update(unit.to_le_bytes());
        }
        units += text.len();
    }
    (units, format!("{:x}", hasher.finalize()))
}

fn game_session<'a>(
    evaluator: &mut InterpEvaluator,
    program: &'a Program<'a>,
) -> (DecoderProgram<'a>, Session<'a>) {
    let decoder_program = common::decoder_program(
        program,
        GAME_ARRAY_FN,
        &[GAME_DECODER],
        Some(GAME_ROTATOR_STATEMENT),
    );
    let session = evaluator
        .instantiate(&decoder_program)
        .expect("game.js: instantiation must succeed");
    (decoder_program, session)
}

#[test]
fn game_js_instantiates_and_rotates_its_string_array_exactly_as_node_does() {
    let Some(source) = common::real_world_fixture("game.js") else {
        return;
    };
    common::with_program(&source, |program| {
        let mut evaluator = InterpEvaluator::new(Budget::new(GAME_BUDGET));
        let (decoder_program, mut session) = game_session(&mut evaluator, program);
        assert_eq!(session.array_len(), GAME_ARRAY_LENGTH);
        let found = session
            .apply_rotation(&decoder_program)
            .expect("game.js: the rotation search must complete");
        assert_eq!(found, Some(GAME_ROTATION));
        let digest = rotated_array_digest(&session.elements());
        assert_eq!(
            digest,
            (GAME_ROTATED_UNITS, GAME_ROTATED_SHA256.to_string())
        );
    });
}

#[test]
fn a_memoised_rotation_reproduces_game_js_rotation_and_its_digest_byte_for_byte() {
    let Some(source) = common::real_world_fixture("game.js") else {
        return;
    };
    common::with_program(&source, |program| {
        let mut memo = RotationMemo::default();
        let mut evaluator = InterpEvaluator::new(Budget::new(GAME_BUDGET));
        let (decoder_program, mut session) = game_session(&mut evaluator, program);
        let searched = session
            .apply_rotation_with(&decoder_program, &mut memo)
            .expect("game.js: the rotation search must complete");
        let searched_digest = rotated_array_digest(&session.elements());

        let mut evaluator = InterpEvaluator::new(Budget::new(GAME_BUDGET));
        let (decoder_program, mut session) = game_session(&mut evaluator, program);
        let memoised = session
            .apply_rotation_with(&decoder_program, &mut memo)
            .expect("game.js: the memoised rotation must complete");
        let memoised_digest = rotated_array_digest(&session.elements());

        assert_eq!(
            memoised_digest,
            (GAME_ROTATED_UNITS, GAME_ROTATED_SHA256.to_string())
        );
        assert_eq!(memoised_digest, searched_digest);
        assert_eq!(searched, Some(GAME_ROTATION));
        assert_eq!(memoised, searched);
        assert_eq!((memo.searches(), memo.hits(), memo.stored()), (1, 1, 1));
    });
}

#[test]
fn game_js_decodes_every_literal_decoder_call_site() {
    let Some(source) = common::real_world_fixture("game.js") else {
        return;
    };
    common::with_program(&source, |program| {
        let calls = common::decoder_calls_anywhere(program, &[GAME_DECODER]);
        assert_eq!(calls.len(), GAME_DECODED_CALL_SITES);
        let mut evaluator = InterpEvaluator::new(Budget::new(GAME_BUDGET));
        let (decoder_program, mut session) = game_session(&mut evaluator, program);
        session
            .apply_rotation(&decoder_program)
            .expect("game.js: the rotation search must complete");
        let mut values = Vec::with_capacity(calls.len());
        for call in &calls {
            values.push(
                session
                    .eval_call(call)
                    .expect("game.js: decode call must not bail"),
            );
        }
        assert!(
            values.iter().all(|value| matches!(value, Value::Str(_))),
            "not every call site decoded to a string"
        );
        for note in session.notes() {
            assert!(!note.contains("threw"), "decode note: {note}");
        }
    });
}
