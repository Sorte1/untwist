use std::path::{Path, PathBuf};

use untwist::source_map::{self, OwnedSourceMap, Token};
use untwist::{Options, Output, Pipeline};
use untwist_core::pass::NoteKind;
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{node_or_skip, printed};

const SECOND_ROUND: &str = "../../fixtures/obfuscator-versions/0.18.8--plain-norot-t75-self--wp4-module--uglify--control-1.js";

const ASSIGNED_TWICE: &str =
    "../../fixtures/obfuscator-versions/4.2.2--rc4-rot-noshuf-shift-inum-w2f5-nocalls-t1--iife--uglify--classes-1.js";

const DECODED_UNDER_UNSAFE: &[&str] = &[
    "cfo-dead-branch",
    "stateful-outside-calls",
    "decoder-not-callee",
    "alias-before-declaration",
    "dp-else-if-controller",
    "dp-controller-blocks",
    "dp-nested-var-object",
    "dp-late-object",
    "wrapper-dead-write",
];

const FINGERPRINTS: &[&str] = &[
    "(((.+)+)+)+$",
    "debu",
    "gger",
    "return (function() ",
    "\"exception\"",
    "fromCharCode",
    "shift()",
];

fn data() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/obfuscator_sweep")
}

fn read(name: &str) -> String {
    std::fs::read_to_string(data().join(name)).unwrap()
}

fn fixture(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

fn traced(source: &str, unsafe_transforms: bool) -> Output {
    let options = Options {
        unsafe_transforms,
        source_map: true,
        decoded_strings: true,
        statement_spans: true,
        ..Options::default()
    };
    Pipeline::new(options).run(source)
}

fn default_code(source: &str) -> String {
    Pipeline::new(Options::default())
        .run(source)
        .code()
        .to_string()
}

fn prints_the_same(source: &str, output: &str) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let expected = printed(&node, dir.path(), source);
    let actual = printed(&node, dir.path(), output);
    assert!(
        expected.is_ok() && expected == actual,
        "{expected:?} vs {actual:?}"
    );
}

fn unsafe_lane(source: &str) -> String {
    let options = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn dead_code_and_control_flow_cases_decode_fully_under_unsafe() {
    for label in DECODED_UNDER_UNSAFE {
        let output = unsafe_lane(&read(&format!("{label}.js")));
        let left: Vec<&&str> = FINGERPRINTS
            .iter()
            .filter(|print| output.contains(**print))
            .collect();
        assert!(left.is_empty(), "{label} keeps {left:?}:\n{output}");
    }
}

#[test]
fn dead_code_and_control_flow_cases_print_the_source_output_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for label in DECODED_UNDER_UNSAFE {
        let source = read(&format!("{label}.source.js"));
        let output = unsafe_lane(&read(&format!("{label}.js")));
        let expected = printed(&node, dir.path(), &source);
        let actual = printed(&node, dir.path(), &output);
        assert!(
            expected.is_ok() && expected == actual,
            "{label}: {expected:?} vs {actual:?}"
        );
    }
}

#[test]
fn a_self_defending_block_a_later_layer_put_in_a_control_flow_function_is_removed() {
    let output = unsafe_lane(&read("sd-controller-survives.js"));
    assert!(!output.contains("_0x1977fa"), "{output}");
    assert!(!output.contains("_0x59eeab"), "{output}");
    assert!(!output.contains("setInterval"), "{output}");
}

#[test]
fn a_decoder_reached_only_from_a_branch_the_decoded_strings_prove_dead_is_decoded_under_unsafe() {
    let output = unsafe_lane(&read("guarded-escape-dead.js"));
    assert!(output.contains("\"hello\""), "{output}");
    assert!(!output.contains("TyxkK"), "{output}");
}

#[test]
fn a_decoder_reached_from_a_live_guarded_branch_keeps_its_unit_under_unsafe() {
    let output = unsafe_lane(&read("guarded-escape-live.js"));
    assert!(output.contains("typeof d"), "{output}");
    assert!(output.contains("TyxkK"), "{output}");
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let source = read("guarded-escape-live.js");
    let expected = printed(&node, dir.path(), &source);
    let actual = printed(&node, dir.path(), &output);
    assert!(
        expected.is_ok() && expected == actual,
        "{expected:?} vs {actual:?}"
    );
}

#[test]
fn a_later_round_keeps_the_reach_reason_of_the_layer_round_one_decoded() {
    let output = Pipeline::new(Options::default()).run(&read("round-two-reach.js"));
    let reach = output
        .notes
        .iter()
        .any(|note| note.message == notes::STRING_ARRAY_GLOBAL_REACH);
    let kept = output.notes.iter().find(|note| {
        note.kind == NoteKind::Refusal && note.message.starts_with("decoder machinery kept: ")
    });
    assert!(reach, "{:#?}", output.notes);
    let kept = kept.expect("a kept-machinery note");
    assert!(
        kept.message.contains("may hold the global object"),
        "{}",
        kept.message
    );
}

fn utf16_len(line: &str) -> u32 {
    u32::try_from(line.encode_utf16().count()).unwrap_or(u32::MAX)
}

fn mapped_tokens(map: &OwnedSourceMap, source: &str) -> Vec<Token> {
    let lines: Vec<&str> = source.split('\n').collect();
    let tokens: Vec<Token> = map
        .get_tokens()
        .filter(|token| token.get_source_id().is_some())
        .collect();
    for token in &tokens {
        let line = lines[token.get_src_line() as usize];
        assert!(
            token.get_src_col() <= utf16_len(line),
            "source column in range"
        );
    }
    tokens
}

fn origin_of(map: &OwnedSourceMap, code: &str, needle: &str) -> Option<(u32, u32)> {
    let at = code.find(needle)?;
    let line = code[..at].matches('\n').count();
    let start = code[..at].rfind('\n').map_or(0, |found| found + 1);
    let column = utf16_len(&code[start..at]);
    let line = u32::try_from(line).ok()?;
    let table = map.generate_lookup_table();
    let token = map.lookup_token(&table, line, column)?;
    Some((token.get_src_line(), token.get_src_col()))
}

fn source_text_at(source: &str, (line, column): (u32, u32)) -> String {
    let line = source.split('\n').nth(line as usize).unwrap_or_default();
    line.chars().skip(column as usize).collect()
}

#[test]
fn a_second_round_keeps_a_source_map_into_the_input_in_both_lanes() {
    let source = fixture(SECOND_ROUND);
    for unsafe_transforms in [false, true] {
        let output = traced(&source, unsafe_transforms);
        let map = output.source_maps[0].as_ref().expect("a map");
        let map = source_map::parse(&map.to_json_string()).expect("the map parses back");
        assert_eq!(map.get_source_content(0), Some(source.as_str()));
        let tokens = mapped_tokens(&map, &source);
        assert!(tokens.len() > 100, "{} mappings", tokens.len());
        let origin = origin_of(&map, output.code(), "console.log(").expect("a mapped call");
        assert!(
            source_text_at(&source, origin).starts_with("console.log("),
            "{:?}",
            source_text_at(&source, origin)
        );
    }
}

#[test]
fn a_second_round_maps_statement_spans_and_decoded_strings_into_the_input() {
    let source = fixture(SECOND_ROUND);
    let output = traced(&source, false);
    let spans = output.statement_spans.expect("statement spans");
    assert!(!spans.is_empty());
    for (start, end) in spans {
        assert!(
            start < end && end as usize <= source.len(),
            "{start}..{end}"
        );
    }
    let sited: Vec<(u32, u32)> = output
        .decoded_strings
        .iter()
        .filter_map(|site| site.span)
        .collect();
    assert!(sited.len() > 10, "{} sited strings", sited.len());
    for (start, end) in sited {
        let text = &source[start as usize..end as usize];
        assert!(text.contains('('), "{text:?}");
    }
}

#[test]
fn a_wrapper_call_assigning_one_variable_twice_keeps_each_argument_under_unsafe() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for source in [
        read("wrapper-arguments-assign-twice.js"),
        fixture(ASSIGNED_TWICE),
    ] {
        let output = unsafe_lane(&source);
        let expected = printed(&node, dir.path(), &source);
        let actual = printed(&node, dir.path(), &output);
        assert!(
            expected.is_ok() && expected == actual,
            "{expected:?} vs {actual:?}\n{output}"
        );
    }
}

const PATCH_SCAN: &[&str] = &[
    "a03", "a04", "a07", "a20", "b21", "c03", "c04", "c05", "c06", "c13", "c17", "d01", "d03",
    "d04", "d13", "n02", "n19", "n20",
];

#[test]
fn a_built_in_patched_through_a_shape_the_scan_skipped_prints_like_node_in_the_default_lane() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let wrong: Vec<String> = PATCH_SCAN
        .iter()
        .filter_map(|label| {
            let input = read(&format!("scan-{label}.js"));
            let output = Pipeline::new(Options::default())
                .run(&input)
                .code()
                .to_string();
            let expected = printed(&node, dir.path(), &input);
            let actual = printed(&node, dir.path(), &output);
            (expected.is_err() || expected != actual)
                .then(|| format!("{label}: {expected:?} vs {actual:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

fn default_lane(source: &str) -> untwist::Output {
    Pipeline::new(Options::default()).run(source)
}

fn has_note(output: &untwist::Output, message: &str) -> bool {
    output.notes.iter().any(|note| note.message == message)
}

#[test]
fn a_console_disable_is_kept_with_a_debug_protection_timer_the_default_lane_keeps() {
    let output = default_lane(&read("console-beside-timer.js"));
    let code = output.code();
    assert!(code.contains("setInterval"), "{code}");
    assert!(code.contains("\"exception\""), "{code}");
    assert!(
        has_note(
            &output,
            untwist_deobfuscate::notes::PROTECTIONS_KEPT_TOGETHER
        ),
        "{:?}",
        output.notes
    );
    let decoded = unsafe_lane(&read("console-beside-timer.js"));
    assert!(!decoded.contains("setInterval"), "{decoded}");
    assert!(!decoded.contains("\"exception\""), "{decoded}");
}

#[test]
fn a_dead_debug_protection_a_global_listing_may_reach_is_kept_with_a_premise_note() {
    let output = default_lane(&read("dead-protections-listed.js"));
    assert!(
        has_note(
            &output,
            untwist_deobfuscate::notes::DEBUG_PROTECTION_GLOBAL_REACH
        ),
        "{:?}",
        output.notes
    );
}

#[test]
fn a_decoder_used_as_a_value_in_a_branch_that_runs_keeps_its_unit_under_unsafe() {
    let source = read("branch-escape-live.js");
    let output = unsafe_lane(&source);
    assert!(output.contains("map(d)"), "{output}");
    prints_the_same(&source, &output);
}

#[test]
fn fresh_instances_handed_to_a_printer_do_not_keep_a_layered_unit() {
    let source = read("fresh-instance-printed.js");
    let output = default_code(&source);
    assert!(!output.contains("_0xa"), "{output}");
    assert!(!output.contains("_0xc"), "{output}");
    prints_the_same(&source, &output);
}

#[test]
fn an_outer_string_array_with_one_decoded_entry_is_decoded_in_the_default_lane() {
    let output = default_code(&read("one-entry-outer-array.js"));
    assert!(!output.contains("_0x5930"), "{output}");
    assert!(!output.contains("_0x3980"), "{output}");
    prints_the_same(&read("one-entry-outer-array.source.js"), &output);
}

const NESTED_OBJECT_HAZARDS: &[&str] = &[
    "nested-this-write",
    "nested-this-returned",
    "nested-this-arrow",
    "nested-inherited-valueof",
    "nested-inherited-definegetter",
    "nested-async-property",
    "nested-generator-property",
    "nested-proto-string-key",
    "nested-proto-identifier-key",
];

#[test]
fn a_nested_object_that_can_change_or_is_not_plain_keeps_the_branch_in_both_lanes() {
    for label in NESTED_OBJECT_HAZARDS {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(output.contains("\"changed\""), "{label}:\n{output}");
        }
    }
}

#[test]
fn a_nested_object_that_can_change_or_is_not_plain_prints_what_node_prints_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for label in NESTED_OBJECT_HAZARDS {
        let source = read(&format!("{label}.js"));
        let expected = printed(&node, dir.path(), &source);
        for output in [default_code(&source), unsafe_lane(&source)] {
            let actual = printed(&node, dir.path(), &output);
            assert!(
                expected.is_ok() && expected == actual,
                "{label}: {expected:?} vs {actual:?}"
            );
        }
    }
}

#[test]
fn a_plain_read_only_nested_object_still_decides_the_branch_in_both_lanes() {
    let source = read("nested-plain-decided.js");
    for output in [default_code(&source), unsafe_lane(&source)] {
        assert!(!output.contains("\"changed\""), "{output}");
    }
}
