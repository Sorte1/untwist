use std::path::{Path, PathBuf};

use untwist::source_map::{self, OwnedSourceMap, Token};
use untwist::{Options, Output, Pipeline, RenameMode};
use untwist_core::pass::NoteKind;
use untwist_deobfuscate::notes;
use untwist_testing::node::run;
use untwist_testing::node_rows::{node_or_skip, observed, printed};

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
    "dp-object-alias",
    "dp-nested-dead-uses",
    "cfo-alias-console",
    "round-unlocked-console",
    "braced-controller-lock",
    "wrapper-in-inner-decoder",
    "strict-block-wrapper",
    "block-wrapper-dead-write",
    "cfo-escape-in-branch",
    "interval-before-rotator",
    "decoded-foreign-rotator",
    "runners-in-wrapper",
    "flattened-controller-rounds",
    "dp-redeclared-var",
    "dp-key-object-assigned",
    "wrapper-emptied-shell",
    "alias-dead-write",
    "cfo-forwarder-arg",
    "escape-name-reused",
    "runners-in-nested-controller",
    "proxied-source-check",
    "pw011m3-class-proxy-gate",
    "pw011m3-class-dead-wrapper",
    "pw011m3-closures",
    "pw011m3-greeting",
    "pw011m3-control",
    "pw011m3-mixed",
    "pw011m3-generators",
    "pw011m3-async",
];

const PW011_M3: &[&str] = &[
    "pw011m3-class-proxy-gate",
    "pw011m3-class-dead-wrapper",
    "pw011m3-closures",
    "pw011m3-greeting",
    "pw011m3-control",
    "pw011m3-mixed",
    "pw011m3-generators",
    "pw011m3-async",
    "decoder-call-throws",
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
fn writes_in_branches_only_decoding_proves_dead_print_like_node_by_default() {
    for label in ["alias-dead-write", "cfo-forwarder-arg"] {
        let source = read(&format!("{label}.js"));
        prints_the_same(&source, &default_code(&source));
    }
}

#[test]
fn decoders_a_proxy_or_a_removed_wrapper_held_back_print_like_node_by_default() {
    for label in PW011_M3 {
        let source = read(&format!("{label}.js"));
        prints_the_same(&source, &default_code(&source));
    }
}

#[test]
fn a_control_flow_object_that_escapes_in_a_live_branch_prints_like_node_under_unsafe() {
    let source = read("cfo-live-escape.js");
    let output = unsafe_lane(&source);
    assert!(output.contains("[f].length"), "{output}");
    prints_the_same(&source, &output);
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
    let kept_protection = keeps_unproven_built_ins(&output);
    let reach = output
        .notes
        .iter()
        .any(|note| note.message == notes::STRING_ARRAY_GLOBAL_REACH);
    let kept = output.notes.iter().find(|note| {
        note.kind == NoteKind::Refusal && note.message.starts_with("decoder machinery kept: ")
    });
    let kept = kept.expect("a kept-machinery note");
    assert!(kept_protection || reach, "{:#?}", output.notes);
    assert!(
        kept_protection || kept.message.contains("may hold the global object"),
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
fn a_second_round_keeps_a_source_map_into_the_input_under_unsafe() {
    let source = fixture(SECOND_ROUND);
    {
        let output = traced(&source, true);
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
    if output.code() == source {
        assert!(
            output
                .notes
                .iter()
                .any(|note| note.message.starts_with(untwist::SOURCE_READER_UNGUARDED)),
            "{:?}",
            output.notes
        );
        return;
    }
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

fn keeps_unproven_built_ins(output: &untwist::Output) -> bool {
    has_note(
        output,
        untwist_deobfuscate::notes::SELF_DEFENDING_BUILTIN_PATCHED,
    )
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
        ) || keeps_unproven_built_ins(&output),
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
        ) || keeps_unproven_built_ins(&output),
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

#[test]
fn a_nested_object_whose_prototype_the_program_may_patch_keeps_the_branch_in_the_default_lane() {
    let source = read("nested-patched-prototype.js");
    let output = default_code(&source);
    assert!(output.contains("\"changed\""), "{output}");
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let expected = printed(&node, dir.path(), &source);
    let actual = printed(&node, dir.path(), &output);
    assert!(
        expected.is_ok() && expected == actual,
        "{expected:?} vs {actual:?}"
    );
}

#[test]
fn a_rotator_holding_a_call_controller_is_kept_with_a_premise_note_in_the_default_lane() {
    let output = default_lane(&read("rotator-holds-controller.js"));
    assert!(
        has_note(
            &output,
            untwist_deobfuscate::notes::ROTATOR_HOLDS_PROTECTION
        ),
        "{:?}",
        output.notes
    );
}

#[test]
fn a_call_of_a_key_no_object_has_in_a_dead_branch_still_decides_it_in_both_lanes() {
    let source = read("nested-absent-key-call.js");
    for output in [unsafe_lane(&source), default_code(&source)] {
        assert!(!output.contains("missing"), "{output}");
        prints_the_same(&source, &output);
    }
}

const NESTED_HAZARDS_BOTH_LANES: &[&str] = &[
    "r5-f1",
    "r5-this-factory",
    "r5-this-member",
    "r5-this-param",
    "r5-this-builtin",
    "r5-valueof-own",
    "r5-this-delete",
    "r5-tagged",
    "r5-this-after-test",
    "r5-chain-three",
    "r5-this-ternary",
    "r5-f3",
    "r5-annexb-if",
    "r5-annexb-switch",
    "r5-annexb-nested-call",
];

const NESTED_HAZARDS_PATCHED_BUILTINS: &[&str] = &[
    "r5-f2a",
    "r5-f2b",
    "r5-owner-member-undef",
    "r5-b-math",
    "r5-b-parseint",
    "r5-b-string-fn",
    "r5-b-fromcc",
    "r5-b-json",
    "r5-b-atob",
    "r5-b-escape",
    "r5-b-alias-patch",
    "r5-body-valueof",
    "r5-body-proto-read",
    "r5-t-typeof",
    "r5-t-cond",
    "r5-t-idx",
    "r5-t-num-member",
    "r5-t-member-undef",
    "r5-v-typeof",
    "r5-v-member",
];

fn prints_like_node_in(labels: &[&str], lanes: &[fn(&str) -> String]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for label in labels {
        let source = read(&format!("{label}.js"));
        let expected = printed(&node, dir.path(), &source);
        for lane in lanes {
            let actual = printed(&node, dir.path(), &lane(&source));
            if expected.is_err() || expected != actual {
                wrong.push(format!("{label}: {expected:?} vs {actual:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_nested_object_a_call_or_a_block_function_can_change_prints_like_node_in_both_lanes() {
    prints_like_node_in(NESTED_HAZARDS_BOTH_LANES, &[default_code, unsafe_lane]);
}

#[test]
fn a_fold_through_a_built_in_the_program_patches_prints_like_node_in_the_default_lane() {
    prints_like_node_in(NESTED_HAZARDS_PATCHED_BUILTINS, &[default_code]);
}

#[test]
fn a_domain_lock_registered_through_a_braced_call_controller_is_removed_under_unsafe() {
    let output = unsafe_lane(&read("braced-controller-lock.js"));
    assert!(!output.contains("RegExp"), "{output}");
    assert!(!output.contains(".apply("), "{output}");
}

#[test]
fn an_uglify_inlined_rotator_decodes_under_unsafe() {
    let source = fixture(
        "../../fixtures/obfuscator-versions/4.2.2--rc4-rot-noshuf-shift-inum-w2f5-nocalls-t1--iife--uglify--classes-1.js",
    );
    let output = unsafe_lane(&source);
    assert!(!output.contains("shift()"), "{output}");
    prints_the_same(&source, &output);
}

const ALIAS_BLOCK_FUNCTIONS: &[&str] = &[
    "alias-annexb-lit-blockfn-i",
    "alias-annexb-lit-blockfn-h",
    "alias-annexb-lit-iffn-i",
    "alias-annexb-lit-iffn-h",
    "alias-annexb-lit-switchfn-i",
    "alias-annexb-lit-tryfn-i",
    "alias-annexb-lit-catchfn-h",
    "alias-annexb-lit-loopfn-i",
    "alias-annexb-lit-labelfn-i",
    "alias-annexb-lit-deepfn-i",
    "alias-annexb-key-blockfn-i",
    "alias-annexb-key-blockfn-h",
    "alias-annexb-key-iffn-i",
    "alias-annexb-key-iffn-h",
    "alias-annexb-key-switchfn-i",
    "alias-annexb-key-tryfn-i",
    "alias-annexb-key-catchfn-h",
    "alias-annexb-key-loopfn-i",
    "alias-annexb-key-labelfn-i",
    "alias-annexb-key-deepfn-i",
    "alias-annexb-f1a",
    "alias-annexb-f1b",
    "alias-annexb-f1c",
    "alias-annexb-f1d",
    "alias-annexb-f2",
];

#[test]
fn an_object_alias_a_block_function_or_an_early_read_can_reach_prints_like_node_in_both_lanes() {
    prints_like_node_in(ALIAS_BLOCK_FUNCTIONS, &[default_code, unsafe_lane]);
}

const FOLDED_NAME_HAZARDS_BOTH_LANES: &[&str] = &[
    "r6-f1",
    "r6-f1b",
    "r6-f2",
    "r6-name-Infinity",
    "r6-name-isNaN",
    "r6-name-Math",
    "r6-name-NaN",
    "r6-name-Number",
    "r6-name-param-same",
    "r6-name-parseInt",
    "r6-name-String2",
    "r6-name-String",
    "r6-name-typeof",
    "r6-name-undefined0",
    "r6-name-undefined",
    "r6-with-wrap1",
    "r6-with-wrap2",
    "r6-with-wrap3",
    "r6-with-wrap4",
    "r6-with-wrap5",
];

const FOLDED_NAME_HAZARDS_UNDER_EVAL: &[&str] = &[
    "r6-f2b",
    "r6-eval-after1",
    "r6-eval-after2",
    "r6-eval-after3",
    "r6-eval-after4",
    "r6-eval-after5",
    "r6-eval-arrow1",
    "r6-eval-arrow2",
    "r6-eval-arrow3",
    "r6-eval-arrow4",
    "r6-eval-arrow5",
    "r6-eval-lit1",
    "r6-eval-lit2",
    "r6-eval-lit3",
    "r6-eval-lit4",
    "r6-eval-lit5",
    "r6-eval-wrap1",
    "r6-eval-wrap2",
    "r6-eval-wrap3",
    "r6-eval-wrap4",
    "r6-eval-wrap5",
];

#[test]
fn a_body_that_reads_its_own_name_or_a_with_binding_prints_like_node_in_both_lanes() {
    prints_like_node_in(FOLDED_NAME_HAZARDS_BOTH_LANES, &[default_code, unsafe_lane]);
}

#[test]
fn a_body_under_a_direct_eval_that_declares_its_global_prints_like_node_in_the_default_lane() {
    prints_like_node_in(FOLDED_NAME_HAZARDS_UNDER_EVAL, &[default_code]);
}

const ASSEMBLED_BASE_AND_OPTIONAL_CALLS: &[&str] = &[
    "cfo-r6-a6",
    "cfo-r6-a7",
    "cfo-r6-f1a",
    "cfo-r6-f1b",
    "cfo-r6-f1c",
    "cfo-r6-f1-computed",
    "cfo-r6-f1-cond",
    "cfo-r6-f1-fn",
    "cfo-r6-f1-halfrun2",
    "cfo-r6-f1-halfrun",
    "cfo-r6-f1-order-v-first",
    "cfo-r6-f1-twoalias",
    "cfo-r6-f2a",
    "cfo-r6-f2-asm-del",
    "cfo-r6-f2-asm-delp",
    "cfo-r6-f2-asm-optcall",
    "cfo-r6-f2-asm-optc",
    "cfo-r6-f2-asm-opt",
    "cfo-r6-f2-asm-optn",
    "cfo-r6-f2-asm-optp",
    "cfo-r6-f2-asm-optt",
    "cfo-r6-f2-asm-paren",
    "cfo-r6-f2-asm-plain",
    "cfo-r6-f2-asm-ptag",
    "cfo-r6-f2-asm-seq",
    "cfo-r6-f2-asm-tag",
    "cfo-r6-f2b",
    "cfo-r6-f2c",
    "cfo-r6-f2-lit-del",
    "cfo-r6-f2-lit-delp",
    "cfo-r6-f2-lit-optcall",
    "cfo-r6-f2-lit-optc",
    "cfo-r6-f2-lit-opt",
    "cfo-r6-f2-lit-optn",
    "cfo-r6-f2-lit-optp",
    "cfo-r6-f2-lit-optt",
    "cfo-r6-f2-lit-paren",
    "cfo-r6-f2-lit-plain",
    "cfo-r6-f2-lit-ptag",
    "cfo-r6-f2-lit-seq",
    "cfo-r6-f2-lit-tag",
    "cfo-r6-h1",
    "cfo-r6-h2",
    "cfo-r6-h3",
    "cfo-r6-h4",
    "cfo-r6-h5",
    "cfo-r6-h6",
    "cfo-r6-h7",
    "cfo-r6-j1",
    "cfo-r6-j2",
    "cfo-r6-j3",
    "cfo-r6-j4",
    "cfo-r6-j5",
    "cfo-r6-j6",
    "cfo-r6-j7",
];

#[test]
fn an_assembled_base_read_or_an_optional_method_call_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        ASSEMBLED_BASE_AND_OPTIONAL_CALLS,
        &[default_code, unsafe_lane],
    );
}

const ASSEMBLED_BASE_CALLS_DURING_THE_RUN: &[&str] = &[
    "cfo-r7-h1a",
    "cfo-r7-h1b",
    "cfo-r7-h1c",
    "cfo-r7-h1d",
    "cfo-r7-h1e",
    "cfo-r7-h1f",
    "cfo-r7-h1g",
    "cfo-r7-h1h",
];

#[test]
fn a_method_call_on_an_assembled_base_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        ASSEMBLED_BASE_CALLS_DURING_THE_RUN,
        &[default_code, unsafe_lane],
    );
}

#[test]
fn a_method_call_on_an_assembled_base_keeps_the_branch_in_both_lanes() {
    for label in ASSEMBLED_BASE_CALLS_DURING_THE_RUN {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(output.contains("\"changed\""), "{label}:\n{output}");
        }
    }
}

const BLOCK_ROTATOR_ESCAPES: &[&str] = &[
    "block-rotator-return",
    "block-rotator-loop-var",
    "block-rotator-checksum-var",
    "block-rotator-for-init-var",
    "block-rotator-loop-function",
    "block-rotator-leading-function",
    "block-rotator-outer-continue",
    "block-rotator-outer-break",
    "block-rotator-loop-await",
    "block-rotator-loop-yield",
    "block-rotator-loop-super",
    "block-rotator-leading-await",
    "block-rotator-leading-yield",
    "block-rotator-leading-super",
    "block-rotator-annexb-var",
    "block-rotator-annexb-function",
    "block-rotator-annexb-top-level",
    "block-rotator-annexb-second",
    "block-rotator-annexb-closure",
    "block-rotator-annexb-for-var",
    "block-rotator-annexb-expression",
    "block-rotator-annexb-checksum",
    "block-rotator-closed",
];

const CALLED_ROTATOR_ANNEX_B: &[&str] = &["called-rotator-annexb"];

const BLOCK_ROTATOR_USING: &[&str] = &[
    "block-rotator-leading-using",
    "block-rotator-leading-await-using",
];

#[test]
fn a_block_rotator_whose_loop_or_declarations_reach_outside_the_block_prints_like_node_in_both_lanes(
) {
    prints_like_node_in(BLOCK_ROTATOR_ESCAPES, &[default_code, unsafe_lane]);
}

#[test]
fn a_block_rotator_whose_loop_or_declarations_reach_outside_the_block_is_not_wrapped_in_both_lanes()
{
    for label in BLOCK_ROTATOR_ESCAPES
        .iter()
        .filter(|label| **label != "block-rotator-closed")
    {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(!output.contains("})();"), "{label}:\n{output}");
        }
    }
}

#[test]
fn a_called_rotator_beside_a_block_function_copied_to_an_outer_binding_prints_like_node_in_both_lanes(
) {
    prints_like_node_in(CALLED_ROTATOR_ANNEX_B, &[default_code, unsafe_lane]);
}

#[test]
fn a_block_rotator_led_by_a_using_declaration_is_not_wrapped_in_both_lanes() {
    for label in BLOCK_ROTATOR_USING {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(!output.contains("})();"), "{label}:\n{output}");
        }
    }
}

#[test]
fn a_closed_block_rotator_is_still_wrapped_in_both_lanes() {
    let source = read("block-rotator-closed.js");
    for output in [default_code(&source), unsafe_lane(&source)] {
        assert!(output.contains("(function () {\n    const w"), "{output}");
    }
}

#[test]
fn protection_runners_at_the_head_of_a_decoder_are_dropped_under_unsafe() {
    let output = unsafe_lane(&read("runner-in-decoder.js"));
    assert_eq!(
        output.trim(),
        "console.log(\"hello, ada\");\nconsole.log(\"done\");",
        "{output}"
    );
    let kept = default_code(&read("runner-in-decoder.js"));
    assert!(kept.contains("_0x2a2695()"), "{kept}");
}

const CONSOLE_KEPT_BY_DEAD_CODE: &[&str] =
    &["console-braced-controller", "console-alias-foreign-key"];

#[test]
fn console_disabling_behind_dead_code_copies_is_removed_in_the_default_lane() {
    for label in CONSOLE_KEPT_BY_DEAD_CODE {
        let source = read(&format!("{label}.js"));
        let output = default_code(&source);
        assert!(!output.contains("\"exception\""), "{label}:\n{output}");
        prints_the_same(&read(&format!("{label}.source.js")), &output);
    }
}

#[test]
fn a_controller_block_whose_name_only_a_discarded_copy_reads_is_merged_in_the_default_lane() {
    let output = Pipeline::new(Options::default()).run(&read("console-dropped-global.js"));
    assert!(!output.code().contains("{{const"), "{}", output.code());
    assert!(keeps_unproven_built_ins(&output), "{:?}", output.notes);
}

const RESPANNED_ESCAPES: &[&str] = &[
    "d3-and_outer-if-dead",
    "d3-and_outer-or-dead",
    "d3-cfo_callee-if-live",
    "d3-cfo_dormant-if-live",
    "d3-cfo_dormant-or-live",
    "d3-cfo_eq-if-live",
    "d3-cfo_eq-or-live",
    "d3-cfo_inarg-if-live",
    "d3-cfo_inarg-or-live",
    "d3-cfo_nested-if-live",
    "d3-cfo_objarg-if-live",
    "d3-cfo_plus-if-live",
    "d3-cfo_plus-or-live",
    "d3-cfo_three-if-live",
    "d3-cfo_three-or-live",
    "d3-computed_member-if-live",
    "d3-computed_member-or-live",
    "d3-cond_arg-if-live",
    "d3-cond_arg-or-live",
    "d3-default-if-live",
    "d3-default-or-live",
    "d3-f1a",
    "d3-f1b",
    "d3-f1c",
    "d3-f1d",
    "d3-f1e",
    "d3-f1f",
    "d3-ifx_outer-if-dead",
    "d3-ifx_outer-or-dead",
    "d3-ifx_outer_alias-if-dead",
    "d3-ifx_outer_alias-or-dead",
    "d3-label_fn-if-live",
    "d3-label_fn-or-live",
    "d3-ternary_test_outer-if-dead",
    "d3-ternary_test_outer-or-dead",
];

#[test]
fn an_escape_an_inlined_control_flow_call_respans_prints_like_node_in_both_lanes() {
    prints_like_node_in(RESPANNED_ESCAPES, &[default_code, unsafe_lane]);
}

const CONTROL_FLOW_CYCLES: &[(&str, &str)] = &[
    ("cfo-assembled-cycle", "yPxtr"),
    ("cfo-helper-key", "DFSyf"),
];

#[test]
fn control_flow_objects_whose_escapes_other_objects_decide_dead_are_inlined_in_both_lanes() {
    for (label, key) in CONTROL_FLOW_CYCLES {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(!output.contains(key), "{label}:\n{output}");
        }
    }
}

#[test]
fn control_flow_objects_whose_escapes_other_objects_decide_dead_print_like_node_in_both_lanes() {
    let labels: Vec<&str> = CONTROL_FLOW_CYCLES
        .iter()
        .map(|(label, _)| *label)
        .collect();
    prints_like_node_in(&labels, &[default_code, unsafe_lane]);
}

#[test]
fn a_decoder_written_only_in_a_branch_its_strings_prove_dead_is_decoded_in_both_lanes() {
    let source = read("decoder-guarded-write.js");
    for output in [default_code(&source), unsafe_lane(&source)] {
        assert_eq!(output.trim(), "console.log(\"hello\");", "{output}");
    }
}

#[test]
fn a_decoder_written_in_a_live_branch_is_kept_in_both_lanes() {
    let source = read("decoder-guarded-write-live.js");
    for output in [default_code(&source), unsafe_lane(&source)] {
        assert!(output.contains("\"replaced\""), "{output}");
    }
    prints_like_node_in(
        &["decoder-guarded-write-live"],
        &[default_code, unsafe_lane],
    );
}

const GUARDED_DECODER_ALIASES: &[&str] = &[
    "guarded-alias-plain",
    "guarded-alias-hook",
    "guarded-alias-lazy",
    "guarded-alias-in-function",
    "guarded-alias-in-iife",
    "guarded-alias-chain",
    "guarded-alias-assigned",
    "guarded-alias-conditional",
    "guarded-alias-object",
    "guarded-alias-wrapper",
    "guarded-alias-wrapper-offset",
    "guarded-alias-else",
    "guarded-alias-ternary",
    "guarded-alias-after-rotator",
    "guarded-alias-fn-scope",
    "guarded-alias-dead-guard",
    "guarded-alias-taken-after",
    "guarded-alias-called-before",
    "guarded-alias-named-wrapper",
];

#[test]
fn an_alias_of_a_decoder_written_under_a_guard_prints_like_node_in_both_lanes() {
    prints_like_node_in(GUARDED_DECODER_ALIASES, &[default_code, unsafe_lane]);
}

const COOKIE_BLOCK_MOVES: &[&str] = &[
    "cookie-block-k1",
    "cookie-block-k2",
    "cookie-block-k3",
    "cookie-block-k4",
    "cookie-block-k5",
    "cookie-block-k6",
    "cookie-block-k7",
    "cookie-block-k8",
    "cookie-block-k9",
    "cookie-block-k10",
    "cookie-block-k11",
    "cookie-block-k12",
    "cookie-block-k13",
    "cookie-block-k14",
    "cookie-block-k15",
    "cookie-block-k28",
    "cookie-block-k29",
    "cookie-block-k30",
    "cookie-block-k31",
    "cookie-block-k32",
    "cookie-block-k33",
    "cookie-block-k34",
    "cookie-block-k35",
    "cookie-block-k36",
    "cookie-block-k37",
    "cookie-block-k38",
    "cookie-block-k39",
    "cookie-block-k40",
    "cookie-block-k41",
    "cookie-block-k42",
    "cookie-block-shaped",
    "cookie-block-shaped-direct",
    "cookie-block-shaped-closure",
    "cookie-block-shaped-array-write",
    "checksum-direct-write",
    "checksum-for-await",
    "checksum-alias-write",
];

#[test]
fn statements_moved_into_a_new_function_keep_their_meaning_in_both_lanes() {
    prints_like_node_in(COOKIE_BLOCK_MOVES, &[default_code, unsafe_lane]);
}

#[test]
fn a_cookie_block_in_the_obfuscator_shape_is_still_decoded_in_both_lanes() {
    for label in ["cookie-block-shaped", "cookie-block-shaped-direct"] {
        let source = read(&format!("{label}.js"));
        for output in [default_code(&source), unsafe_lane(&source)] {
            assert!(
                output.contains("\"one\", \"two\", \"three\""),
                "{label}:\n{output}"
            );
            assert!(!output.contains("arr0"), "{label}:\n{output}");
        }
    }
}

const MOVED_VARS_IN_LOOPS_AND_KEPT_WRITES: &[&str] = &[
    "ox2d4d-g1-b-d1",
    "ox2d4d-g1-b-d2",
    "ox2d4d-g1-b-d3",
    "ox2d4d-g1-b-s1",
    "ox2d4d-g1-b-s2",
    "ox2d4d-g1-b-s3",
    "ox2d4d-g1-b-s5",
    "ox2d4d-g1-b-s6",
    "ox2d4d-g1-block",
    "ox2d4d-g1-c-l1",
    "ox2d4d-g1-c-l3",
    "ox2d4d-g1-called",
    "ox2d4d-g1-function-array",
    "ox2d4d-g1-k-a2",
    "ox2d4d-g1-k-a4",
    "ox2d4d-g1-k-d1",
    "ox2d4d-g1-k-d2",
    "ox2d4d-g1-k-d4",
    "ox2d4d-g2-c-a2",
    "ox2d4d-g2-c-a3",
    "ox2d4d-g2-c-a4",
    "ox2d4d-g2-c-a5",
    "ox2d4d-g2-c-a6",
    "ox2d4d-g2-c-a7",
    "ox2d4d-g2-c-a7c",
];

#[test]
fn a_wrap_inside_a_loop_or_over_a_binding_that_is_not_a_plain_var_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        MOVED_VARS_IN_LOOPS_AND_KEPT_WRITES,
        &[default_code, unsafe_lane],
    );
}

const SHARED_CONTROLLERS_AND_LIVE_CALLBACKS: &[&str] = &[
    "d6b-f1a", "d6b-f1b", "d6b-f1c", "d6b-f1d", "d6b-f1e", "d6b-f1f", "d6b-f1g", "d6b-f1h",
    "d6b-f2a", "d6b-p1a", "d6b-p1b", "d6b-p1c", "d6b-p3",
];

#[test]
fn a_shared_call_controller_or_a_live_callback_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        SHARED_CONTROLLERS_AND_LIVE_CALLBACKS,
        &[default_code, unsafe_lane],
    );
}

const MERGED_BLOCK_GLOBAL_READS: &[&str] = &[
    "dc1-u1a",
    "dc1-u1b",
    "dc1-m01",
    "dc1-m02",
    "dc1-m03",
    "dc1-m04",
    "dc1-m05",
    "dc1-m06",
    "dc1-m07",
    "dc1-m08",
    "dc1-m09",
    "dc1-m10",
    "dc1-m11",
    "dc1-m12",
    "dc1-m13",
    "dc1-m14",
    "dc1-m15",
    "dc1-m16",
    "dc1-mg-000",
    "dc1-mg-050",
    "dc1-mg-100",
    "dc1-mg-149",
];

#[test]
fn a_block_kept_from_a_string_comparison_does_not_capture_a_global_read_in_either_lane() {
    prints_like_node_in(MERGED_BLOCK_GLOBAL_READS, &[default_code, unsafe_lane]);
}

const MEMO_HITS_AND_THROWS: &[&str] = &[
    "c4-throw-on-hit",
    "c4-write-then-throw",
    "c4-loop-throw-on-hit",
    "c4-loop-hit-differs",
    "c4-base64-throw-on-hit",
    "c4-base64-write-then-throw",
    "c4-base64-loop-hit-not-fn",
];

#[test]
fn a_stateful_decoder_whose_hit_path_or_thrown_call_differs_prints_like_node_in_both_lanes() {
    prints_like_node_in(MEMO_HITS_AND_THROWS, &[default_code, unsafe_lane]);
}

const COOKIE_VARS_WRITTEN_AFTER_THE_MOVE: &[&str] = &[
    "ox2d4e-g3-write",
    "ox2d4e-g3-read",
    "ox2d4e-g3-strict",
    "ox2d4e-g3-static",
    "ox2d4e-g3-break",
    "ox2d4e-g3-eval",
    "ox2d4e-g3-block-after",
    "ox2d4e-g3-closure-after",
    "ox2d4e-g3-var-after-strict",
];

#[test]
fn a_cookie_var_written_after_the_cookie_moves_keeps_an_outer_declaration_in_both_lanes() {
    prints_like_node_in(
        COOKIE_VARS_WRITTEN_AFTER_THE_MOVE,
        &[default_code, unsafe_lane],
    );
}

const COOKIE_VARS_NAMED_OUTSIDE_THE_LIST: &[&str] = &[
    "ox2d4f-g4-read-after",
    "ox2d4f-g4-read-after-strict",
    "ox2d4f-g4-holder-after",
    "ox2d4f-g4-holder-after-strict",
    "ox2d4f-g4-closure-after",
    "ox2d4f-g4-closure-after-strict",
    "ox2d4f-g4-fn-after",
    "ox2d4f-g4-fn-after-strict",
    "ox2d4f-g4-fn-before",
    "ox2d4f-g4-fn-before-strict",
    "ox2d4f-g4-write-before",
    "ox2d4f-g4-write-before-strict",
    "ox2d4f-g4-var-after",
    "ox2d4f-g4-var-after-strict",
    "ox2d4f-g4-if-after",
    "ox2d4f-g4-if-after-strict",
    "ox2d4f-g4-try-finally",
    "ox2d4f-g4-try-finally-strict",
    "ox2d4f-g4-labelled",
    "ox2d4f-g4-labelled-strict",
    "ox2d4f-g4-switch-next",
    "ox2d4f-g4-switch-next-strict",
    "ox2d4f-g4-do-after",
    "ox2d4f-g4-do-after-strict",
    "ox2d4f-g4-eval-after",
    "ox2d4f-g4-eval-after-strict",
];

#[test]
fn a_cookie_var_named_outside_the_wrapped_list_stays_outside_in_both_lanes() {
    prints_like_node_in(
        COOKIE_VARS_NAMED_OUTSIDE_THE_LIST,
        &[default_code, unsafe_lane],
    );
}

const DECODER_SELF_WRITES_AND_LAZY_ALIASES: &[&str] = &[
    "a4-body-write-reset",
    "a4-body-write-hook",
    "a4-lazy-inner-write",
    "a4-lazy-inner-replaces",
    "a4-lazy-alias-captured-hit",
    "a4-lazy-alias-throw-on-hit",
    "a4-lazy-alias-prop-memo",
    "a4-lazy-alias-identity",
    "a4-lazy-alias-property",
    "a4-lazy-alias-passed",
    "a4-lazy-alias-stored",
    "a4-lazy-alias-name",
    "a4-lazy-alias-compared",
    "a4-lazy-alias-call-only",
    "a4-lazy-alias-chain",
    "a4-lazy-alias-chain-compared",
    "a4-lazy-alias-keyed-memo",
    "a4-lazy-alias-arguments-memo",
];

#[test]
fn an_alias_of_a_decoder_that_rewrites_itself_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        DECODER_SELF_WRITES_AND_LAZY_ALIASES,
        &[default_code, unsafe_lane],
    );
}

#[test]
fn an_alias_of_a_lazy_decoder_used_only_as_a_callee_still_decodes() {
    let decoded = [
        ("a4-lazy-alias-call-only", "function d("),
        ("a4-lazy-alias-chain", "function d("),
        ("a4-lazy-alias-arguments-memo", "function bQ("),
    ];
    for (label, decoder) in decoded {
        let source = read(&format!("{label}.js"));
        for lane in [default_code, unsafe_lane] {
            let output = lane(&source);
            assert!(!output.contains(decoder), "{label}: {output}");
        }
    }
}

const MISS_THROWS_WITH_A_SHARED_ENTRY: &[&str] = &[
    "c5-miss-throw-shared-key",
    "c5-miss-throw-mod-key",
    "c5-base64-uri-error-mod-key",
    "c5-base64-uri-error-and-key",
    "c5-base64-loop-miss-throw",
];

#[test]
fn a_stateful_decoder_call_that_throws_on_a_shared_miss_prints_like_node_in_both_lanes() {
    prints_like_node_in(
        MISS_THROWS_WITH_A_SHARED_ENTRY,
        &[default_code, unsafe_lane],
    );
}

const MERGED_BLOCK_NAMES: &[&str] = &[
    "dc2-a1-class-name",
    "dc2-a1-fn-name",
    "dc2-a1-string-test",
    "dc2-a1-two-names",
    "dc2-a1-assigned",
    "dc2-a1-logical",
    "dc2-a1-pattern",
    "dc2-a1-target-default",
    "dc2-a1-closure",
    "dc2-a1-dropped-var",
    "dc2-a1-live-var",
    "dc2-a1-var-in-outer",
];

#[test]
fn a_block_binding_renamed_on_a_merge_keeps_its_function_or_class_name_in_the_default_lane() {
    prints_like_node_in(MERGED_BLOCK_NAMES, &[default_code]);
}

const MERGED_BLOCK_ANNEX_B_SHADOWS: &[&str] = &[
    "dc2-a2-annexb",
    "dc2-a2-annexb-switch",
    "dc2-a2-annexb-class",
    "dc2-a2-deep",
    "dc2-a2-forwarded",
    "dc2-a2-sibling",
    "dc2-arguments",
];

#[test]
fn a_block_binding_renamed_on_a_merge_still_shadows_a_nested_block_function_in_both_lanes() {
    prints_like_node_in(MERGED_BLOCK_ANNEX_B_SHADOWS, &[default_code, unsafe_lane]);
}

const MERGED_BLOCK_SHORTHAND_KEYS: &[&str] = &[
    "rn1-shorthand-target",
    "rn1-rest",
    "rn1-for-of",
    "rn1-nested",
    "rn1-default",
    "rn1-proto",
    "rn1-binding-and-literal",
    "rn1-mixed",
];

#[test]
fn a_block_binding_renamed_on_a_merge_keeps_the_key_of_a_shorthand_property_in_both_lanes() {
    prints_like_node_in(MERGED_BLOCK_SHORTHAND_KEYS, &[default_code, unsafe_lane]);
}

fn renamed_lane(rename: RenameMode, source: &str) -> String {
    let options = Options {
        rename,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

fn smart_lane(source: &str) -> String {
    renamed_lane(RenameMode::Smart, source)
}

fn mangle_lane(source: &str) -> String {
    renamed_lane(RenameMode::Mangle, source)
}

#[test]
fn a_block_binding_renamed_on_a_merge_keeps_the_key_of_a_shorthand_property_under_rename() {
    prints_like_node_in(MERGED_BLOCK_SHORTHAND_KEYS, &[smart_lane, mangle_lane]);
}

const LAZY_DECODERS_WHOSE_INNER_DIFFERS: &[&str] = &[
    "a5-lazy-f1",
    "a5-lazy-f1b",
    "a5-lazy-f1c",
    "a5-lazy-f1-obj",
    "a5-lazy-f2",
    "a5-lazy-f2-obj",
    "a5-lazy-namedsame",
    "a5-lazy-f3",
    "a5-lazy-f3-obj",
    "a5-lazy-f3b",
    "a5-lazy-f4",
    "a5-lazy-f4b",
    "a5-lazy-innerasync",
    "a5-lazy-f5",
    "a5-lazy-f5-obj",
    "a5-lazy-f5b",
    "a5-lazy-dcall-count",
    "a5-lazy-dpass",
    "a5-lazy-dtostr",
    "a5-lazy-f6",
    "a5-lazy-f6-obj",
    "a5-lazy-f6-iife",
    "a5-lazy-cwrite-in",
];

#[test]
fn an_alias_of_a_lazy_decoder_whose_inner_function_differs_from_a_fresh_call_prints_like_node_in_both_lanes(
) {
    prints_like_node_in(
        LAZY_DECODERS_WHOSE_INNER_DIFFERS,
        &[default_code, unsafe_lane],
    );
}

const LAZY_DECODER_COPIES: &[&str] = &[
    "a6-lazy-copy-argalias",
    "a6-lazy-copy-argalias-decl",
    "a6-lazy-copy-arrow",
    "a6-lazy-copy-asg",
    "a6-lazy-copy-asgseq-iife",
    "a6-lazy-copy-chain",
    "a6-lazy-copy-closure",
    "a6-lazy-copy-const-sif",
    "a6-lazy-copy-ctl",
    "a6-lazy-copy-declobj-o",
    "a6-lazy-copy-evald",
    "a6-lazy-copy-evalread",
    "a6-lazy-copy-fnget",
    "a6-lazy-copy-fnobj",
    "a6-lazy-copy-iife",
    "a6-lazy-copy-iife-o",
    "a6-lazy-copy-let",
    "a6-lazy-copy-loop",
    "a6-lazy-copy-mixed",
    "a6-lazy-copy-obj",
    "a6-lazy-copy-objsh",
    "a6-lazy-copy-objstr-o",
    "a6-lazy-copy-onecall",
    "a6-lazy-copy-tag",
    "a6-lazy-copy-twice",
    "a6-lazy-copy-two-dec-sif",
];

#[test]
fn copies_of_a_lazy_decoder_taken_apart_keep_every_alias_of_it_in_both_lanes() {
    prints_like_node_in(LAZY_DECODER_COPIES, &[default_code, unsafe_lane]);
}

#[test]
fn callee_only_aliases_of_a_lazy_decoder_in_a_function_run_twice_still_decode() {
    let source = read("a6-lazy-copy-callee-only.js");
    for lane in [default_code, unsafe_lane] {
        let output = lane(&source);
        assert!(
            !output.contains("var y") && !output.contains("Y.k"),
            "{output}"
        );
    }
    prints_like_node_in(&["a6-lazy-copy-callee-only"], &[default_code, unsafe_lane]);
}

const SHORTHAND_PROTO_ALIASES: &[&str] = &["rn2-cfo-alias-proto", "rn2-cfo-alias-proto-var"];

#[test]
fn a_control_flow_object_alias_read_through_a_shorthand_proto_key_keeps_the_key_in_both_lanes() {
    prints_like_node_in(SHORTHAND_PROTO_ALIASES, &[default_code, unsafe_lane]);
}

fn stdout_of(node: &Path, file: &Path) -> Option<String> {
    let observed = untwist_testing::node::run(node, file).ok()??;
    (observed.exit == Some(0)).then_some(observed.stdout)
}

fn unpacked_entry_prints(node: &Path, source: &str, options: Options) -> Option<String> {
    let dir = tempfile::tempdir().unwrap();
    Pipeline::new(options)
        .run(source)
        .write_to(dir.path())
        .ok()?;
    stdout_of(node, &dir.path().join("index.js"))
}

fn unpacked_prints_like_node(label: &str, lanes: &[Options]) {
    let Some(node) = node_or_skip() else {
        return;
    };
    let source = read(&format!("{label}.js"));
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle.js");
    std::fs::write(&bundle, &source).unwrap();
    let expected = stdout_of(&node, &bundle);
    assert!(expected.is_some(), "{label}");
    for lane in lanes {
        let actual = unpacked_entry_prints(&node, &source, lane.clone());
        assert_eq!(actual, expected, "{label}");
    }
}

#[test]
fn a_webpack_parameter_named_proto_in_a_shorthand_property_keeps_the_key_in_both_lanes() {
    unpacked_prints_like_node(
        "rn2-webpack-param-proto",
        &[
            Options::default(),
            Options {
                unsafe_transforms: true,
                ..Options::default()
            },
        ],
    );
}

#[test]
fn a_metro_parameter_named_proto_in_a_shorthand_property_keeps_the_key() {
    unpacked_prints_like_node(
        "rn2-metro-param-proto",
        &[Options {
            unsafe_transforms: true,
            unminify: false,
            ..Options::default()
        }],
    );
}

const LAZY_DECODER_COPIES_STILL_READ: &[&str] = &[
    "a7-lazy-copy-asgval",
    "a7-lazy-copy-asgval-arg",
    "a7-lazy-copy-asgval-copychain",
    "a7-lazy-copy-asgval-nested",
    "a7-lazy-copy-asgval-obj",
    "a7-lazy-copy-asgval-ret",
    "a7-lazy-copy-asgval-seq",
    "a7-lazy-copy-chain-param",
    "a7-lazy-copy-chain-param-two",
    "a7-lazy-copy-member-param",
];

#[test]
fn a_lazy_decoder_copy_read_through_its_assignment_or_a_mapped_parameter_prints_like_node_in_both_lanes(
) {
    prints_like_node_in(LAZY_DECODER_COPIES_STILL_READ, &[default_code, unsafe_lane]);
}

const LAZY_DECODER_COPIES_EXPORTED: &[&str] = &[
    "a7-lazy-copy-export",
    "a7-lazy-copy-export-asg",
    "a7-lazy-copy-export-chain",
    "a7-lazy-copy-export-obj",
];

fn printed_as_module(node: &Path, dir: &Path, text: &str) -> Option<String> {
    let path = dir.join("program.mjs");
    std::fs::write(&path, text).unwrap();
    run(node, &path).unwrap().map(|out| observed(&out))
}

#[test]
fn a_lazy_decoder_copy_exported_by_its_declaration_prints_like_node_in_both_lanes() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let mut wrong = Vec::new();
    for label in LAZY_DECODER_COPIES_EXPORTED {
        let source = read(&format!("{label}.mjs"));
        let expected = printed_as_module(&node, dir.path(), &source);
        assert_eq!(
            expected.as_deref(),
            Some("hello\nworld\nfalse\nexit 0\n"),
            "{label}"
        );
        for lane in [default_code, unsafe_lane] {
            let actual = printed_as_module(&node, dir.path(), &lane(&source));
            if expected != actual {
                wrong.push(format!("{label}: {expected:?} vs {actual:?}"));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const SHORTHAND_PROTO_CONSTANTS: &[&str] = &[
    "rn3-const-arg-proto",
    "rn3-const-arg-proto-later",
    "rn3-const-arg-proto-source",
];

#[test]
fn a_decoder_argument_constant_named_proto_inlined_into_a_shorthand_keeps_the_key_in_both_lanes() {
    prints_like_node_in(SHORTHAND_PROTO_CONSTANTS, &[default_code, unsafe_lane]);
}

#[test]
fn an_assign_over_a_literal_with_a_proto_key_is_not_spread_in_either_lane() {
    prints_like_node_in(&["rn3-assign-literal-proto"], &[default_code, unsafe_lane]);
}
