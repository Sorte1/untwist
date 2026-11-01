const REFERENCE_DEOBFUSCATE_FIXTURES: usize = 16;
const JSX_FIXTURES: usize = 6;

fn count_js_files(dir: &str) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "js"))
        .count()
}

#[test]
fn reference_fixtures_are_imported() {
    let deob = count_js_files(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference/deobfuscate"
    ));
    let unpack = count_js_files(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/reference/unpack"
    ));
    let real_world = count_js_files(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../fixtures/real-world"
    ));
    let jsx = count_js_files(concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/jsx"));
    assert_eq!(
        deob, REFERENCE_DEOBFUSCATE_FIXTURES,
        "deobfuscate fixtures: {deob}"
    );
    assert_eq!(unpack, 18, "unpack fixtures: {unpack}");
    assert!(real_world <= 1, "real-world fixtures: {real_world}");
    assert_eq!(jsx, JSX_FIXTURES, "jsx fixtures: {jsx}");
}

#[test]
fn every_deobfuscate_fixture_has_an_expected_snapshot() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .join("fixtures/reference/deobfuscate");
    let mut inputs = 0;
    for entry in std::fs::read_dir(&root).expect("fixtures dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "js") {
            inputs += 1;
            let snapshot = path.with_extension("js.snap");
            assert!(
                snapshot.is_file(),
                "missing snapshot for {}",
                path.display()
            );
        }
    }
    assert_eq!(inputs, REFERENCE_DEOBFUSCATE_FIXTURES);
}
