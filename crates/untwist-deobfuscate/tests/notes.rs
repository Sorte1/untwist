#[path = "notes/cases.rs"]
mod cases;
mod common;

use std::collections::HashSet;

use cases::{program, stages, Match, CASES};
use untwist_deobfuscate::notes;
use untwist_testing::notes as invariants;

const TABLES: &[invariants::Table] = &[notes::ALL_NOTES, notes::GUARD_NOTES];

const PROGRESS: &[&str] = &[
    notes::STRING_ARRAY_FOUND,
    notes::ROTATOR_FOUND,
    notes::ROTATOR_NONE,
    notes::DECODERS_FOUND,
    notes::DECODED_CALL_SITES,
    notes::DECODE_ERROR,
    notes::MERGE_ADOPTED_DYNAMIC_KEY,
    notes::CONTROL_FLOW_DEAD_WRITE_KEPT,
    notes::NO_STRING_ARRAY_NOTE,
    notes::SIMPLE_ARRAY_NAME_OBSERVED,
    notes::MACHINERY_REMOVED_UNDER_OUTSIDE_PREMISE,
];

fn pinned() -> Vec<&'static str> {
    CASES.iter().map(|case| case.expects).collect()
}

#[test]
fn every_near_miss_reports_its_reason() {
    for case in CASES {
        let source = program(&case.source);
        let (_code, all) = stages(case, &source);
        let reported = all.iter().any(|note| match case.mode {
            Match::Contains => note.contains(case.expects),
            Match::Exact => note == case.expects,
        });
        assert!(
            reported,
            "{source} did not report {}: {all:?}",
            case.expects
        );
    }
}

#[test]
fn every_progress_note_is_emitted_as_progress_and_every_other_as_refusal() {
    for case in CASES {
        let source = program(&case.source);
        let (_code, all) = cases::stage_notes(case, &source);
        invariants::every_note_carries_the_kind_its_text_claims(&all, PROGRESS);
    }
}

#[test]
fn a_refusal_the_next_round_resolves_is_not_reported() {
    let source = "function hi() {\n  var obj = { rLxJs: 'x', aokfw: function (a, b) { return a + b; } };\n  console.log(obj.aokfw(obj.rLxJs, 2));\n}";
    let (code, all) = common::run_all_stages(source);
    assert_eq!(code.trim(), "function hi() {\n  console.log(\"x\" + 2);\n}");
    assert!(
        !all.iter()
            .any(|note| note.contains(notes::CONTROL_FLOW_OBJECT_PARTIAL)),
        "the first round's partial fold is reported over a program that has no object left: \
         {all:?}"
    );
}

#[test]
fn a_refusal_that_still_holds_is_reported() {
    let source =
        "function hi() {\n  var obj = { rLxJs: '6|0|4' };\n  console.log(obj.rLxJs, obj.zzzzz);\n}";
    let (code, all) = common::run_all_stages(source);
    assert!(code.contains("obj.zzzzz"), "{code}");
    assert!(
        all.iter()
            .any(|note| note.contains(notes::CONTROL_FLOW_OBJECT_PARTIAL)),
        "{all:?}"
    );
}

#[test]
fn every_catalogued_note_is_reachable_from_some_case() {
    invariants::every_catalogued_note_is_reachable(notes::ALL_NOTES, &pinned());
}

#[test]
fn every_case_names_a_catalogued_note() {
    invariants::every_case_names_a_catalogued_note(notes::ALL_NOTES, &pinned());
}

#[test]
fn every_message_constant_is_catalogued_exactly_once() {
    invariants::every_constant_is_catalogued_exactly_once(include_str!("../src/notes.rs"), TABLES);
}

#[test]
fn no_guard_note_fires_from_any_pinned_program() {
    for case in CASES {
        let source = program(&case.source);
        let (_code, all) = stages(case, &source);
        for (name, guard) in notes::GUARD_NOTES {
            assert!(
                !all.iter().any(|note| note.contains(guard)),
                "{name} is a guard but fires on {source}: {all:?}"
            );
        }
    }
}

fn composed_string_array() -> Vec<(&'static str, String)> {
    vec![
        ("STRING_ARRAY_FOUND", notes::string_array_found(21916)),
        ("SIMPLE_ARRAY_ACCESS", notes::simple_array_access("push")),
        (
            "SIMPLE_ARRAY_ACCESS",
            notes::simple_array_accesses(
                23,
                &[("push".to_string(), 14), ("indexOf".to_string(), 9)],
            ),
        ),
        ("ARRAY_ELEMENT_NOT_FOUND", notes::array_element_not_found(3)),
        (
            "ARRAY_ALIAS_NAME_NOT_UNIQUE",
            notes::array_alias_name_not_unique("a"),
        ),
        (
            "ALIAS_REMOVAL_PLAN_INCOMPLETE",
            notes::planned_and_applied_alias_removals(2, 1),
        ),
        ("DECODERS_FOUND", notes::decoders_found(1)),
    ]
}

fn composed_objects() -> Vec<(&'static str, String)> {
    vec![
        ("OBJECT_MISSING_PROP", notes::object_missing_prop("prm")),
        (
            "OBJECT_PROP_VALUE_MISSING",
            notes::object_prop_value_missing("prm"),
        ),
        ("OBJECT_PROPS_PARTIAL", notes::object_props_partial(1, 2)),
        ("OBJECT_AMBIGUOUS_KEY", notes::object_ambiguous_key("aaa")),
    ]
}

fn composed_wrappers() -> Vec<(&'static str, String)> {
    vec![
        (
            "WRAPPER_SITE_SCOPED_ARGUMENT",
            notes::wrapper_site_scoped_argument(),
        ),
        (
            "WRAPPER_SITE_IMPURE_ARGUMENT",
            notes::wrapper_site_impure_argument(),
        ),
        ("WRAPPER_PARTIAL", notes::wrapper_partial(1, 2)),
        ("WRAPPER_ROUNDS", notes::wrapper_rounds(16)),
    ]
}

fn composed_decoding() -> Vec<(&'static str, String)> {
    vec![
        (
            "DECODE_BAILED",
            notes::decode_bailed("unresolved global binding: q"),
        ),
        ("DECODE_PARTIAL", notes::decode_partial(1, 2)),
        ("DECODE_ERROR", notes::decode_error()),
        (
            "UNATTRIBUTED_DECODER",
            notes::unattributed_decoder("__DECODE_0__"),
        ),
        ("DECODED_CALL_SITES", notes::decoded_call_sites(0, 0, 1)),
        (
            "MACHINERY_REFERENCES_KEPT",
            notes::machinery_references_kept(1),
        ),
        ("MACHINERY_PARTIAL_REMOVAL", notes::machinery_removed(2, 3)),
    ]
}

fn composed_control_flow() -> Vec<(&'static str, String)> {
    vec![
        (
            "CONTROL_FLOW_MISSING_PROP",
            notes::control_flow_missing_prop("zzzzz"),
        ),
        (
            "CONTROL_FLOW_DUPLICATE_KEY",
            notes::control_flow_duplicate_key("aaaaa"),
        ),
        (
            "CONTROL_FLOW_OBJECT_PARTIAL",
            notes::control_flow_object_partial(1, 2),
        ),
        (
            "CONTROL_FLOW_UNCALLED_FUNCTION_PROPERTY",
            notes::control_flow_uncalled_function_properties(5),
        ),
        (
            "LOGICAL_FORWARDER_ARGUMENT",
            notes::logical_forwarder_arguments(5),
        ),
        ("SWITCH_SEQUENCE", notes::switch_sequence("9")),
    ]
}

fn composed_stage_wide() -> Vec<(&'static str, String)> {
    vec![
        ("GLOBAL_NOT_EVALUATED", notes::global_not_evaluated("atob")),
        ("GLOBAL_REBOUND", notes::global_rebound("atob")),
        ("INIT_ORDER_BUDGET", notes::init_order_budget("m")),
        (
            "EDIT_PLAN_INCOMPLETE",
            notes::planned_and_applied_edits("control-flow-object", 3, 2),
        ),
        (
            "PROTECTION_REMOVAL_PLAN_INCOMPLETE",
            notes::planned_and_applied_removals("self-defending", 3, 2),
        ),
        ("SCOPE_HAZARD", notes::scope_hazard("with statement")),
        ("ROUND_CAP", notes::round_cap(64)),
        ("ROUND_CAP", notes::round_cap_undone(64)),
        (
            "PREPARATION_STOPPED_EARLY",
            notes::preparation_stopped_early("iteration cap"),
        ),
    ]
}

fn composed_messages() -> Vec<(&'static str, String)> {
    let mut all = composed_string_array();
    all.extend(composed_objects());
    all.extend(composed_wrappers());
    all.extend(composed_decoding());
    all.extend(composed_control_flow());
    all.extend(composed_stage_wide());
    all
}

fn builders_declared() -> HashSet<String> {
    include_str!("../src/notes.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("pub fn "))
        .filter(|line| line.ends_with("-> String {"))
        .filter_map(|line| line.split('(').next())
        .map(str::to_string)
        .collect()
}

fn builders_composed() -> HashSet<String> {
    include_str!("notes.rs")
        .lines()
        .filter_map(|line| line.split_once("notes::"))
        .filter_map(|(_, rest)| rest.split('(').next())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_builder_appears_in_the_composed_list() {
    let declared = builders_declared();
    let composed = builders_composed();
    let missing: Vec<&String> = declared.difference(&composed).collect();
    assert!(
        missing.is_empty(),
        "a builder in notes.rs is missing from composed_messages, so the message it emits is \
         never checked for a second catalogued constant: {missing:?}"
    );
    assert_eq!(
        composed_messages().len(),
        composed_messages()
            .iter()
            .map(|(_, message)| message.clone())
            .collect::<HashSet<String>>()
            .len(),
        "composed_messages repeats a message"
    );
}

#[test]
fn a_composed_message_carrying_a_second_catalogued_note_is_pinned_exactly() {
    for (owner, message) in composed_messages() {
        for (name, catalogued) in notes::ALL_NOTES.iter().chain(notes::GUARD_NOTES) {
            if *name == owner || !message.contains(catalogued) {
                continue;
            }
            let pinned_exactly = CASES
                .iter()
                .any(|case| case.expects == *catalogued && matches!(case.mode, Match::Exact));
            assert!(
                pinned_exactly,
                "{name} is a substring of the message {owner} composes ({message:?}), so its \
                 case must pin it with Match::Exact or a program that only produces {owner} \
                 would satisfy it"
            );
        }
    }
}

#[test]
fn no_catalogued_note_is_a_substring_of_another() {
    invariants::no_note_is_a_substring_of_another(TABLES);
}

#[test]
fn a_clean_program_reports_no_note_other_than_the_skip_note() {
    let (_code, all) = common::run_all_stages("console.log(1 + 2);");
    let unexpected: Vec<&String> = all
        .iter()
        .filter(|note| *note != untwist_deobfuscate::NO_STRING_ARRAY_NOTE)
        .collect();
    assert!(unexpected.is_empty(), "{all:?}");
}
