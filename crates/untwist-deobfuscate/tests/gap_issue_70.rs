mod common;

#[test]
fn issue_70_every_unit_in_a_multi_unit_bundle_decodes_in_one_round() {
    let source = untwist_testing::read_gap_fixture("two-string-array-units.js");
    let (code, report) = common::run_report(&source);
    let notes: Vec<String> = report
        .notes
        .iter()
        .map(|note| note.message.clone())
        .collect();

    assert!(code.contains(r#"console["log"]("first")"#), "{code}");
    assert!(code.contains(r#"console["warn"]("second")"#), "{code}");

    for name in ["a1", "b1", "a2", "b2"] {
        assert!(!code.contains(name), "{name} survived\n{code}");
    }
    assert!(
        !notes
            .iter()
            .any(|note| note.contains("additional string array ignored")),
        "{notes:?}"
    );
    assert_eq!(report.iterations, 2, "{:?}", report.per_pass);
}
