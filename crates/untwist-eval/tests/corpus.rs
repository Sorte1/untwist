mod common;

use std::path::PathBuf;

use untwist_eval::decode::{all_decoded, Decoded, Evaluator, InterpEvaluator};

#[derive(serde::Deserialize)]
struct Case {
    file: String,
    array_fn: String,
    decoders: Vec<String>,
    rotator_index: Option<usize>,
    expected: Vec<String>,
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("fixtures/eval"))
        .unwrap_or_default()
}

#[test]
fn every_generated_case_decodes_to_its_known_plaintext() {
    let manifest = std::fs::read_to_string(corpus_dir().join("manifest.json"))
        .expect("run `cargo xtask corpus` to generate fixtures/eval");
    let cases: Vec<Case> = serde_json::from_str(&manifest).expect("manifest parses");
    assert_eq!(cases.len(), 36, "corpus size changed; regenerate or update");
    for case in &cases {
        let source = std::fs::read_to_string(corpus_dir().join(&case.file))
            .unwrap_or_else(|_| panic!("missing corpus file {}", case.file));
        common::with_program(&source, |program| {
            let decoders: Vec<&str> = case.decoders.iter().map(String::as_str).collect();
            let decoder_program =
                common::decoder_program(program, &case.array_fn, &decoders, case.rotator_index);
            let calls = common::calls_for(program, 0);
            let mut evaluator = InterpEvaluator::default();
            let values = evaluator
                .decode(&decoder_program, &calls)
                .unwrap_or_else(|bail| panic!("{} bailed: {bail}", case.file));
            assert!(all_decoded(&values), "{} did not fully decode", case.file);
            let rendered: Vec<String> = values
                .iter()
                .map(|value| match value {
                    Decoded::Str(text) => text.to_lossy_string(),
                    other => format!("{other:?}"),
                })
                .collect();
            assert_eq!(rendered, case.expected, "{} decoded wrongly", case.file);
        });
    }
}
