use std::path::Path;

fn export_names(register: &str) -> Vec<&str> {
    register
        .split("_export(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

#[test]
fn the_generated_systemjs_entry_exports_names_in_sorted_order() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("corpus-bundle-systemjs");
    xtask::corpus_bundle::generate_kind(&root, "systemjs", 2, xtask::corpus_bundle::DEFAULT_SEED)
        .unwrap();
    let text =
        std::fs::read_to_string(root.join(xtask::corpus_bundle::output_for("systemjs"))).unwrap();
    let entry = text
        .split("System.register(\"m0\"")
        .nth(1)
        .and_then(|rest| rest.split("System.register(").next())
        .expect("the entry register");
    let names = export_names(entry);
    assert_eq!(names.len(), 61);
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]), "{names:?}");
}
