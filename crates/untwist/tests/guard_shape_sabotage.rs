use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline, SOURCE_READER_UNGUARDED};
use untwist_core::guard_shape::narrow_guard_shapes_for_tests;
use untwist_eval::source::satisfies_self_defending_regex;
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const COOKIE_ROTATOR: &str = include_str!("data/guards/cookie-rotator.js");
const INLINE_CONTROLLER: &str = include_str!("data/guards/inline-controller.js");
const SEQUENCE_CONTROLLER: &str = include_str!("data/guards/sequence-controller.js");
const READ_KEY: &str = "removeCookie: ";

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

fn read_function(code: &str) -> &str {
    let start = code
        .find(READ_KEY)
        .map_or(code.len(), |at| at + READ_KEY.len());
    let rest = &code[start..];
    let end = rest.find("},").map_or(rest.len(), |at| at + 1);
    &rest[..end]
}

fn printed(source: &str, narrowed: bool) -> String {
    narrow_guard_shapes_for_tests(narrowed);
    let out = Pipeline::new(Options::default()).run(source);
    narrow_guard_shapes_for_tests(false);
    out.code().to_string()
}

fn the_cookie_check_fails_on_its_printed_function_once_the_shape_is_narrowed() {
    let kept = printed(COOKIE_ROTATOR, false);
    let narrowed = printed(COOKIE_ROTATOR, true);
    assert_eq!(read_function(&kept), "function () {return \"dev\";}");
    assert!(satisfies_self_defending_regex(read_function(&kept)));
    assert!(satisfies_self_defending_regex("function(){return'dev';}"));
    assert!(
        !satisfies_self_defending_regex(read_function(&narrowed)),
        "{narrowed}"
    );
}

fn passed_through(source: &str) -> bool {
    narrow_guard_shapes_for_tests(true);
    let out = Pipeline::new(Options::default()).run(source);
    narrow_guard_shapes_for_tests(false);
    let noted = out
        .notes
        .iter()
        .any(|note| note.message == SOURCE_READER_UNGUARDED);
    noted && out.code() == source
}

fn the_controllers_are_passed_through_once_the_shapes_are_narrowed() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    for source in [INLINE_CONTROLLER, SEQUENCE_CONTROLLER] {
        let before = observed(&node, dir.path(), source);
        assert_eq!(before, "start\nend 5\nexit Some(0)");
        let kept = observed(&node, dir.path(), &printed(source, false));
        assert_eq!(kept, before, "{source}");
        assert!(passed_through(source), "{source}");
    }
}

#[test]
fn every_new_guard_shape_turns_red_once_narrowed() {
    the_cookie_check_fails_on_its_printed_function_once_the_shape_is_narrowed();
    the_controllers_are_passed_through_once_the_shapes_are_narrowed();
}
