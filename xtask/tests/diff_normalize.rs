use xtask::diff::normalize;

#[test]
fn normalize_ignores_formatting_but_not_semantics() {
    assert_eq!(
        normalize("var a = true;\nx();"),
        normalize("var a=true;x()")
    );
    assert_ne!(normalize("var a = true;"), normalize("var a = false;"));
}

#[test]
fn module_paths_compare_without_the_dot_slash_prefix() {
    assert_eq!(xtask::diff::normalize_module_path("./3.js"), "3.js");
    assert_eq!(xtask::diff::normalize_module_path("3.js"), "3.js");
    assert_eq!(
        xtask::diff::normalize_module_path("node_modules/lib/index.js"),
        "node_modules/lib/index.js"
    );
    assert_eq!(xtask::diff::normalize_module_path("./a/./b.js"), "a/./b.js");
}

#[test]
fn a_missing_bundle_json_yields_a_failing_outcome_naming_the_fixture() {
    let expected_dir = tempfile::tempdir().unwrap();
    let actual_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        actual_dir.path().join("bundle.json"),
        r#"{"type":"webpack","entryId":null,"modules":[]}"#,
    )
    .unwrap();
    let fixture = "fixtures/reference/unpack/example.js";
    let outcome =
        xtask::diff::outcome_for_dirs(fixture, expected_dir.path(), actual_dir.path(), false)
            .unwrap();
    match outcome {
        xtask::diff::Outcome::Failing(report) => assert!(report.contains(fixture)),
        _ => panic!("expected a Failing outcome when one side has no bundle.json"),
    }
}

#[test]
fn entry_id_sentinel_and_module_paths_normalise_to_identical() {
    let expected_dir = tempfile::tempdir().unwrap();
    let actual_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        expected_dir.path().join("bundle.json"),
        r#"{"type":"webpack","entryId":"","modules":[{"id":"0","path":"./index.js"}]}"#,
    )
    .unwrap();
    std::fs::write(expected_dir.path().join("index.js"), "1;").unwrap();
    std::fs::write(
        actual_dir.path().join("bundle.json"),
        r#"{"type":"webpack","entryId":null,"modules":[{"id":"0","path":"index.js","isEntry":false}]}"#,
    )
    .unwrap();
    std::fs::write(actual_dir.path().join("index.js"), "1;").unwrap();
    let outcome = xtask::diff::outcome_for_dirs(
        "fixtures/reference/unpack/example.js",
        expected_dir.path(),
        actual_dir.path(),
        false,
    )
    .unwrap();
    assert!(matches!(outcome, xtask::diff::Outcome::Identical));

    let stale = xtask::diff::outcome_for_dirs(
        "fixtures/reference/unpack/example.js",
        expected_dir.path(),
        actual_dir.path(),
        true,
    )
    .unwrap();
    assert!(
        matches!(stale, xtask::diff::Outcome::StaleAllowlist),
        "an allowlisted fixture that matches again must surface its dead allowlist line"
    );
}
