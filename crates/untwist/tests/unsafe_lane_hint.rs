use untwist::{NoteKind, Options, Output, Pipeline, UNSAFE_LANE_DECODES};

const HIGH: &str = include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-high.js");
const PLAIN_ARRAY: &str =
    include_str!("../../../fixtures/reference/deobfuscate/obfuscator.io-multi-encoders.js");

fn hints(output: &Output) -> Vec<&str> {
    output
        .notes
        .iter()
        .filter(|note| note.pass == "pipeline" && note.message.ends_with(UNSAFE_LANE_DECODES))
        .map(|note| {
            assert_eq!(note.kind, NoteKind::Refusal);
            note.message.as_str()
        })
        .collect()
}

#[test]
fn a_default_lane_alias_refusal_tells_the_reader_to_try_unsafe() {
    let output = Pipeline::new(Options::default()).run(HIGH);
    let found = hints(&output);
    assert_eq!(found.len(), 1, "{:?}", output.notes);
    let count = found[0]
        .strip_suffix(UNSAFE_LANE_DECODES)
        .and_then(|head| head.trim_end().parse::<u64>().ok())
        .unwrap_or_else(|| panic!("no leading count in {}", found[0]));
    assert!(count > 0);
}

#[test]
fn the_unsafe_lane_gives_no_hint() {
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let output = Pipeline::new(options).run(HIGH);
    assert!(hints(&output).is_empty(), "{:?}", output.notes);
}

#[test]
fn a_program_that_decodes_by_default_gives_no_hint() {
    let output = Pipeline::new(Options::default()).run(PLAIN_ARRAY);
    assert!(hints(&output).is_empty(), "{:?}", output.notes);
}
