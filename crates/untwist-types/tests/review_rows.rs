use untwist_types::{annotate, annotate_in, realm, Mode, Refusal};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

struct Row {
    probe: &'static str,
    ts_absent: &'static [&'static str],
    jsdoc_absent: &'static [&'static str],
}

const ROWS: &[Row] = &[
    Row {
        probe: "a1.js",
        ts_absent: &["x: 1 ="],
        jsdoc_absent: &["@type {1} */ var x"],
    },
    Row {
        probe: "a2.js",
        ts_absent: &["a: 1", "): 1 {"],
        jsdoc_absent: &["{1} a", "@returns {1}"],
    },
    Row {
        probe: "a4.js",
        ts_absent: &["a: 1", "b: 2"],
        jsdoc_absent: &["{1} a", "{2} b"],
    },
    Row {
        probe: "a5.js",
        ts_absent: &["m: number =", "r: number ="],
        jsdoc_absent: &["{number} */ const m", "{number} */ const r"],
    },
    Row {
        probe: "a6.js",
        ts_absent: &["e: \"s\"", "e: string"],
        jsdoc_absent: &["*/ var e"],
    },
    Row {
        probe: "b1.js",
        ts_absent: &["a: 1"],
        jsdoc_absent: &["{1} a"],
    },
    Row {
        probe: "e23.js",
        ts_absent: &["r: number", "p: number", "n: string", "u: string"],
        jsdoc_absent: &[
            "{number} */ const r",
            "{number} */ const p",
            "{string} */ const n",
        ],
    },
    Row {
        probe: "e24.js",
        ts_absent: &["r: number", "p: number", "n: string", "u: string"],
        jsdoc_absent: &[
            "{number} */ const r",
            "{number} */ const p",
            "{string} */ const n",
        ],
    },
    Row {
        probe: "e30b.js",
        ts_absent: &["p: number", "r: number"],
        jsdoc_absent: &["{number} */ const p", "{number} */ const r"],
    },
    Row {
        probe: "g18.js",
        ts_absent: &["f: 1", "k: \"s\"", "k: string"],
        jsdoc_absent: &["*/ var f", "*/ var k"],
    },
    Row {
        probe: "g19.js",
        ts_absent: &["(a: 1)", "(c: 1)", "(d: 1)", "(e: 1)"],
        jsdoc_absent: &["{1} a", "{1} c", "{1} d", "{1} e"],
    },
    Row {
        probe: "g20.js",
        ts_absent: &["r: 1", "s: 3", "r: number", "s: number"],
        jsdoc_absent: &["{1} */ const r", "{3} */ const s"],
    },
    Row {
        probe: "g21.js",
        ts_absent: &["x: 1 =", "y: 1 =", "z: 0 =", "w: 1 ="],
        jsdoc_absent: &[
            "{1} */ let x",
            "{1} */ let y",
            "{0} */ let z",
            "{1} */ let w",
        ],
    },
];

#[test]
fn every_review_probe_keeps_its_false_type_out_in_both_modes() {
    let mut failures = Vec::new();
    for row in ROWS {
        let source = probe(row.probe);
        for (mode, absent) in [(Mode::Ts, row.ts_absent), (Mode::Jsdoc, row.jsdoc_absent)] {
            let out = annotate(&source, mode).code;
            for needle in absent {
                if out.contains(needle) {
                    failures.push(format!("{} {mode:?}: found {needle:?}\n{out}", row.probe));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_file_typescript_reads_differently_is_refused_even_without_annotations() {
    let out = annotate(&probe("b3.js"), Mode::Ts);
    assert_eq!(out.refused, Some(Refusal::Erasure));
    assert_eq!(out.code, probe("b3.js"));
    assert_eq!(annotate(&probe("b3.js"), Mode::Jsdoc).refused, None);
}

#[test]
fn a_patch_in_another_input_turns_the_built_in_rows_off() {
    let (patch, user) = (probe("m1.js"), probe("m2.js"));
    let alone = annotate(&user, Mode::Ts).code;
    assert!(alone.contains("r: number"), "{alone}");
    let shared = realm(&[&patch, &user]);
    assert!(!shared.builtins);
    let together = annotate_in(&user, Mode::Ts, Some(shared)).code;
    assert!(!together.contains("r: number"), "{together}");
    assert!(!together.contains("t: string"), "{together}");
}

#[test]
fn a_local_dynamic_import_keeps_the_built_in_rows() {
    let source = "(function () {\n  import(\"./chunk.js\").then(function () {\n    const n = Math.floor(2.5);\n    console.log(n);\n  });\n})();\n";
    assert!(annotate(source, Mode::Ts).code.contains("n: number"));
}

#[test]
fn a_setter_default_typescript_flags_as_grammar_still_gets_types() {
    let source = "(function () {\n  class Texture {\n    set image(data = null) {\n      this.data = data;\n    }\n  }\n  let count = 0;\n  count += 1;\n  console.log(new Texture(), count);\n})();\n";
    let out = annotate(source, Mode::Ts);
    assert_eq!(out.refused, None);
    assert!(out.code.contains("let count: number = 0"), "{}", out.code);
}
