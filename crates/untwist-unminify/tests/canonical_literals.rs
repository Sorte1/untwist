use untwist_core::parse::parse_program;
use untwist_core::pass::PassOptions;
use untwist_testing::{transform_report, transform_with, Allocator};
use untwist_unminify::unminify_ctors;

#[test]
fn the_printer_canonicalises_literals_without_any_pass() {
    let alloc = Allocator::default();
    let out = transform_with(
        &alloc,
        "var a = 0x10; var b = 'x';",
        &unminify_ctors(),
        PassOptions::default(),
    );
    assert_eq!(out, "var a = 16;\nvar b = \"x\";");
}

#[test]
fn already_canonical_literals_report_no_changes() {
    let alloc = Allocator::default();
    let (out, report) = transform_report(
        &alloc,
        "var a = 16;\nvar b = \"x\";",
        &unminify_ctors(),
        PassOptions::default(),
    );
    assert_eq!(out, "var a = 16;\nvar b = \"x\";");
    assert_eq!(report.total_changes, 0);
    assert_eq!(report.iterations, 1);
}

#[test]
fn a_lone_surrogate_escape_round_trips_through_the_printer() {
    let alloc = Allocator::default();
    let (out, _report) = transform_report(
        &alloc,
        r#"x = "\uD800";"#,
        &unminify_ctors(),
        PassOptions::default(),
    );
    let reparsed = parse_program(&alloc, &out);
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:?}",
        reparsed.diagnostics
    );
    let (_, second) = transform_report(&alloc, &out, &unminify_ctors(), PassOptions::default());
    assert_eq!(second.total_changes, 0);
    assert_eq!(second.iterations, 1);
}

#[test]
fn line_and_paragraph_separators_round_trip_through_the_printer() {
    let alloc = Allocator::default();
    let (out, _report) = transform_report(
        &alloc,
        "f(\"\\u2028\\u2029\\t\")",
        &unminify_ctors(),
        PassOptions::default(),
    );
    let reparsed = parse_program(&alloc, &out);
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:?}",
        reparsed.diagnostics
    );
    let (_, second) = transform_report(&alloc, &out, &unminify_ctors(), PassOptions::default());
    assert_eq!(second.total_changes, 0);
    assert_eq!(second.iterations, 1);
}
