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
fn babel_iterator_helper_loops_become_for_of() {
    let code = run("matrix/babel-7.29/ie11/for-of.js", true);
    assert_has(
        &code,
        &[
            " of \"héllo\")",
            " of new Map([",
            "of custom) {",
            "of arguments) {",
            "continue outer;",
        ],
    );
    assert!(!code.contains("_createForOfIteratorHelper"), "{code}");
}

#[test]
fn early_exits_keep_the_loop_body() {
    let code = run("matrix/babel-7.29/ie11/for-of-exits.js", true);
    assert_has(
        &code,
        &[
            "for (last of counted(\"assign\", [7, 8]))",
            " of counted(\"brk\", [1, 2, 3])) {",
            "    break;",
            "throw new Error(\"boom \" + v);",
        ],
    );
    assert!(!code.contains(".s();"), "{code}");
}

#[test]
fn bundled_and_minified_copies_are_recognised() {
    for path in [
        "matrix/babel-7.29/ie11-rollup-terser/for-of-exits.js",
        "matrix/babel-7.29/ie11-terser/for-of-exits.js",
        "matrix/babel-7.28/ie11-runtime-iife-terser/for-of.js",
        "matrix/babel-8/ie11-runtime/for-of-exits.js",
        "matrix/babel-7.12/ie11/for-of-exits.js",
    ] {
        let code = run(path, true);
        assert!(!code.contains(".s();"), "{path}:\n{code}");
    }
}

#[test]
fn the_default_lane_keeps_the_helper() {
    let code = run("matrix/babel-7.29/ie11/for-of.js", false);
    assert_has(&code, &["_createForOfIteratorHelper(\"héllo\")", ".s();"]);
}

#[test]
fn the_loose_helper_is_kept() {
    let code = run("matrix/babel-7.29/ie11-loose/for-of-exits.js", true);
    assert!(code.contains("_createForOfIteratorHelperLoose("), "{code}");
}

#[test]
fn guarded_loops_keep_the_helper() {
    let code = run("hand/for-of-helper-guards.js", true);
    assert_has(
        &code,
        &[
            "_createForOfIteratorHelper(items, true);",
            "_createForOfIteratorHelper(row);",
            "log.push(_step3.value, _step3.done);",
            "log.push(typeof _iterator4.s);",
            " of [7, 8]) {",
        ],
    );
}

#[test]
fn typescript_values_loops_become_for_of() {
    let code = run("matrix/ts-3.9/es5-iter/for-of.js", true);
    assert_has(
        &code,
        &[
            "of \"héllo\")",
            "of custom) {",
            "of arguments) {",
            "outer: for (",
        ],
    );
    assert!(!code.contains(".next(); !"), "{code}");
}

#[test]
fn typescript_values_loops_stay_in_the_default_lane() {
    let code = run("matrix/ts-3.9/es5-iter/for-of.js", false);
    assert!(code.contains("__values(\"héllo\")"), "{code}");
}

#[test]
fn inline_protocol_loops_become_for_of() {
    for path in [
        "matrix/swc-1.16/es5/for-of-exits.js",
        "matrix/babel6-6/ie11/for-of.js",
    ] {
        let code = run(path, true);
        assert!(
            !code.contains("_iteratorNormalCompletion"),
            "{path}:\n{code}"
        );
    }
    let code = run("matrix/swc-1.16/es5/for-of-exits.js", false);
    assert!(code.contains("_iteratorNormalCompletion"), "{code}");
}

#[test]
fn a_loop_closure_is_inlined_with_per_iteration_bindings() {
    let code = run("matrix/babel-7.24/ie11/for-of-closures.js", true);
    assert!(!code.contains("_loop"), "{code}");
    assert_has(
        &code,
        &[" of new Set([1, 2, 3])) {", "let note = \"n\" + v;"],
    );
}
