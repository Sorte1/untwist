use untwist::{Options, Output, Pipeline};
use untwist_deobfuscate::notes;
use untwist_testing::node_rows::{mismatches, node_or_skip, patched_after_named_rotator, Mode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lane {
    Safe,
    Unsafe,
    Conservative,
}

const LANES: [Lane; 3] = [Lane::Safe, Lane::Unsafe, Lane::Conservative];

fn run(source: &str, lane: Lane) -> Output {
    Pipeline::new(Options {
        unsafe_transforms: lane == Lane::Unsafe,
        extended: lane != Lane::Conservative,
        unpack: false,
        jsx: false,
        ..Options::default()
    })
    .run(source)
}

fn decoded(source: &str, lane: Lane) -> bool {
    let out = run(source, lane);
    ["console.log(bQ(", ", bQ(1)"]
        .iter()
        .all(|call| !out.code().contains(call))
}

fn bootstrap(runtime: &str, table: &str, start: &str) -> String {
    format!(
        "!(function (e) {{
  var r = {{}};
  function n(t) {{
    if (r[t]) return r[t].exports;
    var m = (r[t] = {{ i: t, l: false, exports: {{}} }});
    return e[t].call(m.exports, m, m.exports, n), (m.l = true), m.exports;
  }}
  n.m = e;
  n.c = r;
  n.d = function (t, k, g) {{ Object.defineProperty(t, k, {{ enumerable: true, value: g }}); }};
  n.r = function (t) {{ Object.defineProperty(t, '__esModule', {{ value: true }}); }};
  n.t = function (v, mode) {{
    if (mode & 1) v = n(v);
    if (mode & 8) return v;
    var ns = Object.create(null);
    n.r(ns);
    for (var key in v) n.d(ns, key, v[key]);
    return ns;
  }};
  n.n = function (t) {{ var g = t && t.__esModule ? function () {{ return t.default; }} : function () {{ return t; }}; n.d(g, 'a', g); return g; }};
  n.o = function (t, k) {{ return Object.prototype.hasOwnProperty.call(t, k); }};
  n.p = '';
{runtime}
  return n({start});
}})([
{table}
]);"
    )
}

const MODULE_ZERO: &str = "  function (e, t, i) {
    'use strict';
    i.r(t);
    var lib = i(1);
    i.d(t, 'shown', lib.text);
    console.log(lib.text, bQ(0) === lib.text, i.n(lib).a === lib, i.c[1].l);
  }";

const MODULE_ONE: &str = "  function (e, t, i) {
    i.r(t);
    i.d(t, 'text', bQ(1));
  }";

const PATCH: &str = "function () { return 'patched'; }";

fn before_bootstrap(prefix: &str, program: String) -> String {
    program.replacen(
        "!(function (e) {",
        &format!("{prefix}\n!(function (e) {{"),
        1,
    )
}

fn with_modules(runtime: &str, modules: &[&str], start: &str) -> String {
    patched_after_named_rotator(
        "Q",
        "true",
        &bootstrap(runtime, &modules.join(",\n"), start),
    )
}

fn proven() -> String {
    with_modules("", &[MODULE_ZERO, MODULE_ONE], "(n.s = 0)")
}

fn define_on_global(alias: &str) -> String {
    format!("{alias}.d(globalThis, String.fromCharCode(98, 81), {PATCH});")
}

fn module_one_with(body: &str) -> String {
    format!("  function (e, t, i) {{\n    i.r(t);\n    {body}\n    i.d(t, 'text', bQ(1));\n  }}")
}

fn mutants() -> Vec<(&'static str, &'static str, String)> {
    let planted = format!(
        "function (e, t, q) {{ {} q.d(t, 'text', bQ(1)); }}",
        define_on_global("q")
    );
    vec![
        (
            "require.d is called on the global object",
            "",
            with_modules(
                "",
                &[MODULE_ZERO, &module_one_with(&define_on_global("i"))],
                "(n.s = 0)",
            ),
        ),
        (
            "the module table is written by the bootstrap",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            with_modules(
                &format!("  e[1] = {planted};"),
                &[MODULE_ZERO, MODULE_ONE],
                "(n.s = 0)",
            ),
        ),
        (
            "the module table is written through require.m",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            with_modules(
                "",
                &[
                    &MODULE_ZERO.replacen(
                        "var lib = i(1);",
                        &format!("i.m[1] = {planted}; var lib = i(1);"),
                        1,
                    ),
                    MODULE_ONE,
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "a module table element is not a literal function",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            before_bootstrap(
                &format!("var planted = {planted};"),
                with_modules("", &[MODULE_ZERO, "  planted"], "(n.s = 0)"),
            ),
        ),
        (
            "a module reads its require through arguments",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&define_on_global("arguments[2]")),
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "require escapes into a local",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&format!("var q = i; {}", define_on_global("q"))),
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "require.d is read as a value",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&format!(
                        "var dd = i.d; dd(globalThis, String.fromCharCode(98, 81), {PATCH});"
                    )),
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "require names a module id outside the table",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            before_bootstrap(
                &format!("Array.prototype[2] = {planted};"),
                with_modules(
                    "",
                    &[
                        &MODULE_ZERO.replacen("var lib = i(1);", "i(2); var lib = i(1);", 1),
                        MODULE_ONE,
                    ],
                    "(n.s = 0)",
                ),
            ),
        ),
        (
            "the dispatch may call a function outside the table",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            before_bootstrap(
                &format!("var planted = {planted};"),
                with_modules("", &[MODULE_ZERO, MODULE_ONE], "(n.s = 0)")
                    .replacen("return e[t].call(", "return (e[t] || planted).call(", 1)
                    .replacen("var lib = i(1);", "i(5); var lib = i(1);", 1),
            ),
        ),
    ]
}

#[test]
fn every_lane_decodes_a_proven_webpack_bootstrap() {
    let source = proven();
    for lane in LANES {
        let out = run(&source, lane);
        let messages: Vec<&str> = out.notes.iter().map(|n| n.message.as_str()).collect();
        assert!(decoded(&source, lane), "{lane:?}: {messages:#?}");
    }
}

#[test]
fn the_safe_lane_refuses_every_mutant_and_names_why() {
    for (label, note, source) in mutants() {
        let out = run(&source, Lane::Safe);
        assert!(!decoded(&source, Lane::Safe), "{label}: {}", out.code());
        let messages: Vec<&str> = out.notes.iter().map(|n| n.message.as_str()).collect();
        assert!(
            messages
                .iter()
                .any(|m| m.contains(notes::STRING_ARRAY_GLOBAL_REACH)),
            "{label}: {messages:?}"
        );
        if !note.is_empty() {
            assert!(
                messages.iter().any(|m| m.contains(note)),
                "{label}: {messages:?}"
            );
        }
    }
}

fn lane_runner(lane: Lane) -> impl Fn(&str) -> String {
    move |source: &str| run(source, lane).code().to_string()
}

fn decoded_anyway(source: &str) -> String {
    let unsafe_out = run(source, Lane::Unsafe);
    unsafe_out.code().to_string()
}

#[test]
fn node_agrees_in_the_safe_and_conservative_lanes_and_every_mutant_diverges_when_decoded() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let mut programs = vec![("a proven bootstrap".to_string(), proven())];
    programs.extend(
        mutants()
            .into_iter()
            .map(|(label, _, source)| (label.to_string(), source)),
    );
    let safe = lane_runner(Lane::Safe);
    let conservative = lane_runner(Lane::Conservative);
    let modes: [Mode<'_>; 2] = [("safe", &safe), ("conservative", &conservative)];
    let wrong = mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let unsafe_lane = lane_runner(Lane::Unsafe);
    let modes: [Mode<'_>; 1] = [("unsafe", &unsafe_lane)];
    assert!(mismatches(&node, &programs[..1], &modes).is_empty());
    let decoded_modes: [Mode<'_>; 1] = [("decoded", &decoded_anyway)];
    let mut diverged: Vec<String> = mismatches(&node, &programs[1..], &decoded_modes)
        .into_iter()
        .filter_map(|row| row.split(" (decoded)").next().map(str::to_string))
        .collect();
    diverged.sort();
    let mut want: Vec<String> = programs[1..]
        .iter()
        .map(|(label, _)| label.clone())
        .collect();
    want.sort();
    assert_eq!(diverged, want);
}
