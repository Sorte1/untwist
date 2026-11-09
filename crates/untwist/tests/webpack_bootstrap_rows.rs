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

const MODULE_ZERO_UNCACHED: &str = "  function (e, t, i) {
    'use strict';
    i.r(t);
    var lib = i(1);
    i.d(t, 'shown', lib.text);
    console.log(lib.text, bQ(0) === lib.text, i.n(lib).a === lib);
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

fn webpack4(record: &str, dispatch: &str) -> String {
    format!(
        "!(function (e) {{
  var r = {{}};
  function n(t) {{
    if (r[t]) return r[t].exports;
    {record}
    return {dispatch}, (m.l = true), m.exports;
  }}
  return (
    (n.m = e),
    (n.c = r),
    (n.d = function (t, k, g) {{ n.o(t, k) || Object.defineProperty(t, k, {{ enumerable: true, get: g }}); }}),
    (n.r = function (t) {{
      typeof Symbol !== 'undefined' && Symbol.toStringTag && Object.defineProperty(t, Symbol.toStringTag, {{ value: 'Module' }});
      Object.defineProperty(t, '__esModule', {{ value: true }});
    }}),
    (n.t = function (v, mode) {{
      if (mode & 1) v = n(v);
      if (mode & 8) return v;
      if (mode & 4 && typeof v === 'object' && v && v.__esModule) return v;
      var ns = Object.create(null);
      n.r(ns);
      Object.defineProperty(ns, 'default', {{ enumerable: true, value: v }});
      if (mode & 2 && typeof v != 'string') for (var key in v) n.d(ns, key, function (k) {{ return v[k]; }}.bind(null, key));
      return ns;
    }}),
    (n.n = function (t) {{ var g = t && t.__esModule ? function () {{ return t.default; }} : function () {{ return t; }}; return n.d(g, 'a', g), g; }}),
    (n.o = function (t, k) {{ return Object.prototype.hasOwnProperty.call(t, k); }}),
    (n.p = ''),
    n((n.s = 0))
  );
}})([
  function (e, t, i) {{
    'use strict';
    i.r(t);
    var lib = i(1);
    i.d(t, 'shown', function () {{ return lib.text; }});
    console.log(lib.text, bQ(0) === lib.text, i.n(lib).a === lib, t[Symbol.toStringTag]);
  }},
  function (e, t, i) {{
    i.r(t);
    i.d(t, 'text', function () {{ return bQ(1); }});
  }}
]);"
    )
}

const RECORD: &str = "var m = (r[t] = { i: t, l: false, exports: {} });";

const DISPATCH: &str = "e[t].call(m.exports, m, m.exports, n)";

fn webpack4_proven() -> String {
    patched_after_named_rotator("Q", "true", &webpack4(RECORD, DISPATCH))
}

fn arrow_outer(source: String) -> String {
    assert!(source.contains("!(function (e) {"));
    source.replacen("!(function (e) {", "!((e) => {", 1)
}

fn cached_or_created(source: String) -> String {
    let canonical = format!(
        "if (r[t]) return r[t].exports;\n    {RECORD}\n    return {DISPATCH}, (m.l = true), m.exports;"
    );
    let minified = format!(
        "var m;\n    return (r[t] || ((m = r[t] = {{ i: t, l: false, exports: {{}} }}), {DISPATCH}, (m.l = true), m)).exports;"
    );
    assert!(source.contains(&canonical));
    source.replacen(&canonical, &minified, 1)
}

fn minified_bootstraps() -> Vec<(&'static str, String)> {
    vec![
        ("an arrow bootstrap", arrow_outer(proven())),
        ("the webpack4 runtime in an arrow", arrow_outer(webpack4_proven())),
        (
            "the webpack4 runtime reading its cache with ||",
            cached_or_created(webpack4_proven()),
        ),
        (
            "the minified webpack4 runtime",
            arrow_outer(cached_or_created(webpack4_proven())),
        ),
        (
            "a minified module defines a computed key on its exports",
            cached_or_created(exports_defined_by_computed_key()),
        ),
    ]
}

const MINIFIED_RECORD: &str = "(m = r[t] = { i: t, l: false, exports: {} }), ";

fn define_on_exports() -> String {
    format!("i.d(t, String.fromCharCode(98, 81), {PATCH});")
}

fn exports_mutant(record: &str, dispatch: &str) -> String {
    exports_mutant_from(MODULE_ZERO_UNCACHED, record, dispatch)
}

fn exports_mutant_from(module_zero: &str, record: &str, dispatch: &str) -> String {
    with_modules(
        "",
        &[module_zero, &module_one_with(&define_on_exports())],
        "(n.s = 0)",
    )
    .replacen(RECORD, record, 1)
    .replacen(
        &format!("return {DISPATCH},"),
        &format!("return {dispatch},"),
        1,
    )
}

fn exports_defined_by_computed_key() -> String {
    exports_mutant(RECORD, DISPATCH)
}

fn amd_define(factory: &str) -> String {
    format!("var r; !((r = {factory}.call(t, i, t, e)), r !== undefined && (e.exports = r));")
}

fn amd_factory_ignores_require() -> String {
    with_modules(
        "",
        &[MODULE_ZERO, &module_one_with(&amd_define("function () {}"))],
        "(n.s = 0)",
    )
}

fn dispatched_mutants() -> Vec<(&'static str, String)> {
    let (record, dispatch) = (RECORD, DISPATCH);
    let creation_patched = before_bootstrap(
        &format!("Object.create = {};", "function () { return globalThis; }"),
        with_modules(
            "",
            &[
                &MODULE_ZERO.replacen("var lib = i(1);", "var lib = i(1); i.t(1, 1);", 1),
                &module_one_with(&define_on_exports()),
            ],
            "(n.s = 0)",
        ),
    );
    vec![
        (
            "require hands a module the global object between the record and the dispatch",
            exports_mutant(&format!("{record} m.exports = globalThis;"), dispatch),
        ),
        (
            "the dispatch passes the global object as exports",
            exports_mutant(record, "e[t].call(m.exports, m, globalThis, n)"),
        ),
        (
            "a dispatch argument runs code before exports is read",
            exports_mutant(
                record,
                "e[t].call(m.exports, ((m.exports = globalThis), m), m.exports, n)",
            ),
        ),
        (
            "the module record defines exports through a getter",
            exports_mutant(
                "var m = (r[t] = { i: t, l: false, get exports() { return globalThis; } });",
                dispatch,
            ),
        ),
        (
            "the module record defines exports twice",
            exports_mutant(
                "var m = (r[t] = { i: t, l: false, exports: {}, exports: globalThis });",
                dispatch,
            ),
        ),
        (
            "a module reassigns its exports parameter",
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&format!("t = globalThis; {}", define_on_exports())),
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "Object.create is replaced before the bootstrap",
            creation_patched,
        ),
        (
            "a module installs a setter on the module cache",
            exports_mutant_from(
                &MODULE_ZERO_UNCACHED.replacen(
                    "var lib = i(1);",
                    "Object.defineProperty(i.c, 1, { configurable: true, set: function (v) { v.exports = globalThis; } }); var lib = i(1);",
                    1,
                ),
                record,
                dispatch,
            ),
        ),
        (
            "an AMD factory takes require as a parameter",
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&amd_define(&format!(
                        "function (q) {{ {} }}",
                        define_on_global("q")
                    ))),
                ],
                "(n.s = 0)",
            ),
        ),
        (
            "an AMD factory reads require through arguments",
            with_modules(
                "",
                &[
                    MODULE_ZERO,
                    &module_one_with(&amd_define(&format!(
                        "function () {{ {} }}",
                        define_on_global("arguments[0]")
                    ))),
                ],
                "(n.s = 0)",
            ),
        ),
    ]
}

const W5_RECORD: &str = "var m = (c[id] = { exports: {} });";

const W5_DISPATCH: &str = "e[id](m, m.exports, n);";

const W5_PATCH: &str = "t[String.fromCharCode(98, 81)] = function () { return 'patched'; };";

fn webpack5(record: &str, dispatch: &str, module_one: &str) -> String {
    patched_after_named_rotator(
        "Q",
        "true",
        &format!(
            "(() => {{
  var e = {{
    0: (m, t, i) => {{
      i.r(t);
      var lib = i(1);
      console.log(lib.text, bQ(0) === lib.text, typeof lib.shown, t[Symbol.toStringTag]);
    }},
    1: (m, t, i) => {{
      i.r(t);
      i.d(t, {{ text: () => bQ(1), shown: () => 'x' }});
      {module_one}
    }},
  }};
  var c = {{}};
  function n(id) {{
    var cached = c[id];
    if (cached !== undefined) return cached.exports;
    {record}
    {dispatch}
    return m.exports;
  }}
  n.d = (exports, definition) => {{
    for (var key in definition)
      if (n.o(definition, key) && !n.o(exports, key))
        Object.defineProperty(exports, key, {{ enumerable: true, get: definition[key] }});
  }};
  n.o = (o, p) => Object.prototype.hasOwnProperty.call(o, p);
  n.r = (exports) => {{
    typeof Symbol !== 'undefined' && Symbol.toStringTag && Object.defineProperty(exports, Symbol.toStringTag, {{ value: 'Module' }});
    Object.defineProperty(exports, '__esModule', {{ value: true }});
  }};
  n(0);
}})();"
        ),
    )
}

fn webpack5_proven() -> String {
    webpack5(W5_RECORD, W5_DISPATCH, W5_PATCH)
}

fn webpack5_mutants() -> Vec<(&'static str, String)> {
    vec![
        (
            "webpack 5 require hands a module the global object",
            webpack5(
                &format!("{W5_RECORD} m.exports = globalThis;"),
                W5_DISPATCH,
                W5_PATCH,
            ),
        ),
        (
            "the webpack 5 dispatch passes the global object as exports",
            webpack5(W5_RECORD, "e[id](m, globalThis, n);", W5_PATCH),
        ),
        (
            "a webpack 5 definition holds a bound definer as a getter",
            webpack5(W5_RECORD, W5_DISPATCH, "")
                .replacen(
                    "i.d(t, { text: () => bQ(1), shown: () => 'x' });",
                    "i.d(t, { text: () => bQ(1), shown: Object.defineProperty.bind(null, globalThis, String.fromCharCode(98, 81), { value: function () { return 'patched'; } }) });",
                    1,
                ),
        ),
        (
            "a webpack 5 module reassigns its exports parameter",
            webpack5(
                W5_RECORD,
                W5_DISPATCH,
                &format!("t = globalThis; {W5_PATCH}"),
            ),
        ),
    ]
}

fn mutants() -> Vec<(&'static str, &'static str, String)> {
    let planted = format!(
        "function (e, t, q) {{ {} q.d(t, 'text', bQ(1)); }}",
        define_on_global("q")
    );
    let mut out: Vec<(&'static str, &'static str, String)> = dispatched_mutants()
        .into_iter()
        .chain(webpack5_mutants())
        .map(|(label, source)| (label, "", source))
        .collect();
    out.extend(vec![
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
            "require.d is called on the global object in an arrow bootstrap",
            "",
            arrow_outer(with_modules(
                "",
                &[MODULE_ZERO, &module_one_with(&define_on_global("i"))],
                "(n.s = 0)",
            )),
        ),
        (
            "require.d is called on the global object in a minified bootstrap",
            "",
            arrow_outer(cached_or_created(with_modules(
                "",
                &[MODULE_ZERO, &module_one_with(&define_on_global("i"))],
                "(n.s = 0)",
            ))),
        ),
        (
            "a minified record is replaced before the dispatch",
            "",
            cached_or_created(exports_defined_by_computed_key()).replacen(
                MINIFIED_RECORD,
                &format!("{MINIFIED_RECORD}(m = {{ exports: globalThis }}), "),
                1,
            ),
        ),
        (
            "an arrow bootstrap may dispatch a function outside the table",
            notes::WEBPACK_BOOTSTRAP_UNPROVEN,
            arrow_outer(before_bootstrap(
                &format!("var planted = {planted};"),
                with_modules("", &[MODULE_ZERO, MODULE_ONE], "(n.s = 0)")
                    .replacen("return e[t].call(", "return (e[t] || planted).call(", 1)
                    .replacen("var lib = i(1);", "i(5); var lib = i(1);", 1),
            )),
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
    ]);
    out
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
fn every_lane_decodes_the_webpack4_runtime_with_proven_module_objects() {
    let rows = [
        ("the webpack4 runtime", webpack4_proven()),
        ("a computed exports key", exports_defined_by_computed_key()),
        ("an AMD factory", amd_factory_ignores_require()),
        ("the webpack 5 runtime", webpack5_proven()),
    ];
    for (label, source) in rows.into_iter().chain(minified_bootstraps()) {
        for lane in LANES {
            let out = run(&source, lane);
            let messages: Vec<&str> = out.notes.iter().map(|n| n.message.as_str()).collect();
            assert!(decoded(&source, lane), "{label} {lane:?}: {messages:#?}");
        }
    }
}

fn premise_rows() -> Vec<(String, String)> {
    include_str!("data/webpack_bootstrap_premise_rows.txt")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('|'))
        .map(|(label, prefix)| {
            (
                label.to_string(),
                before_bootstrap(prefix, exports_mutant(RECORD, DISPATCH)),
            )
        })
        .collect()
}

#[test]
fn the_safe_lane_decodes_the_documented_premise_rows() {
    let rows = premise_rows();
    assert!(!rows.is_empty());
    for (label, source) in rows {
        let out = run(&source, Lane::Safe);
        let messages: Vec<&str> = out.notes.iter().map(|n| n.message.as_str()).collect();
        let aliased = messages
            .iter()
            .any(|message| message.starts_with(untwist_deobfuscate::notes::BUILTIN_ALIAS_PATCHED));
        assert!(
            decoded(&source, Lane::Safe) || aliased,
            "{label}: {messages:#?}"
        );
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
    let mut programs = vec![
        ("a proven bootstrap".to_string(), proven()),
        ("the webpack4 runtime".to_string(), webpack4_proven()),
        (
            "a module defines a computed key on its exports".to_string(),
            exports_defined_by_computed_key(),
        ),
        (
            "an AMD factory ignores require".to_string(),
            amd_factory_ignores_require(),
        ),
        ("the webpack 5 runtime".to_string(), webpack5_proven()),
    ];
    programs.extend(
        minified_bootstraps()
            .into_iter()
            .map(|(label, source)| (label.to_string(), source)),
    );
    let proven_count = programs.len();
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
    assert!(mismatches(&node, &programs[..proven_count], &modes).is_empty());
    let decoded_modes: [Mode<'_>; 1] = [("decoded", &decoded_anyway)];
    let mut diverged: Vec<String> = mismatches(&node, &programs[proven_count..], &decoded_modes)
        .into_iter()
        .filter_map(|row| row.split(" (decoded)").next().map(str::to_string))
        .collect();
    diverged.sort();
    let mut want: Vec<String> = programs[proven_count..]
        .iter()
        .map(|(label, _)| label.clone())
        .collect();
    want.sort();
    assert_eq!(diverged, want);
}
