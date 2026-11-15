#![cfg_attr(feature = "realm-premise", allow(dead_code))]

use untwist_types::{annotate, Mode};

const HELPERS: [&str; 3] = ["patch", "peek", "run"];

fn probes() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/a40r4");
    let mut found = Vec::new();
    for dir in std::fs::read_dir(&root).unwrap() {
        let dir = dir.unwrap().path();
        for file in std::fs::read_dir(&dir).unwrap() {
            let path = file.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if HELPERS.iter().any(|helper| name.starts_with(helper)) {
                continue;
            }
            let label = format!("{}/{name}", dir.file_name().unwrap().to_string_lossy());
            found.push((label, std::fs::read_to_string(&path).unwrap()));
        }
    }
    found.sort();
    found
}

const BUILT_IN_CALLS: [&str; 4] = ["Math.random(", "parseInt(", "Math.floor(", ".trim("];
const RESULT_TYPES: [&str; 4] = [": number", "{number}", ": string", "{string}"];

#[cfg(not(feature = "realm-premise"))]
#[test]
fn no_built_in_result_row_is_written_in_either_mode() {
    let mut failures = Vec::new();
    for (name, source) in probes() {
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            let typed = out.lines().any(|line| {
                BUILT_IN_CALLS.iter().any(|call| line.contains(call))
                    && RESULT_TYPES.iter().any(|ty| line.contains(ty))
            });
            if typed {
                failures.push(format!("{name} {mode:?}\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[cfg(not(feature = "realm-premise"))]
#[test]
fn a_sloppy_function_never_takes_types_from_its_callers() {
    let source =
        "(function () {\n  function f(a) {\n    return a;\n  }\n  console.log(f(1));\n})();\n";
    for mode in [Mode::Jsdoc, Mode::Ts] {
        let out = annotate(source, mode).code;
        assert!(
            !out.contains("a: 1") && !out.contains("{1} a"),
            "{mode:?}\n{out}"
        );
    }
}

#[test]
fn a_strict_function_keeps_its_closed_call_types_in_a_script_and_a_module() {
    let script = "(function () {\n  \"use strict\";\n  function f(a) {\n    return a;\n  }\n  console.log(f(1));\n})();\n";
    let module = "function f(a) {\n  return a;\n}\nconsole.log(f(1));\nexport {};\n";
    for source in [script, module] {
        assert!(annotate(source, Mode::Ts)
            .code
            .contains("function f(a: 1): 1"));
        assert!(annotate(source, Mode::Jsdoc).code.contains("@param {1} a"));
    }
}

#[test]
fn an_async_function_keeps_its_promise_return_type() {
    let source = "\"use strict\";\nasync function f() {\n  return 1;\n}\nf().then(console.log);\n";
    let out = annotate(source, Mode::Ts).code;
    assert!(out.contains("async function f(): Promise<number>"), "{out}");
}

#[test]
fn free_commonjs_names_are_never_quiet_outside_a_proven_wrapper() {
    let user = "exports.x = 1;\n";
    let alone = untwist_types::realm(&[user]);
    assert!(!alone.builtins && !alone.self_contained, "{alone:?}");
}
