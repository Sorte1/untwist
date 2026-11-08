use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_core::guard_shape::narrow_guard_shapes_for_tests;
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const ROWS: [(&str, &str); 4] = [
    (
        "assigned holder",
        include_str!("data/guards/minified-assigned-holder.js"),
    ),
    (
        "flagged holder",
        include_str!("data/guards/minified-flagged-holder.js"),
    ),
    (
        "block holder",
        include_str!("data/guards/minified-block-holder.js"),
    ),
    (
        "block holder sharing a var",
        include_str!("data/guards/minified-block-holder-shared-var.js"),
    ),
];

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn observed(node: &Path, dir: &Path, text: &str) -> String {
    let path = dir.join("program.js");
    std::fs::write(&path, text).unwrap();
    match run(node, &path).unwrap() {
        Some(out) => format!("{}exit {:?}", out.stdout, out.exit),
        None => "did not finish".to_string(),
    }
}

fn printed(source: &str, unsafe_transforms: bool) -> String {
    let options = Options {
        unsafe_transforms,
        ..Options::default()
    };
    Pipeline::new(options).run(source).code().to_string()
}

#[test]
fn refused_minified_cookie_guards_keep_working_and_break_once_narrowed() {
    for (label, source) in ROWS {
        let code = printed(source, false);
        assert!(
            code.contains("removeCookie: function () {return \"dev\";}"),
            "{label}:\n{code}"
        );
    }
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for (label, source) in ROWS {
        let before = observed(&node, dir.path(), source);
        for lane in [false, true] {
            let after = observed(&node, dir.path(), &printed(source, lane));
            assert_eq!(after, before, "{label} (unsafe={lane})");
        }
        narrow_guard_shapes_for_tests(true);
        let narrowed = printed(source, false);
        narrow_guard_shapes_for_tests(false);
        let broken = observed(&node, dir.path(), &narrowed);
        assert_ne!(
            broken, before,
            "{label} still runs once the shape is narrowed"
        );
    }
}
