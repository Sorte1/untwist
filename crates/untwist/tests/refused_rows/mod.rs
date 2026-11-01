use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const STAMPS: [&str; 2] = ["__STRING_ARRAY_", "__DECODE_"];

pub struct Row {
    pub source: &'static str,
    pub prints: &'static str,
    pub kept: &'static str,
}

fn modes() -> [Options; 3] {
    [
        Options::default(),
        Options {
            extended: false,
            ..Options::default()
        },
        Options {
            extended: false,
            unsafe_transforms: true,
            ..Options::default()
        },
    ]
}

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

fn printed(node: &Path, text: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("program.js");
    std::fs::write(&path, text).unwrap();
    run(node, &path)
        .unwrap()
        .map(|out| out.stdout)
        .unwrap_or_default()
}

const NEWER_UNICODE_NOTE: &str = "note: this node's Unicode tables are newer than the row assumes, so it accepts the construct; the node-side assertion compares the output with the input instead";

fn expected_prints(node: &Path, row: &Row, tolerate_newer_unicode: bool) -> String {
    let actual = printed(node, row.source);
    if tolerate_newer_unicode && actual != row.prints {
        eprintln!("{NEWER_UNICODE_NOTE}");
        return actual;
    }
    assert_eq!(actual, row.prints, "node on the input");
    actual
}

pub fn check_with(row: &Row, tolerate_newer_unicode: bool) {
    let node = node_or_skip();
    let expected = node
        .as_ref()
        .map(|node| expected_prints(node, row, tolerate_newer_unicode));
    for options in modes() {
        let code = Pipeline::new(options).run(row.source).code().to_string();
        for stamp in STAMPS {
            assert!(!code.contains(stamp), "a stamp name is left: {code}");
        }
        assert!(code.contains(row.kept), "the call must stay: {code}");
        if let (Some(node), Some(expected)) = (&node, &expected) {
            assert_eq!(
                &printed(node, &code),
                expected,
                "node on the output: {code}"
            );
        }
    }
}

pub fn check(row: &Row) {
    check_with(row, false);
}
