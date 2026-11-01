#[test]
fn the_shared_helper_reads_a_real_world_fixture_from_the_workspace_root() {
    let Some(source) = untwist_testing::real_world_fixture("game.js") else {
        return;
    };
    assert!(source.len() > 1_000_000, "game.js is a 10.7 MB fixture");
    assert!(
        source.contains("sourceMappingURL"),
        "trailing comment survives"
    );
}

#[test]
fn the_shared_helper_reads_a_reference_fixture_from_the_workspace_root() {
    let source = untwist_testing::read_fixture("deobfuscate", "simple-string-array.js");
    assert!(source.contains("Hello, World!"), "{source}");
}
