use untwist_unpack::path::{
    is_unsafe_path, join_posix, module_id_path, normalize_posix, relative_path, PosixPath,
};

#[test]
fn normalizes_dot_segments() {
    assert_eq!(normalize_posix("./a.js"), "a.js");
    assert_eq!(normalize_posix("a/./b.js"), "a/b.js");
    assert_eq!(normalize_posix("a/b/../c.js"), "a/c.js");
    assert_eq!(normalize_posix("a//b.js"), "a/b.js");
    assert_eq!(normalize_posix("././../tmp.js"), "../tmp.js");
    assert_eq!(normalize_posix("../../c.js"), "../../c.js");
    assert_eq!(normalize_posix(""), "");
}

#[test]
fn relative_paths_match_reference() {
    assert_eq!(relative_path("a.js", "x/y.js"), "./x/y.js");
    assert_eq!(relative_path("x/y.js", "a.js"), "../a.js");
    assert_eq!(relative_path("a.js", "node_modules/lib"), "lib");
    assert_eq!(relative_path("utils/color.js", "1.js"), "../1.js");
    assert_eq!(relative_path("index.js", "1.js"), "./1.js");
}

#[test]
fn join_matches_posix_join() {
    assert_eq!(join_posix(".", "./utils"), "utils");
    assert_eq!(join_posix("", "./index.js"), "index.js");
    assert_eq!(join_posix("tmp0/tmp1", "../bar/b.js"), "tmp0/bar/b.js");
    assert_eq!(join_posix("tmp0/tmp1", "../../c.js"), "c.js");
    assert_eq!(
        join_posix("node_modules/uuid", "./v1"),
        "node_modules/uuid/v1"
    );
}

#[test]
fn unsafe_paths_are_rejected() {
    assert!(is_unsafe_path("../tmp.js"));
    assert!(is_unsafe_path("/etc/passwd"));
    assert!(is_unsafe_path("C:/windows/system32"));
    assert!(is_unsafe_path("\\..\\node_modules\\debug\\src\\index.js"));
    assert!(is_unsafe_path(""));
    assert!(!is_unsafe_path("a/b.js"));
    assert!(!is_unsafe_path("node_modules/lib/index.js"));
}

#[test]
fn module_id_paths_match_reference() {
    assert_eq!(
        PosixPath::from_module_id("3", false).map(|p| p.as_str().to_string()),
        Some("3.js".to_string())
    );
    assert_eq!(
        PosixPath::from_module_id("3", true).map(|p| p.as_str().to_string()),
        Some("index.js".to_string())
    );
    assert_eq!(
        PosixPath::from_module_id("2wwy.js", false).map(|p| p.as_str().to_string()),
        Some("2wwy.js".to_string())
    );
    assert_eq!(PosixPath::from_module_id("../tmp.js", false), None);
    assert_eq!(
        PosixPath::from_module_id("./\\..\\node_modules\\debug\\src\\index", false),
        None
    );
}

#[test]
fn module_id_path_produces_the_string_from_module_id_refuses() {
    assert_eq!(module_id_path("../evil", false), "../evil.js");
    assert_eq!(PosixPath::from_module_id("../evil", false), None);
}

#[test]
fn dirname_and_stem_suffix() {
    let path = PosixPath::parse("a/b/c.js").expect("safe");
    assert_eq!(path.dirname(), "a/b");
    assert_eq!(path.with_stem_suffix("-2").as_str(), "a/b/c-2.js");
    let root = PosixPath::parse("index.js").expect("safe");
    assert_eq!(root.dirname(), "");
    assert!(!root.is_node_modules());
    assert!(PosixPath::parse("node_modules/lib/index.js")
        .expect("safe")
        .is_node_modules());
}
