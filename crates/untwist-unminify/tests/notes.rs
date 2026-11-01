use untwist_testing::notes as invariants;
use untwist_unminify::notes;

const TABLES: &[invariants::Table] = &[notes::ALL_NOTES];

#[test]
fn no_note_is_a_substring_of_another() {
    invariants::no_note_is_a_substring_of_another(TABLES);
}

#[test]
fn every_constant_is_catalogued_exactly_once() {
    invariants::every_constant_is_catalogued_exactly_once(include_str!("../src/notes.rs"), TABLES);
}

#[test]
fn the_pass_modules_still_name_their_hazard() {
    assert_eq!(
        untwist_unminify::let_to_const::SCOPE_HAZARD,
        notes::LET_TO_CONST_SCOPE_HAZARD
    );
    assert_eq!(
        untwist_unminify::unwrap_comma_call::SCOPE_HAZARD,
        notes::UNWRAP_COMMA_CALL_SCOPE_HAZARD
    );
}
