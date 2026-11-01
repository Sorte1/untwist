use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::parse::parse_program;
use untwist_core::pass::{PassOptions, RunReport, StopReason};
use untwist_core::print::{print, PrintStyle};
use untwist_core::semantic::build_scoping;
use untwist_deobfuscate::removal::{machinery, outstanding_references};
use untwist_deobfuscate::run_deobfuscate;
use untwist_deobfuscate::stamp::STRING_ARRAY_NAME;
use untwist_eval::decode::DECODE_ERROR_MARKER;

const GAME_DECODER: &str = "iíìîiîï";
const GAME_ARRAY_FN: &str = "iïîïïíî";
const STAMPED_DECODER_PREFIX: &str = "__DECODE_";
const INLINE_DECODED_STRINGS: &str = "inline-decoded-strings";

const REFERENCE_INLINE_DECODER_WRAPPERS: u32 = 121_866;
const REFERENCE_INLINE_DECODED_STRINGS: u32 = 110_481;
const ROTATOR_INTERNAL_CALLS_LEFT_TO_THE_REMOVAL_PASS: u32 = 12;

const GAME_STRING_ARRAY_RENAMES: u32 = 26;
const GAME_DECODER_RENAMES: u32 = 1_655;
const GAME_SITES_REWRITTEN_ACROSS_ALIAS_AND_FUNCTION_WRAPPERS: u32 = 142_967;
const GAME_DECODED_CALL_SITES: u32 = 110_469;
const GAME_MACHINERY_STATEMENTS_REMOVED: u32 = 3;
const GAME_OBJECT_PROPS_INLINED_AFTER_REMOVAL: u32 = 0;
const GAME_MERGED_STRINGS: u32 = 1;
const GAME_DEAD_CODE_REMOVED: u32 = 0;
const GAME_CONTROL_FLOW_OBJECTS_INLINED: u32 = 0;
const GAME_CONTROL_FLOW_SWITCHES_FLATTENED: u32 = 0;
const GAME_ROUNDS: u32 = 3;
const GAME_TOTAL_CHANGES: u32 = 255_121;
const GAME_DECODE_ERROR_NOTES: usize = 0;
const GAME_DECODING_PASS_NOTES: usize = 0;
const GAME_OUTPUT_BYTES: usize = 10_827_045;
const GAME_OUTPUT_FNV1A: u64 = 8_299_649_761_812_506_613;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn assert_the_output_is_the_frozen_one(code: &str) {
    assert_eq!(
        (code.len(), fnv1a(code.as_bytes())),
        (GAME_OUTPUT_BYTES, GAME_OUTPUT_FNV1A),
        "the per-pass counts alone cannot see a changed byte inside a decoded string; \
         re-measure both only with the mechanism for the change written down"
    );
}

const _: () = assert!(
    GAME_DECODED_CALL_SITES
        == REFERENCE_INLINE_DECODED_STRINGS - ROTATOR_INTERNAL_CALLS_LEFT_TO_THE_REMOVAL_PASS
);

fn expected_per_pass() -> Vec<(&'static str, u32)> {
    vec![
        ("find-string-array", GAME_STRING_ARRAY_RENAMES),
        ("find-decoders", GAME_DECODER_RENAMES),
        (
            "inline-object-props",
            GAME_OBJECT_PROPS_INLINED_AFTER_REMOVAL,
        ),
        (
            "inline-decoder-wrappers",
            GAME_SITES_REWRITTEN_ACROSS_ALIAS_AND_FUNCTION_WRAPPERS,
        ),
        (INLINE_DECODED_STRINGS, GAME_DECODED_CALL_SITES),
        (
            "remove-decoder-machinery",
            GAME_MACHINERY_STATEMENTS_REMOVED,
        ),
    ]
}

fn assert_stage_counts(report: &RunReport) {
    assert_eq!(
        report.per_pass,
        expected_per_pass(),
        "inline-decoder-wrappers counts every site it rewrites across both alias and function \
         wrappers, so it exceeds the reference deobfuscator's {REFERENCE_INLINE_DECODER_WRAPPERS} transform count; \
         inline-decoded-strings is the reference deobfuscator's {REFERENCE_INLINE_DECODED_STRINGS} minus the \
         {ROTATOR_INTERNAL_CALLS_LEFT_TO_THE_REMOVAL_PASS} rotator-internal call sites untwist \
         leaves to remove-decoder-machinery; inline-object-props reaches the two constant lookup \
         maps in the extra round, but refuses them because every refusing read sits in \
         an anonymous method closure inside the descriptor array a _createClass call receives, \
         inside an IIFE body that runs in a statement above the declaration, and the shared \
         init-order helper seeds every function body created in a root as reachable"
    );
    assert_eq!(
        report.fixpoint_per_pass,
        vec![
            ("merge-strings", GAME_MERGED_STRINGS),
            ("dead-code", GAME_DEAD_CODE_REMOVED),
            ("control-flow-object", GAME_CONTROL_FLOW_OBJECTS_INLINED),
            ("control-flow-switch", GAME_CONTROL_FLOW_SWITCHES_FLATTENED),
        ],
        "the cleanup fixpoint group reports beside the seven decoding stages, not inside them"
    );
    assert_eq!(report.iterations, GAME_ROUNDS, "rounds");
    assert_eq!(report.total_changes, GAME_TOTAL_CHANGES, "total changes");
}

fn assert_every_call_site_decoded_to_a_string(report: &RunReport) {
    let marked = report
        .notes
        .iter()
        .filter(|note| note.message.contains(DECODE_ERROR_MARKER))
        .count();
    assert_eq!(
        marked, GAME_DECODE_ERROR_NOTES,
        "a call site that does not decode to a string is replaced by undefined and only recorded \
         as a {DECODE_ERROR_MARKER} note, so the pinned counts alone would survive a bundle that \
         decoded entirely to undefined"
    );
    let from_the_decoding_pass: Vec<&str> = report
        .notes
        .iter()
        .filter(|note| note.pass == INLINE_DECODED_STRINGS)
        .map(|note| note.message.as_str())
        .collect();
    assert_eq!(
        from_the_decoding_pass.len(),
        GAME_DECODING_PASS_NOTES,
        "every note this pass emits reports a decode error, a partial result, a non-string result \
         or a bail: {from_the_decoding_pass:?}"
    );
}

fn assert_machinery_is_gone(program: &Program<'_>) {
    let left = machinery(program);
    assert!(left.array.is_none(), "the string array survived");
    assert!(left.rotator.is_none(), "the rotator survived");
    assert!(
        left.decoders.is_empty(),
        "{} decoder declarations survived",
        left.decoders.len()
    );
    let outstanding = outstanding_references(program);
    assert!(
        outstanding.is_empty(),
        "{} references to the string array or its decoders did not decode",
        outstanding.len()
    );
}

fn assert_no_machinery_name_is_printed(code: &str) {
    assert!(!code.contains(GAME_DECODER), "the decoder name survived");
    assert!(!code.contains(GAME_ARRAY_FN), "the array name survived");
    assert!(
        !code.contains(STAMPED_DECODER_PREFIX),
        "a stamped decoder name survived"
    );
    assert!(
        !code.contains(STRING_ARRAY_NAME),
        "the stamped string array name survived"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "10.7 MB fixture; run with cargo test --release -p untwist-deobfuscate --test real_world"
)]
fn game_js_string_array_machinery_is_decoded_and_removed() {
    let Some(source) = untwist_testing::real_world_fixture("game.js") else {
        return;
    };
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, &source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let stripped =
        untwist_deobfuscate::directives::strip_anonymous_shell(&mut parsed.program, &alloc);
    assert!(
        matches!(
            stripped,
            untwist_deobfuscate::directives::ShellStrip::Stripped { .. }
        ),
        "game.js is one anonymous shell, which the Pipeline strips before deobfuscating"
    );
    let sem = build_scoping(&parsed.program);
    let opts = PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: None,
        fuel: None,
        nested_root: true,
        ..PassOptions::default()
    };
    let (_sem, report) = run_deobfuscate(&alloc, &mut parsed.program, sem, &opts);
    assert!(
        report.prepare_changes > 0,
        "run_deobfuscate runs the normalisers as its own stage 0, outside per_pass \
         and total_changes"
    );
    assert_eq!(report.stop, StopReason::Converged, "{:?}", report.stop);
    assert_stage_counts(&report);
    assert_every_call_site_decoded_to_a_string(&report);
    assert_machinery_is_gone(&parsed.program);

    let code = print(&parsed.program, PrintStyle::Pretty);
    assert_no_machinery_name_is_printed(&code);
    assert_the_output_is_the_frozen_one(&code);

    let check = Allocator::default();
    let reparsed = parse_program(&check, &code);
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:?}",
        reparsed.diagnostics
    );
}
