use untwist::{Options, Output, Pipeline, TestHooks};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::patched_after_rotator;

const GLOBAL_KEYED_WRITE: &str =
    "var g = this; function put(k, v) { g[k] = v; } put(String.fromCharCode(120), 1);";
const READS_SLICE: &str = "i.slice(0) === i";

fn recording(unsafe_transforms: bool, refusal_spans: bool) -> Pipeline {
    Pipeline::new(Options {
        unsafe_transforms,
        unpack: false,
        jsx: false,
        test_hooks: TestHooks {
            refusal_spans,
            ..TestHooks::default()
        },
        ..Options::default()
    })
}

fn run(source: &str, unsafe_transforms: bool) -> Output {
    recording(unsafe_transforms, true).run(source)
}

fn position_of(source: &str, needle: &str) -> String {
    let offset = source.find(needle).expect("the needle is in the source");
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rfind('\n')
        .map_or(before, |at| &before[at + 1..])
        .chars()
        .count()
        + 1;
    format!("{line}:{column}")
}

fn refused_as(out: &Output, prefix: &str) -> bool {
    out.notes
        .iter()
        .any(|note| note.message.starts_with(prefix))
}

#[test]
fn an_escaped_read_is_reported_beside_the_refusal_it_causes_and_unsafe_decodes_past_it() {
    let source = patched_after_rotator(
        READS_SLICE,
        "const o = {}; export function f(k) { return o[k]; }",
    );
    let at = position_of(&source, "o[k]");
    let out = run(&source, false);
    assert!(refused_as(&out, notes::BUILTIN_PATCHED), "{:#?}", out.notes);
    let spans = &out.refusal_spans;
    assert!(
        spans
            .iter()
            .any(|span| span.starts_with(&format!("{at} escaped read (definer"))),
        "{spans:#?}"
    );
    assert!(
        spans.iter().any(|span| span.contains(" BUILTIN_PATCHED")),
        "{spans:#?}"
    );
    let pristine = run(&source, true);
    assert!(
        !refused_as(&pristine, notes::BUILTIN_PATCHED),
        "{:#?}",
        pristine.notes
    );
    assert!(
        !pristine
            .refusal_spans
            .iter()
            .any(|span| span.contains(" BUILTIN_PATCHED")),
        "{:#?}",
        pristine.refusal_spans
    );
    assert!(pristine.code().contains("\"log!\""), "{}", pristine.code());
}

#[test]
fn every_escaped_read_is_reported_not_only_the_first() {
    let source = patched_after_rotator(
        READS_SLICE,
        "const o = {}; export function f(k) { return o[k]; }\nexport function g(j) { return '' + o[j]; }",
    );
    let out = run(&source, false);
    for needle in ["o[k]", "o[j]"] {
        let at = format!("{} escaped read", position_of(&source, needle));
        assert!(
            out.refusal_spans.iter().any(|span| span.starts_with(&at)),
            "{needle}: {:#?}",
            out.refusal_spans
        );
    }
}

#[test]
fn a_global_reach_refusal_is_reported_at_the_reaching_write() {
    let source = patched_after_rotator("true", GLOBAL_KEYED_WRITE);
    let out = run(&source, false);
    assert!(
        refused_as(&out, notes::STRING_ARRAY_GLOBAL_REACH),
        "{:#?}",
        out.notes
    );
    let expected = format!("{} GLOBAL_REACH", position_of(&source, "g[k]"));
    assert!(
        out.refusal_spans.contains(&expected),
        "{:#?}",
        out.refusal_spans
    );
}

#[test]
fn a_program_that_decodes_reports_no_refusal_spans() {
    let source = patched_after_rotator("true", "var o = {}; o.x = 1;");
    let out = run(&source, false);
    assert!(!out.code().contains("b(0)"), "{}", out.code());
    assert!(out.refusal_spans.is_empty(), "{:#?}", out.refusal_spans);
}

#[test]
fn nothing_is_recorded_unless_the_hook_asks_for_it() {
    let source = patched_after_rotator("true", GLOBAL_KEYED_WRITE);
    let out = recording(false, false).run(&source);
    assert!(
        refused_as(&out, notes::STRING_ARRAY_GLOBAL_REACH),
        "{:#?}",
        out.notes
    );
    assert!(out.refusal_spans.is_empty(), "{:#?}", out.refusal_spans);
}

#[test]
fn several_inputs_name_the_input_each_span_belongs_to() {
    let refused = patched_after_rotator("true", GLOBAL_KEYED_WRITE);
    let decoded = patched_after_rotator("true", "");
    let out = recording(false, true).run_many(&[&decoded, &refused]);
    assert!(!out.refusal_spans.is_empty());
    assert!(
        out.refusal_spans
            .iter()
            .all(|span| span.starts_with("input 1: ")),
        "{:#?}",
        out.refusal_spans
    );
}
