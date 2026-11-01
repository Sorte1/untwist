use untwist_core::ast_util::rename::UNIQUE_NAME_SUFFIX_CAP;
use untwist_core::pass::{Note, PassGroup, PassOptions};
use untwist_jsx::notes;
use untwist_testing::notes as invariants;
use untwist_testing::Allocator;

const TABLES: &[invariants::Table] = &[notes::ALL_NOTES, notes::GUARD_NOTES];

const PROGRESS: &[&str] = &[notes::TYPE_HOISTED];

enum Source {
    Whole(&'static str),
    Built(fn() -> String),
}

fn program(source: &Source) -> String {
    match source {
        Source::Whole(text) => (*text).to_string(),
        Source::Built(build) => build(),
    }
}

fn every_component_uid_declared() -> String {
    let mut source = String::from("var Component = 1;\n");
    for suffix in 2..=UNIQUE_NAME_SUFFIX_CAP {
        source.push_str(&format!("var Component_{suffix}__ = 1;\n"));
    }
    source
}

fn every_component_uid_bound() -> String {
    let mut source = every_component_uid_declared();
    source.push_str("function a() {}\njsx(a, {});");
    source
}

fn every_component_uid_bound_at_four_sites() -> String {
    let mut source = every_component_uid_declared();
    source.push_str(
        "function a() {}\nfunction b() {}\nfunction c() {}\nfunction d() {}\n\
         jsx(a, {});\njsx(b, {});\njsx(c, {});\njsx(d, {});",
    );
    source
}

const CASES: &[(Source, &str)] = &[
    (
        Source::Whole("React.createElement(\"div\", { [k]: 1 });"),
        notes::ATTRIBUTE_SHAPE,
    ),
    (Source::Whole("jsx(\"\", {});"), notes::ELEMENT_NAME_SHAPE),
    (
        Source::Whole("jsx(\"Foo\", {});"),
        notes::COMPONENT_LOOKING_STRING,
    ),
    (
        Source::Whole("React.createElement(a, null);"),
        notes::LOWERCASE_UNBOUND,
    ),
    (
        Source::Whole("jsx(r ? \"a\" : \"div\", {});"),
        notes::TYPE_HOISTED,
    ),
    (
        Source::Whole("for (let i = 0; g(jsx(w[i], {})); i++) h();"),
        notes::HOIST_ANCHOR_UNRESOLVED,
    ),
    (
        Source::Whole("jsx(\"div\", {}, k, extra);"),
        notes::KEY_SHAPE,
    ),
    (
        Source::Built(every_component_uid_bound),
        notes::COMPONENT_NAME_EXHAUSTED,
    ),
];

fn pinned() -> Vec<&'static str> {
    CASES.iter().map(|(_, expects)| *expects).collect()
}

fn report_notes(source: &str) -> Vec<Note> {
    let alloc = Allocator::default();
    let opts = PassOptions {
        unsafe_transforms: true,
        extended: true,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    };
    let group = PassGroup::from_ctors(&untwist_jsx::jsx_ctors(), opts);
    let (_code, report) = untwist_testing::transform_report_with(&alloc, source, group);
    report.notes
}

fn report(source: &str) -> Vec<String> {
    report_notes(source)
        .into_iter()
        .map(|note| note.message)
        .collect()
}

#[test]
fn no_catalogued_note_is_a_substring_of_another() {
    invariants::no_note_is_a_substring_of_another(TABLES);
}

#[test]
fn every_message_constant_is_catalogued_exactly_once() {
    invariants::every_constant_is_catalogued_exactly_once(include_str!("../src/notes.rs"), TABLES);
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
fn every_progress_note_is_emitted_as_progress_and_every_other_as_refusal() {
    for (source, _) in CASES {
        let emitted = report_notes(&program(source));
        invariants::every_note_carries_the_kind_its_text_claims(&emitted, PROGRESS);
    }
}

#[test]
fn no_guard_note_fires_from_any_pinned_program() {
    for (source, _) in CASES {
        let source = program(source);
        let all = report(&source);
        for (name, guard) in notes::GUARD_NOTES {
            assert!(
                !all.iter().any(|message| message.contains(guard)),
                "{name} is a guard but fires on {source}: {all:?}"
            );
        }
    }
}

#[test]
fn every_case_fires_its_note() {
    for (source, expects) in CASES {
        let source = program(source);
        let all = report(&source);
        assert!(
            all.iter().any(|message| message.contains(expects)),
            "{source} did not report {expects}: {all:?}"
        );
    }
}

const CAPPED: &[(Source, &str)] = &[
    (
        Source::Whole(
            "React.createElement(\"a\", { [k]: 1 });\nReact.createElement(\"b\", { [k]: 2 });\n\
             React.createElement(\"c\", { [k]: 3 });\nReact.createElement(\"d\", { [k]: 4 });",
        ),
        notes::ATTRIBUTE_SHAPE,
    ),
    (
        Source::Whole(
            "React.createElement(p, null);\nReact.createElement(q, null);\n\
             React.createElement(r, null);\nReact.createElement(s, null);",
        ),
        notes::LOWERCASE_UNBOUND,
    ),
    (
        Source::Whole(
            "jsx(a ? \"a\" : \"b\", {});\njsx(b ? \"a\" : \"b\", {});\n\
             jsx(c ? \"a\" : \"b\", {});\njsx(d ? \"a\" : \"b\", {});",
        ),
        notes::TYPE_HOISTED,
    ),
    (
        Source::Whole("jsx(\"A\", {});\njsx(\"B\", {});\njsx(\"C\", {});\njsx(\"D\", {});"),
        notes::COMPONENT_LOOKING_STRING,
    ),
    (
        Source::Whole(
            "jsx(\"a\", {}, k, x);\njsx(\"b\", {}, k, x);\njsx(\"c\", {}, k, x);\njsx(\"d\", {}, k, x);",
        ),
        notes::KEY_SHAPE,
    ),
    (
        Source::Built(every_component_uid_bound_at_four_sites),
        notes::COMPONENT_NAME_EXHAUSTED,
    ),
];

#[test]
fn a_note_repeated_at_four_sites_is_three_spanned_and_one_summary() {
    for (source, message) in CAPPED {
        let all = report(&program(source));
        let carrying: Vec<&String> = all.iter().filter(|note| note.contains(message)).collect();
        assert_eq!(
            carrying.len(),
            untwist_core::pass::SPANNED_NOTE_CAP + 1,
            "{message} was reported {} times over four sites: {carrying:?}",
            carrying.len()
        );
        assert_eq!(
            carrying
                .iter()
                .filter(|note| note.contains("4 sites"))
                .count(),
            1,
            "exactly one summary counting every site: {carrying:?}"
        );
    }
}
