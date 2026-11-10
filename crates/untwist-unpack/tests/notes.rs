use untwist_testing::notes as invariants;
use untwist_unpack::notes;

const TABLES: &[invariants::Table] = notes::TABLES;

#[test]
fn no_note_is_a_substring_of_another() {
    invariants::no_note_is_a_substring_of_another(TABLES);
}

#[test]
fn every_constant_is_catalogued_exactly_once() {
    let sources = [
        include_str!("../src/notes.rs"),
        include_str!("../src/esbuild/notes.rs"),
        include_str!("../src/bun/notes.rs"),
        include_str!("../src/rollup/notes.rs"),
        include_str!("../src/systemjs/notes.rs"),
        include_str!("../src/amd/notes.rs"),
        include_str!("../src/umd/notes.rs"),
        include_str!("../src/parcel/notes.rs"),
        include_str!("../src/metro/notes.rs"),
        include_str!("../src/turbopack/notes.rs"),
        include_str!("../src/closure/notes.rs"),
    ];
    invariants::every_constant_is_catalogued_exactly_once(&sources.concat(), TABLES);
}

#[test]
fn every_builder_spells_the_text_it_replaced() {
    assert_eq!(
        notes::unsafe_path_replaced("3", "../x", "x.js"),
        "module 3: unsafe path \"../x\" replaced by x.js"
    );
    assert_eq!(
        notes::unsafe_output_path_replaced("3", "a/../b"),
        "module 3: unsafe output path \"a/../b\" replaced"
    );
    assert_eq!(
        notes::path_collision("3", "a-1.js", "a.js"),
        "module 3: output path collision, writing a-1.js instead of a.js"
    );
    assert_eq!(
        notes::mapping_unsafe_path("../x"),
        "mapping \"../x\": unsafe path, ignored"
    );
    assert_eq!(
        notes::mapping_unmatched("a.js"),
        "mapping \"a.js\": no module matched"
    );
    assert_eq!(
        notes::merge_kind_mismatch("browserify", "webpack"),
        "chunk merge: ignoring a browserify bundle in a webpack merge"
    );
    assert_eq!(
        notes::later_entry_ignored("9", "1"),
        "entry 9 from a later chunk ignored; keeping 1"
    );
    assert_eq!(
        notes::duplicate_module("7"),
        "chunk merge: duplicate module 7, keeping the first"
    );
    assert_eq!(
        notes::entry_candidates("1", 3),
        "entry 1 is the first of 3 top-level require calls and no entry marker is assigned; the others are modules the startup body also runs"
    );
    assert_eq!(
        notes::require_missing("4", "./x"),
        "module 4: require(\"./x\") has no module in this bundle"
    );
    assert_eq!(
        notes::unhandled_helper("4", 't'),
        "module 4: unhandled webpack helper require.t"
    );
}
