use untwist_core::pass::{PassCtor, PassOptions};
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

fn read(path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/untranspile")
        .join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn run(path: &str, unsafe_transforms: bool) -> String {
    let alloc = Allocator::default();
    let options = PassOptions {
        unsafe_transforms,
        extended: true,
        ..PassOptions::default()
    };
    transform_report(&alloc, &read(path), &ctors(), options).0
}

fn assert_has(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(code.contains(part), "missing {part:?} in:\n{code}");
    }
}

#[test]
fn babel_regenerator_generators_become_generator_functions() {
    let code = run("matrix/babel-7.29/ie11/generators.js", true);
    assert_has(
        &code,
        &[
            "function* inner() {",
            "got = yield \"first\";",
            "return \"inner-done\";",
            "function* fib() {",
            "yield a;",
        ],
    );
    assert!(!code.contains("_regenerator().m(inner)"), "{code}");
    assert!(!code.contains("_regenerator().m(fib)"), "{code}");
}

#[test]
fn minified_and_older_runtimes_are_recognised() {
    for path in [
        "matrix/babel-7.29/ie11-terser/generators.js",
        "matrix/babel-7.28/ie11/generators.js",
        "matrix/babel-8/ie11/generators.js",
    ] {
        let code = run(path, true);
        assert!(code.contains("function* "), "{path}:\n{code}");
    }
}

#[test]
fn the_default_lane_keeps_the_machine() {
    let code = run("matrix/babel-7.29/ie11/generators.js", false);
    assert!(code.contains("_regenerator().m(inner)"), "{code}");
    assert!(!code.contains("function* inner"), "{code}");
}

#[test]
fn handlers_and_delegation_are_read_and_an_entered_finally_is_kept() {
    let code = run("matrix/babel-7.29/ie11/generators.js", true);
    assert_has(
        &code,
        &[
            "function* thrower() {",
            "} catch (_t) {",
            "_regenerator().m(outer)",
        ],
    );
}

#[test]
fn a_marker_read_elsewhere_keeps_its_generator() {
    let code = run("hand/regenerator-guards.js", true);
    assert_has(&code, &["_regenerator().m(seen)", "function* plain() {"]);
}

#[test]
fn babel_async_wrappers_become_async_functions() {
    let code = run("matrix/babel-7.29/ie11/async-await.js", true);
    assert_has(
        &code,
        &[
            "async function fetchTwice(x) {",
            "a = await later(x);",
            "total: async function total(n) {",
            "add = async function add(k) {",
            "async function main() {",
        ],
    );
    assert!(!code.contains("_fetchTwice"), "{code}");
}

#[test]
fn async_handlers_become_try_statements() {
    let code = run("matrix/babel-7.29/ie11/async-await.js", true);
    assert_has(
        &code,
        &[
            "async function guarded() {",
            "} catch (_t2) {",
            "} finally {",
        ],
    );
}

#[test]
fn the_classic_runtime_machines_convert_too() {
    let code = run("matrix/babel-7.24/ie11/generators.js", true);
    assert_has(
        &code,
        &[
            "function* inner() {",
            "got = yield \"first\";",
            "return \"inner-done\";",
        ],
    );
    let code = run("matrix/babel-7.20/ie11/async-await.js", true);
    assert_has(&code, &["async function fetchTwice(x) {"]);
}
