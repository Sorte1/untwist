use untwist_types::{annotate, annotate_in, realm, Mode};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes/a40r2")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

fn names(prefix: &str) -> Vec<String> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/a40r2");
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            entry
                .ok()
                .map(|e| e.file_name().to_string_lossy().into_owned())
        })
        .filter(|name| name.starts_with(prefix))
        .collect();
    found.sort();
    found
}

fn typed_builtins(out: &str) -> bool {
    out.contains("r: number")
        || out.contains("p: number")
        || out.contains("{number} */ const r")
        || out.contains("{number} */ const p")
}

#[test]
fn every_loading_route_turns_the_built_in_rows_off_in_both_modes() {
    let routes = names("q-");
    assert!(routes.len() > 20);
    let mut failures = Vec::new();
    for name in routes {
        let source = probe(&name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            if typed_builtins(&out) {
                failures.push(format!("{name} {mode:?}: built-in rows stayed on\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn the_older_loading_routes_stay_off_in_jsdoc_mode_too() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/a40r");
    let mut failures = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("ld-") {
            continue;
        }
        let source = std::fs::read_to_string(entry.path()).unwrap();
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            if typed_builtins(&out) {
                failures.push(format!("{name} {mode:?}: built-in rows stayed on\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_script_that_declares_a_built_in_name_turns_the_rows_off_for_the_run() {
    for (declares, user) in [("w2-b.js", "w2-a.js"), ("w4-b.js", "w4-a.js")] {
        let (declares, user) = (probe(declares), probe(user));
        let shared = realm(&[&declares, &user]);
        assert!(!shared.builtins);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate_in(&user, mode, Some(shared)).code;
            assert!(
                !out.contains(": number") && !out.contains("{number}"),
                "{out}"
            );
        }
    }
}

#[test]
fn a_with_in_another_input_closes_no_function() {
    let (helper, user) = (probe("w1-b.js"), probe("w1-a.js"));
    let shared = realm(&[&helper, &user]);
    assert!(!shared.closed_calls);
    for mode in [Mode::Jsdoc, Mode::Ts] {
        let out = annotate_in(&user, mode, Some(shared)).code;
        assert!(!out.contains("a: 1") && !out.contains("{1} a"), "{out}");
    }
}

#[test]
fn code_from_outside_the_run_keeps_closed_calls_only_for_strict_functions() {
    let sloppy = probe("q-f1.js");
    let out = annotate(&sloppy, Mode::Jsdoc).code;
    assert!(
        !out.contains("{1} a") && !out.contains("{1} */ const r"),
        "{out}"
    );
    let strict = "const peek = require(\"./peek.js\");\n(function () {\n  \"use strict\";\n  function f(a) {\n    peek();\n    return a * 2;\n  }\n  console.log(f(1));\n})();\n";
    let out = annotate(strict, Mode::Jsdoc).code;
    assert!(out.contains("@param {1} a"), "{out}");
}

#[test]
fn a_var_named_arguments_is_never_typed() {
    for name in ["va.js", "vb.js"] {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            assert!(!out.contains("arguments: "), "{name} {mode:?}\n{out}");
            assert!(!out.contains("*/ var arguments"), "{name} {mode:?}\n{out}");
            assert!(
                !out.contains("t: undefined") && !out.contains("u: 1"),
                "{name} {mode:?}\n{out}"
            );
            assert!(
                !out.contains("{undefined} */ const t"),
                "{name} {mode:?}\n{out}"
            );
        }
    }
}
