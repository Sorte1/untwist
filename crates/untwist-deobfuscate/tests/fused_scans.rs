#![cfg(debug_assertions)]

use std::path::Path;

use untwist_core::pass::full_rounds_for_tests;
use untwist_deobfuscate::fused::check::{digest_of, fused_and_separate, recording};
use untwist_testing::{js_files_in, real_world_fixture};

const FROZEN: &str = include_str!("data/fused_scans.digest");
const RECORDS_DIR: &str = "UNTWIST_FUSED_RECORDS";
const MODES: [(&str, bool, bool); 4] = [
    ("d", false, true),
    ("p", false, false),
    ("u", true, true),
    ("up", true, false),
];

fn options(unsafe_transforms: bool, extended: bool) -> untwist::Options {
    untwist::Options {
        unsafe_transforms,
        extended,
        ..untwist::Options::default()
    }
}

fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn frozen(key: &str) -> Option<&'static str> {
    FROZEN.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?;
        rest.strip_prefix(' ')
    })
}

fn mode_records(inputs: &[(String, String)], mode: (&str, bool, bool)) -> (Vec<String>, usize) {
    let (_, unsafe_transforms, extended) = mode;
    full_rounds_for_tests(true);
    let mut all = Vec::new();
    let mut count = 0usize;
    for (label, source) in inputs {
        let pipeline = untwist::Pipeline::new(options(unsafe_transforms, extended));
        let (_, records) = recording(|| pipeline.run(source));
        count += records.len();
        all.push(format!("# {}", file_name(label)));
        all.extend(records);
    }
    (all, count)
}

fn write_records(dir: &Path, key: &str, records: &[String], line: &str) {
    std::fs::create_dir_all(dir).expect("create the records dir");
    let stem = key.replace(' ', "-");
    std::fs::write(dir.join(format!("{stem}.records")), records.join("\n"))
        .expect("write the records");
    std::fs::write(
        dir.join(format!("{stem}.digest")),
        format!("{key} {line}\n"),
    )
    .expect("write the digest line");
}

fn check_mode(scope: &str, inputs: &[(String, String)], mode: (&str, bool, bool)) -> usize {
    let key = format!("{scope} {}", mode.0);
    let (records, count) = mode_records(inputs, mode);
    let line = format!("{count} {:016x}", digest_of(&records));
    match std::env::var_os(RECORDS_DIR) {
        Some(dir) => write_records(Path::new(&dir), &key, &records, &line),
        None => assert_eq!(
            frozen(&key),
            Some(line.as_str()),
            "{key}: the scan records moved; regenerate with {RECORDS_DIR}=<dir> to compare"
        ),
    }
    count
}

fn check_scope(scope: &str, inputs: &[(String, String)]) -> usize {
    assert!(!inputs.is_empty(), "{scope}: no inputs");
    MODES
        .iter()
        .map(|mode| check_mode(scope, inputs, *mode))
        .sum()
}

fn game() -> Option<Vec<(String, String)>> {
    Some(vec![(
        "game.js".to_string(),
        real_world_fixture("game.js")?,
    )])
}

#[test]
fn game_js_default_records_equal_the_separate_walks() {
    let Some(game) = game() else {
        return;
    };
    assert!(check_mode("game", &game, MODES[0]) > 0);
}

#[test]
fn game_js_conservative_records_equal_the_separate_walks() {
    let Some(game) = game() else {
        return;
    };
    assert!(check_mode("game", &game, MODES[1]) > 0);
}

#[test]
fn game_js_unsafe_records_equal_the_separate_walks() {
    let Some(game) = game() else {
        return;
    };
    assert!(check_mode("game", &game, MODES[2]) > 0);
}

#[test]
fn game_js_unsafe_conservative_records_equal_the_separate_walks() {
    let Some(game) = game() else {
        return;
    };
    assert!(check_mode("game", &game, MODES[3]) > 0);
}

#[test]
fn the_high_preset_records_equal_the_separate_walks() {
    let high = js_files_in("fixtures/reference/deobfuscate")
        .into_iter()
        .filter(|(label, _)| label.ends_with("obfuscator.io-high.js"))
        .collect::<Vec<_>>();
    assert!(check_scope("high", &high) > 0);
}

#[test]
fn corpus_npm_records_equal_the_separate_walks() {
    assert!(check_scope("corpus-npm", &js_files_in("fixtures/corpus-npm")) > 0);
}

#[test]
fn behaviour_records_equal_the_separate_walks() {
    assert!(check_scope("behaviour", &js_files_in("fixtures/behaviour")) > 0);
}

#[test]
fn the_diff_sets_records_equal_the_separate_walks() {
    for (scope, dir) in [
        ("deobfuscate", "fixtures/reference/deobfuscate"),
        ("unpack", "fixtures/reference/unpack"),
        ("unminify", "fixtures/unminify"),
        ("jsx", "fixtures/jsx"),
    ] {
        check_scope(scope, &js_files_in(dir));
    }
}

fn fused_records(label: &str, source: &str, records: &mut Vec<String>) {
    for (group, fused, separate) in fused_and_separate(source) {
        assert_eq!(
            fused, separate,
            "{label}: {group} differs fused vs separate"
        );
        records.push(format!("{label} {group} {fused:016x}"));
    }
}

fn check_frozen_scans(key: &str, cases: &[(String, String)]) {
    let mut records = Vec::new();
    for (label, source) in cases {
        fused_records(label, source, &mut records);
    }
    let line = format!("{} {:016x}", records.len(), digest_of(&records));
    match std::env::var_os(RECORDS_DIR) {
        Some(dir) => write_records(Path::new(&dir), key, &records, &line),
        None => assert_eq!(
            frozen(key),
            Some(line.as_str()),
            "{key}: the scan results moved; regenerate with {RECORDS_DIR}=<dir> to compare"
        ),
    }
}

const PRUNES_AND_EXITS: [(&str, &str); 8] = [
    (
        "delete prunes the read-only scan",
        "var o = { a: \"x\", b: \"y\" }; delete o.b; delete o[\"a\"]; delete o;\n\
         console.log(o.a, o[\"a\"]); function f() { delete o.a.b; return o.a; } f();",
    ),
    (
        "writes and updates beside reads",
        "var o = { a: 1 }; o.a = 2; o.a += 1; o.b++; --o.c; [o.d] = [1]; ({ e: o.e } = {});\n\
         console.log(o.a, o.b, o.c, o.d, o.e);",
    ),
    (
        "class keys suspend the init order's value",
        "var k = \"m\"; var B = function () { return Object; };\n\
         class A extends B() { [k] = 1; static [k + \"s\"]() { return k; } get [k]() { return 1; } accessor [k + 1] = 2; static { k; } }\n\
         new A(); var C = class extends (k ? Object : Array) { [k]() { return this; } };",
    ),
    (
        "loops, class bodies and functions deepen the repeated scan",
        "var o = { a: 1 }; for (var i = 0; i < 2; i++) { console.log(o.a); }\n\
         for (var p in o) { o; } for (var q of [o]) { q; } while (false) { o.a; } do { o; } while (false);\n\
         class D { m() { return o; } } (() => o)(); (function () { return o; })();",
    ),
    (
        "parameters, catch bindings and redeclarations",
        "var x = 1; var x = 2; function g(p = x, { r } = {}, ...s) { return p + r + s.length; }\n\
         try { throw x; } catch ({ message }) { message; } catch2: for (let y = x; y; y--) { break catch2; }\n\
         var z; z = 3; z = 4; console.log(g(), z, x);",
    ),
    (
        "dynamic keys of an object run beside the candidates",
        "var k = \"a\", j = `b`; var o = {}; o[k] = 1; o[j] = 2; o.c = 3;\n\
         var p = { x: { y: 1 } }; function f() { var q = {}; q[k] = p; return q; } f();",
    ),
    (
        "wrapper aliases in machinery and outside it",
        "function _0x1a2b(i) { var a = _0x1a2b; return a; } var w = _0x1a2b; var v;\n\
         v = _0x1a2b; v = v; w = w; function g() { var u = w; return u; } g(); var w;",
    ),
    (
        "early exits in the many-span searches",
        "var a = 1, b = 2; var c = a; var d = \"s\"; var e = `t`; var f = 1 + 2; var h = function () {};\n\
         console.log(a, b, c, d, e, f, h); { var a = 3; }",
    ),
];

#[test]
fn each_prune_and_early_exit_equals_the_separate_walks_and_the_frozen_results() {
    let cases: Vec<(String, String)> = PRUNES_AND_EXITS
        .iter()
        .map(|(label, source)| (label.to_string(), source.to_string()))
        .collect();
    check_frozen_scans("prunes", &cases);
}

#[test]
fn the_corpora_initial_programs_equal_the_separate_walks_and_the_frozen_results() {
    let cases: Vec<(String, String)> = [
        "fixtures/behaviour",
        "fixtures/corpus-npm",
        "fixtures/reference/deobfuscate",
        "fixtures/reference/unpack",
        "fixtures/unminify",
        "fixtures/jsx",
    ]
    .iter()
    .flat_map(|dir| js_files_in(dir))
    .map(|(label, source)| (file_name(&label), source))
    .collect();
    check_frozen_scans("corpora", &cases);
}
