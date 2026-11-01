use std::path::Path;

use untwist_testing::node::normalise_stderr;

#[test]
fn two_crash_banners_that_differ_only_in_path_and_frames_normalise_to_the_same_message() {
    let banner_a = "/tmp/a/file1.js:10\n  someCode();\n  ^\n\nReferenceError: foo is not defined\n    at hi (/tmp/a/file1.js:10:5)\n    at Object.<anonymous> (/tmp/a/file1.js:20:1)\n\nNode.js v22.15.0\n";
    let banner_b = "/tmp/b/other-file2.js:99\n  differentCode();\n  ^\n\nReferenceError: foo is not defined\n    at wrapper (/tmp/b/other-file2.js:99:7)\n    at Object.<anonymous> (/tmp/b/other-file2.js:150:3)\n    at Module._compile (node:internal/modules/cjs/loader:1730:14)\n\nNode.js v22.15.0\n";
    let normalised_a = normalise_stderr(banner_a, &[Path::new("/tmp/a/file1.js")]);
    let normalised_b = normalise_stderr(banner_b, &[Path::new("/tmp/b/other-file2.js")]);
    assert_eq!(normalised_a, "ReferenceError: foo is not defined");
    assert_eq!(normalised_a, normalised_b);
}

#[test]
fn different_error_messages_stay_different_after_normalisation() {
    let type_error = "/tmp/x.js:4\n    f = b();\n        ^\n\nTypeError: b is not a function\n    at Object.<anonymous> (/tmp/x.js:4:9)\n\nNode.js v22.15.0\n";
    let reference_error = "/tmp/y.js:8\n  return m.foo();\n         ^\n\nReferenceError: m is not defined\n    at hi (/tmp/y.js:8:3)\n\nNode.js v22.15.0\n";
    let normalised_type_error = normalise_stderr(type_error, &[Path::new("/tmp/x.js")]);
    let normalised_reference_error = normalise_stderr(reference_error, &[Path::new("/tmp/y.js")]);
    assert_eq!(normalised_type_error, "TypeError: b is not a function");
    assert_eq!(
        normalised_reference_error,
        "ReferenceError: m is not defined"
    );
    assert_ne!(normalised_type_error, normalised_reference_error);
}

#[test]
fn console_error_output_survives_untouched() {
    assert_eq!(normalise_stderr("warning: x\n", &[]), "warning: x");
}

#[test]
fn the_nodejs_version_trailer_is_dropped() {
    let text = "SomeError: x\n\nNode.js v22.15.0\n";
    assert_eq!(normalise_stderr(text, &[]), "SomeError: x");
}

#[test]
fn a_canonicalised_path_matches_the_absolute_path_node_would_print() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("x.js");
    std::fs::write(&file, "").expect("write fixture");
    let canonical = file.canonicalize().expect("canonicalize");
    let banner = format!(
        "{}:3\nsomeCode();\n^\n\nError: oops\n    at f ({}:3:1)\n\nNode.js v22.15.0\n",
        canonical.display(),
        canonical.display()
    );
    let normalised = normalise_stderr(&banner, &[canonical.as_path()]);
    assert_eq!(normalised, "Error: oops");
}

#[test]
fn a_console_error_line_starting_with_at_survives_but_a_frame_line_is_dropped() {
    assert_eq!(normalise_stderr("at the start\n", &[]), "at the start");
    let text = "Error: x\n    at foo (<file>:1:1)\n";
    assert_eq!(normalise_stderr(text, &[]), "Error: x");
}

#[test]
fn a_banner_with_no_blank_line_and_no_frames_normalises_to_just_the_message() {
    let text = "<file>:3\nsomeCode();\n^\nError: oops\n";
    assert_eq!(normalise_stderr(text, &[]), "Error: oops");
}
