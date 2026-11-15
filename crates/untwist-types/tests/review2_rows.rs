use untwist_types::{annotate, annotate_in, realm, Mode, Refusal};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes/a40r")
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
        probe: "ev1.js",
        ts_absent: &["a: 1"],
        jsdoc_absent: &["{1} */ var a", "{1 | undefined} */ var a"],
    },
    Row {
        probe: "ev2.js",
        ts_absent: &["a: 1"],
        jsdoc_absent: &["{1} */ let a"],
    },
    Row {
        probe: "ev8.js",
        ts_absent: &[
            "n: number =",
            "number | undefined =>",
            "u: undefined",
            "w: number",
        ],
        jsdoc_absent: &["{number} */ const n", "@returns {number | undefined}"],
    },
    Row {
        probe: "ex-a.mjs",
        ts_absent: &["c: 3", "): 3", "r: 3", "d: 4", "): 4"],
        jsdoc_absent: &["{3} c", "{4} d", "@returns {3}", "@returns {4}"],
    },
    Row {
        probe: "fi1.js",
        ts_absent: &[
            "y: number | undefined =",
            "(c): 1 | undefined =>",
            "z: 1 | undefined =",
            "y: 1",
        ],
        jsdoc_absent: &[
            "{number | undefined} */ const y",
            "@returns {1 | undefined}",
        ],
    },
];

#[test]
fn every_a40r_probe_keeps_its_false_type_out_in_both_modes() {
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
fn a_caller_in_another_input_closes_no_function() {
    let (helper, user) = (probe("fr-b.js"), probe("fr-a.js"));
    let shared = realm(&[&helper, &user]);
    assert!(!shared.closed_calls);
    for mode in [Mode::Ts, Mode::Jsdoc] {
        let out = annotate_in(&user, mode, Some(shared)).code;
        assert!(!out.contains("a: 1") && !out.contains("{1} a"), "{out}");
        assert!(
            !out.contains("r: 1") && !out.contains("{1} */ const r"),
            "{out}"
        );
    }
}

#[test]
fn a_name_node_type_stripping_rejects_keeps_the_js_name() {
    assert_eq!(
        annotate(&probe("t8b.js"), Mode::Ts).refused,
        Some(Refusal::Erasure)
    );
    assert_eq!(annotate(&probe("t8b.js"), Mode::Jsdoc).refused, None);
}

#[test]
fn every_code_loading_route_turns_the_built_in_rows_off() {
    let mut failures = Vec::new();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes/a40r");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| {
            entry
                .ok()
                .map(|e| e.file_name().to_string_lossy().into_owned())
        })
        .filter(|name| name.starts_with("ld-"))
        .collect();
    names.sort();
    assert!(names.len() > 10);
    for name in names {
        let source = probe(&name);
        if !realm(&[&source]).builtins {
            continue;
        }
        let out = annotate(&source, Mode::Ts).code;
        if out.contains("r: number") || out.contains("p: number") {
            failures.push(format!("{name}: built-in rows stayed on\n{out}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn a_commonjs_file_is_left_untyped_in_ts_mode() {
    let source = "var path = require(\"path\");\nfunction f() {\n  let n = 0;\n  n += 1;\n  return n;\n}\nexports.f = f;\n";
    assert_eq!(annotate(source, Mode::Ts).refused, Some(Refusal::CommonJs));
    assert!(annotate(source, Mode::Jsdoc)
        .code
        .contains("@type {number}"));
}
