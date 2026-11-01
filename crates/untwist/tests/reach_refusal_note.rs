use untwist::{Options, Output, Pipeline};
use untwist_core::pass::NoteKind;
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::patched_after_rotator;

const KEPT_PREFIX: &str = "decoder machinery kept: ";

fn run(source: &str, unsafe_transforms: bool) -> Output {
    Pipeline::new(Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn reached_by_key() -> String {
    patched_after_rotator(
        "true",
        "var g = this; function put(k, v) { g[k] = v; } put(String.fromCharCode(120), 1);",
    )
}

fn kept_count(message: &str) -> Option<u64> {
    let rest = message.strip_prefix(KEPT_PREFIX)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let tail = rest.get(digits.len()..)?;
    tail.starts_with(notes::MACHINERY_REFERENCES_KEPT)
        .then(|| digits.parse().ok())
        .flatten()
}

fn cluster(out: &Output) -> Vec<(&'static str, NoteKind, &str)> {
    out.notes
        .iter()
        .filter(|note| {
            note.message == notes::STRING_ARRAY_GLOBAL_REACH
                || note.message.starts_with(notes::WEBPACK_BOOTSTRAP_UNPROVEN)
                || kept_count(&note.message).is_some()
        })
        .map(|note| (note.pass, note.kind, note.message.as_str()))
        .collect()
}

#[test]
fn a_unit_refused_for_global_reach_reports_one_warning_with_the_largest_count() {
    let out = run(&reached_by_key(), false);
    let notes = cluster(&out);
    let warnings: Vec<_> = notes
        .iter()
        .filter(|(_, kind, _)| *kind == NoteKind::Refusal)
        .collect();
    assert_eq!(warnings.len(), 1, "{notes:#?}");
    let (pass, _, message) = warnings[0];
    assert_eq!(*pass, "remove-decoder-machinery", "{notes:#?}");
    let largest = notes
        .iter()
        .filter_map(|(_, _, message)| kept_count(message))
        .max();
    assert_eq!(kept_count(message), largest, "{notes:#?}");
    assert!(
        message.contains("by a key that may be its name"),
        "{message}"
    );
}

#[test]
fn the_details_stay_in_the_report_as_info() {
    let out = run(&reached_by_key(), false);
    let notes = cluster(&out);
    let reach = notes
        .iter()
        .find(|(_, _, message)| *message == notes::STRING_ARRAY_GLOBAL_REACH);
    assert!(
        matches!(reach, Some((_, NoteKind::Progress, _))),
        "{notes:#?}"
    );
}

#[test]
fn a_decoded_unit_is_not_touched() {
    let out = run(&reached_by_key(), true);
    assert!(cluster(&out).is_empty(), "{:#?}", out.notes);
}
