use untwist_types::{annotate, Mode};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes/a40r3")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

#[test]
fn a_frame_read_turns_the_built_in_rows_off_in_both_modes() {
    let mut failures = Vec::new();
    for name in [
        "f-f1.js", "f-f2.js", "f-f3.js", "f-f4.js", "f-f5.js", "f-f6.js",
    ] {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            let typed = [
                "r: number",
                "p: number",
                "{number} */ const r",
                "{number} */ const p",
            ]
            .iter()
            .any(|needle| out.contains(needle));
            if typed {
                failures.push(format!("{name} {mode:?}\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_this_that_can_be_the_global_object_keeps_sloppy_closed_calls_untyped() {
    let mut failures = Vec::new();
    for name in ["h-h1.js", "h-h2.js", "h-h3.js", "h-h4.js"] {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode).code;
            let typed = ["a: 1", "{1} a", "r: 1", "{1} */ const r"]
                .iter()
                .any(|needle| out.contains(needle));
            if typed {
                failures.push(format!("{name} {mode:?}\n{out}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn strict_code_keeps_its_closed_calls_beside_a_sloppy_this() {
    let source = "(function () {\n  const G = (function () { return this; })();\n  console.log(typeof G);\n})();\n(function () {\n  \"use strict\";\n  function f(a) {\n    return a * 2;\n  }\n  console.log(f(1));\n})();\n";
    let out = annotate(source, Mode::Jsdoc).code;
    assert!(out.contains("@param {1} a"), "{out}");
}

#[test]
fn the_probe_controls_are_typed_without_the_hazard() {
    let builtins = "(function () { const r = Math.random(); const p = parseInt(\"12\"); console.log(typeof r, typeof p); })();\n";
    let calls = "(function () {\n  function g() { if (!g.done) { g.done = true; } }\n  function f(a) { console.log(typeof a); g(); return a; }\n  const r = f(1);\n  console.log(typeof r);\n})();\n";
    for mode in [Mode::Jsdoc, Mode::Ts] {
        let typed = annotate(builtins, mode).code;
        assert!(
            ["r: number", "{number} */ const r"]
                .iter()
                .any(|n| typed.contains(n)),
            "{mode:?}\n{typed}"
        );
        let typed = annotate(calls, mode).code;
        assert!(
            ["a: 1", "{1} a"].iter().any(|n| typed.contains(n)),
            "{mode:?}\n{typed}"
        );
    }
}
