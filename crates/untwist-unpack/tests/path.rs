use untwist_unpack::path::{
    is_unsafe_path, join_posix, module_id_path, normalize_posix, PosixPath,
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

#[test]
fn control_characters_make_a_path_unsafe() {
    for path in [
        "a\u{0}b.js",
        "a\u{1b}[31mRED.js",
        "a\nb.js",
        "a\rb.js",
        "a\u{7f}.js",
        "a\u{85}.js",
        "a\u{9b}.js",
        "a\u{2028}.js",
        "a\u{2029}.js",
        "dir\tname/a.js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn characters_windows_reserves_make_a_path_unsafe() {
    for path in [
        "a.js:stream",
        "a<b.js",
        "a>b.js",
        "external \"react\".js",
        "a|b.js",
        "a.js?vue&type=script.js",
        "a*.js",
        "sub\\a.js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn a_segment_ending_in_a_dot_or_a_space_is_unsafe() {
    for path in ["a./b.js", "a /b.js", "dir/name.", "dir/name ", ".../a.js"] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn windows_device_names_are_unsafe_in_any_case_and_with_any_extension() {
    for path in [
        "CON",
        "con.js",
        "Prn.js",
        "AUX.tar.gz",
        "nul",
        "dir/NUL.js",
        "COM1.js",
        "com9",
        "LPT1.js",
        "lpt9.txt",
        "aux/a.js",
        "CON .js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
    for path in [
        "console.js",
        "com10.js",
        "com0.js",
        "lpt.js",
        "auxiliary/a.js",
    ] {
        assert!(!is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn tool_directories_are_unsafe_anywhere_in_a_path() {
    for path in [
        ".git/hooks/x.js",
        "a/.git/config.js",
        ".GIT/hooks/x.js",
        ".hg/a.js",
        ".svn/a.js",
        ".vscode/settings.js",
        ".idea/a.js",
        ".husky/pre-commit.js",
        ".github/workflows/a.js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn a_file_name_starting_with_a_dot_is_unsafe_but_a_directory_is_not() {
    for path in [".eslintrc.js", "src/.env.js", ".npmrc"] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
    for path in [
        "node_modules/.pnpm/a@1.0.0/node_modules/a/index.js",
        ".cache/a.js",
        "eslint.config.js",
        "package.json.js",
        "@scope/pkg/index.js",
        "a b.js",
    ] {
        assert!(!is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn a_segment_longer_than_a_file_name_may_be_is_unsafe() {
    let fits = format!("{}.js", "a".repeat(252));
    let too_long = format!("{}.js", "a".repeat(253));
    assert!(!is_unsafe_path(&fits));
    assert!(is_unsafe_path(&too_long));
    assert!(is_unsafe_path(&format!("{}/a.js", "d".repeat(256))));
    assert!(is_unsafe_path(&format!("{}.js", "é".repeat(127))));
}

#[test]
fn a_path_longer_than_the_whole_path_limit_is_unsafe() {
    let fits = format!("{}a.js", "a/".repeat(500));
    let too_long = format!("{}a.js", "a/".repeat(520));
    assert!(!is_unsafe_path(&fits));
    assert!(is_unsafe_path(&too_long));
}

#[test]
fn the_fallback_name_of_a_very_long_id_still_fits_in_a_file_name() {
    let id = "x".repeat(10_000);
    let path = untwist_unpack::model::fallback_path(&id);
    assert!(path.as_str().starts_with("_unsafe_x"), "{}", path.as_str());
    assert!(path.as_str().len() <= 240, "{}", path.as_str().len());
    assert!(!is_unsafe_path(path.as_str()));
}

#[test]
fn the_names_the_tool_generates_are_safe() {
    for path in [
        untwist_unpack::path::ENTRY_PATH.to_string(),
        untwist_unpack::model::fallback_path("./a\u{1b}[31m")
            .as_str()
            .to_string(),
        untwist_unpack::model::fallback_path("")
            .as_str()
            .to_string(),
        untwist_unpack::model::fallback_path("CON")
            .as_str()
            .to_string(),
    ] {
        assert!(!is_unsafe_path(&path), "{path:?}");
    }
}

#[test]
fn the_other_win32_device_names_are_unsafe() {
    for path in [
        "COM\u{b9}.js",
        "com\u{b2}",
        "Com\u{b3}.txt",
        "LPT\u{b9}.js",
        "lpt\u{b2}.js",
        "lpt\u{b3}",
        "CONIN$",
        "conin$.js",
        "CONOUT$.js",
        "dir/conout$.js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
    for path in ["com\u{b9}x.js", "conin.js", "conout.js"] {
        assert!(!is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn a_segment_shaped_like_an_8_3_short_name_is_unsafe() {
    for path in [
        "DEOBFU~1",
        "DEOBFU~1.js",
        "longfi~1.js",
        "sub/INDEX~12.js",
        "git~1/hooks/pre-commit.js",
        "x~1.min.js",
    ] {
        assert!(is_unsafe_path(path), "{path:?}");
    }
    for path in ["a~b.js", "x~.js", "~1x.js", "tilde~/a.js", "a.js~1"] {
        assert!(!is_unsafe_path(path), "{path:?}");
    }
}

#[test]
fn bidirectional_controls_make_a_path_unsafe() {
    for ch in ('\u{202a}'..='\u{202e}').chain('\u{2066}'..='\u{2069}') {
        let path = format!("x{ch}gnp.js");
        assert!(is_unsafe_path(&path), "{path:?}");
        let dir = format!("d{ch}/a.js");
        assert!(is_unsafe_path(&dir), "{dir:?}");
    }
    assert!(!is_unsafe_path("x\u{e9}\u{2065}.js"));
}
