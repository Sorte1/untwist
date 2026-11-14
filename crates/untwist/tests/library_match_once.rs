use untwist::{Options, Pipeline, RenameMode};

#[test]
fn a_rename_mode_matches_each_program_once() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/libraries/tslib/rollup-cjs.js"
    );
    let source = std::fs::read_to_string(path).expect("reading the tslib fixture");
    for rename in [RenameMode::Smart, RenameMode::Unique, RenameMode::Simple] {
        let options = Options {
            rename,
            unpack: false,
            ..Options::default()
        };
        let marked = format!("{source}\nvar mode = {:?};\n", format!("{rename:?}"));
        let before = untwist_rename::library::cache::computed();
        let output = Pipeline::new(options).run(&marked);
        let matched = untwist_rename::library::cache::computed() - before;
        assert!(
            output.notes.iter().any(|note| note.pass == "library-names"),
            "{rename:?}"
        );
        assert_eq!(matched, 1, "{rename:?} matched the program {matched} times");
    }
}
