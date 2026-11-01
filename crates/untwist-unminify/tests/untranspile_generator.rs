use untwist_core::pass::{Note, PassCtor, PassOptions};
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::notes::{UNTRANSPILE_ASYNC_KEPT, UNTRANSPILE_MACHINE_KEPT};
use untwist_unminify::untranspile::PASS;
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    let path = root().join("fixtures/untranspile").join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn run_with(source: &str, unsafe_transforms: bool, extended: bool) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let options = PassOptions {
        unsafe_transforms,
        extended,
        ..PassOptions::default()
    };
    let (code, report) = transform_report(&alloc, source, &ctors(), options);
    (code, report.notes)
}

fn unsafe_run(path: &str) -> String {
    run_with(&read(path), true, true).0
}

fn count_notes(source: &str, message: &str) -> usize {
    run_with(source, true, true)
        .1
        .iter()
        .filter(|note| note.pass == PASS && note.message == message)
        .count()
}

fn assert_has(code: &str, parts: &[&str]) {
    for part in parts {
        assert!(code.contains(part), "missing {part:?} in:\n{code}");
    }
}

#[test]
fn tslib_generators_become_generator_functions() {
    let code = unsafe_run("ts/generators.es5.js");
    assert_has(
        &code,
        &[
            "function* counter(n) {",
            "got = yield i;",
            "} finally {",
            "r = yield* counter(1);",
            "} catch (e_1) {",
            "function* labelled() {",
            "function* fib() {",
        ],
    );
    assert!(!code.contains("__generator(this"), "{code}");
}

#[test]
fn a_finally_still_runs_when_the_body_returns() {
    let code = unsafe_run("hand/machine-finally-return.js");
    assert_has(
        &code,
        &[
            "function* gen(flag) {\n  try {",
            "return \"early\";",
            "  } finally {\n    log.push(\"finally\");",
            "function* nested() {",
            "async function asyncReturn(x) {",
        ],
    );
}

#[test]
fn a_yield_inside_a_try_keeps_its_handler() {
    let code = unsafe_run("hand/machine-yield-in-try.js");
    assert_has(
        &code,
        &["function* g() {", "got = yield n;", "} catch (e_1) {"],
    );
    assert!(!code.contains("__generator(this"), "{code}");
}

#[test]
fn a_labelled_break_leaves_the_nested_loop() {
    let code = unsafe_run("hand/machine-labelled.js");
    assert_has(&code, &["L1: while (i < n) {", "break L1;", "continue L1;"]);
}

#[test]
fn a_rethrow_from_a_catch_reaches_the_outer_handler() {
    let code = unsafe_run("hand/machine-rethrow.js");
    assert_has(
        &code,
        &[
            "throw new TypeError(\"again\");",
            "} catch (e_2) {",
            "yield \"outer \" + e_2.constructor.name;",
            "throw e_3;",
        ],
    );
}

#[test]
fn delegation_becomes_yield_star() {
    let code = unsafe_run("hand/machine-delegate.js");
    assert_has(&code, &["r = yield* inner();", "yield* inner();"]);
    assert!(!code.contains("__values("), "{code}");
}

#[test]
fn an_irreducible_machine_is_refused_with_one_note() {
    let source = read("hand/machine-irreducible.js");
    let (code, _) = run_with(&source, true, true);
    assert!(code.contains("return __generator(this"), "{code}");
    assert_eq!(count_notes(&source, UNTRANSPILE_MACHINE_KEPT), 1);
}

#[test]
fn eager_getter_and_block_scoped_machines_are_kept() {
    let source = read("hand/machine-guards.js");
    let (code, _) = run_with(&source, true, true);
    assert!(
        code.contains("log.push(\"called \" + n);\n  return __generator(this"),
        "{code}"
    );
    assert!(
        code.contains("get items() {\n  return __generator(this"),
        "{code}"
    );
    assert!(!code.contains("function* "), "{code}");
    assert_eq!(count_notes(&source, UNTRANSPILE_MACHINE_KEPT), 2);
}

#[test]
fn reads_that_would_cross_a_resumption_bind_the_received_value_first() {
    let source = read("hand/machine-resume-order.js");
    let (code, _) = run_with(&source, true, true);
    assert_has(
        &code,
        &[
            "const _t = yield g = 2;\n  return g + _t;",
            "const _t2 = yield 0;\n  return h + _t2;",
            "const _t3 = yield x = 2;\n  return x + _t3;",
            "const _t4 = yield 0;\n  return n + _t4;",
            "const _t5 = yield 0;\n  return o.v + _t5;",
            "const _t6 = yield 0;\n  total += _t6;",
            "b += a + (yield 0);",
        ],
    );
    assert!(!code.contains("__generator(this"), "{code}");
    assert_eq!(count_notes(&source, UNTRANSPILE_MACHINE_KEPT), 0);
}

#[test]
fn spread_arguments_around_an_await_keep_converting_through_a_temporary() {
    for (path, bound) in [
        ("ts/async.es5.js", "const _t = await "),
        ("swc/async.es5.js", "const _t = yield "),
    ] {
        let code = unsafe_run(path);
        assert!(!code.contains("_generator(this"), "{path}:\n{code}");
        assert!(code.contains(bound), "{path}:\n{code}");
    }
}

#[test]
fn async_helpers_become_async_functions() {
    for path in ["ts/async.es5.js", "ts/async.es2015.js"] {
        let code = unsafe_run(path);
        assert_has(
            &code,
            &[
                "async function one(x) {",
                "y = await x;",
                "async function withFinally() {",
                "await throws();",
            ],
        );
    }
    let code = unsafe_run("esbuild/async.0.28.1-es2015.js");
    assert_has(
        &code,
        &[
            "async function one(x) {",
            "const y = await x;",
            "async m(a) {",
            "async (n) => {",
        ],
    );
}

#[test]
fn shadowing_receivers_and_lengths_keep_the_async_helper() {
    let source = read("hand/async-guards.js");
    let (code, _) = run_with(&source, true, true);
    assert!(!code.contains("async function"), "{code}");
    assert_eq!(count_notes(&source, UNTRANSPILE_ASYNC_KEPT), 3);
}

#[test]
fn the_default_mode_and_conservative_keep_every_machine() {
    let source = read("ts/async.es5.js");
    let (default_mode, _) = run_with(&source, false, true);
    let (conservative, _) = run_with(&source, true, false);
    for code in [default_mode, conservative] {
        assert!(code.contains("return __generator(this"), "{code}");
        assert!(!code.contains("async function"), "{code}");
    }
}

#[test]
fn a_written_promise_keeps_the_async_helpers() {
    let helper = read("esbuild/async.0.28.1-es2015.js")
        .lines()
        .take_while(|line| !line.starts_with("const log"))
        .collect::<Vec<_>>()
        .join("\n");
    let tail = "function f() {\n  return __async(this, null, function* () {\n    yield 1;\n    return 2;\n  });\n}\nPromise.prototype.extra = 1;\nf().then(console.log);\n";
    let source = format!("{helper}\n{tail}");
    let (code, _) = run_with(&source, true, true);
    assert!(
        code.contains("return __async(this, null, function* () {"),
        "{code}"
    );
    let untouched = format!(
        "{helper}\n{}",
        tail.replace("Promise.prototype.extra = 1;\n", "")
    );
    let (converted, _) = run_with(&untouched, true, true);
    assert!(converted.contains("async function f() {"), "{converted}");
}
