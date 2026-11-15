use untwist_types::{annotate, Mode, Refusal};

fn probe(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/probes/a40r6")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

const FINDINGS: [&str; 15] = [
    "a/c1.mjs",
    "a/c2.mjs",
    "a/upd1.mjs",
    "b/eq.mjs",
    "b/inop.mjs",
    "b/iter.mjs",
    "b/typeof.mjs",
    "b/not.mjs",
    "b/tpl.mjs",
    "b/cmpnest.mjs",
    "b/cond.mjs",
    "b/callres.mjs",
    "b/getter.mjs",
    "c/noarith.mjs",
    "c/noarith2.mjs",
];

const CONTROLS: [&str; 12] = [
    "a/big1.mjs",
    "a/cmp1.mjs",
    "a/date1.mjs",
    "a/new1.mjs",
    "a/plus1.mjs",
    "a/sym1.mjs",
    "c/big2.mjs",
    "c/big3.mjs",
    "c/bottom.mjs",
    "c/date2.mjs",
    "c/lit.mjs",
    "c/prim.mjs",
];

#[test]
fn a_file_with_a_well_known_symbol_key_is_left_untyped_in_both_modes() {
    let mut failures = Vec::new();
    for name in FINDINGS {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            let out = annotate(&source, mode);
            if out.refused != Some(Refusal::SymbolKey) || out.code != source {
                failures.push(format!("{name} {mode:?}: {:?}\n{}", out.refused, out.code));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn the_controls_without_a_symbol_key_are_not_refused_for_one() {
    let mut failures = Vec::new();
    for name in CONTROLS {
        let source = probe(name);
        for mode in [Mode::Jsdoc, Mode::Ts] {
            if annotate(&source, mode).refused == Some(Refusal::SymbolKey) {
                failures.push(format!("{name} {mode:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_class_member_keyed_by_a_well_known_symbol_keeps_the_types() {
    let source = "class A {\n  [Symbol.iterator]() {\n    return [][Symbol.iterator]();\n  }\n}\nconst n = 1;\nconsole.log(new A(), n);\nexport {};\n";
    let out = annotate(source, Mode::Ts);
    assert_eq!(out.refused, None);
    assert!(out.code.contains("const n: 1 = 1"), "{}", out.code);
}

#[test]
fn objects_the_checker_reports_alike_keep_their_types() {
    for source in [
        "const a = [] * 2;\nconst n = 1;\nconsole.log(a, n);\nexport {};\n",
        "function fn() {}\nconst a = fn - 1;\nconst n = 1;\nconsole.log(a, n);\nexport {};\n",
        "const a = /x/ | 0;\nconst n = 1;\nconsole.log(a, n);\nexport {};\n",
        "const o = { valueOf() { return 1; } };\nconst n = 1;\nconsole.log(o - 1, n);\nexport {};\n",
        "const sym = Symbol(\"s\");\nconst o = { [sym]: 1 };\nconst n = 1;\nconsole.log(o, n);\nexport {};\n",
        "const o = { [\"k\"]: 1 };\nconst n = 1;\nconsole.log(o, n);\nexport {};\n",
    ] {
        let out = annotate(source, Mode::Ts);
        assert_eq!(out.refused, None, "{source}");
        assert!(out.code.contains("const n: 1 = 1"), "{}", out.code);
    }
}
