use untwist_rename::generate::{
    is_identifier_name, lower_camel, sequential_name, strip_accessor_prefix, title_case,
    to_identifier, uid_candidates,
};

#[test]
fn sequential_names_are_bijective_base_26() {
    let cases = [
        (0, "a"),
        (1, "b"),
        (25, "z"),
        (26, "aa"),
        (27, "ab"),
        (51, "az"),
        (52, "ba"),
        (701, "zz"),
        (702, "aaa"),
    ];
    for (index, expected) in cases {
        assert_eq!(sequential_name(index), expected, "index {index}");
    }
}

#[test]
fn to_identifier_matches_babels_behaviour() {
    let cases = [
        ("fs", "fs"),
        ("node:fs", "nodeFs"),
        ("some-lib-a", "someLibA"),
        ("@scope/pkg", "scopePkg"),
        ("123abc", "abc"),
        ("class", "_class"),
        ("", "_"),
        ("---", "_"),
        ("hello world", "helloWorld"),
        ("café", "caf"),
    ];
    for (input, expected) in cases {
        assert_eq!(to_identifier(input), expected, "input {input:?}");
    }
}

#[test]
fn is_identifier_name_accepts_only_usable_names() {
    for name in ["a", "_private", "$el", "videoRef", "a_b", "fn1", "PI"] {
        assert!(is_identifier_name(name), "{name} must be usable");
    }
    for name in ["", "a-b", "0", "1st", "class", "café", "日本", "a b"] {
        assert!(!is_identifier_name(name), "{name} must not be usable");
    }
}

#[test]
fn title_case_matches_reference() {
    let cases = [
        ("hello world", "HelloWorld"),
        ("foo-bar-🗿-ä", "Foobar"),
        ("", ""),
        ("LN1", "LN1"),
        ("exports", "Exports"),
        ("日本語", ""),
    ];
    for (input, expected) in cases {
        assert_eq!(title_case(input), expected, "input {input:?}");
    }
}

#[test]
fn lower_camel_and_accessor_prefixes() {
    assert_eq!(lower_camel("SomeLibA"), "someLibA");
    assert_eq!(lower_camel(""), "");
    assert_eq!(strip_accessor_prefix("getUser").as_deref(), Some("user"));
    assert_eq!(
        strip_accessor_prefix("createElement").as_deref(),
        Some("element")
    );
    assert_eq!(strip_accessor_prefix("get").as_deref(), None);
    assert_eq!(strip_accessor_prefix("handler").as_deref(), None);
}

#[test]
fn uid_candidates_start_at_the_bare_base() {
    let first: Vec<String> = uid_candidates("p").take(3).collect();
    assert_eq!(first, ["p", "p2", "p3"]);
    assert_eq!(uid_candidates("p").count(), 999);
}
