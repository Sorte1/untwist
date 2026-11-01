use untwist_rename::import_path::import_base;

#[test]
fn the_issue_61_sample_produces_the_requested_names() {
    assert_eq!(
        import_base("/lib/some-lib-a/index.js").as_deref(),
        Some("lib_someLibA")
    );
    assert_eq!(
        import_base("/lib/some-lib-b/dist/index.js").as_deref(),
        Some("lib_someLibB")
    );
    assert_eq!(
        import_base("/lib/some-lib-c/dist/modules/some-module.js").as_deref(),
        Some("lib_someModule")
    );
}

#[test]
fn bare_and_scoped_specifiers() {
    assert_eq!(import_base("react").as_deref(), Some("react"));
    assert_eq!(import_base("node:fs").as_deref(), Some("nodeFs"));
    assert_eq!(import_base("@scope/pkg").as_deref(), Some("scope_pkg"));
    assert_eq!(
        import_base("lodash/fp/merge").as_deref(),
        Some("lodash_merge")
    );
}

#[test]
fn relative_specifiers_drop_their_dots() {
    assert_eq!(import_base("./utils").as_deref(), Some("utils"));
    assert_eq!(
        import_base("../../shared/http/client.js").as_deref(),
        Some("shared_client")
    );
    assert_eq!(import_base("./index.js").as_deref(), Some("index"));
    assert_eq!(import_base("./lib/lib.js").as_deref(), Some("lib"));
}

#[test]
fn segments_that_differ_only_before_sanitising_are_not_repeated() {
    assert_eq!(import_base("a-b/aB").as_deref(), Some("aB"));
    assert_eq!(import_base("some-lib/someLib").as_deref(), Some("someLib"));
    assert_eq!(import_base("http/client").as_deref(), Some("http_client"));
}

#[test]
fn query_strings_and_empty_specifiers() {
    assert_eq!(import_base("./worker.js?worker").as_deref(), Some("worker"));
    assert_eq!(import_base("").as_deref(), None);
    assert_eq!(import_base("///").as_deref(), None);
}
