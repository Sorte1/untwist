mod common;

const NEAR_MISSES: &[(&str, &[&str])] = &[
    (
        "function a() { var h = []; a = function () { return h; }; return a(); }\na();",
        &["function a("],
    ),
    (
        "function a() { var h = ['x']; a = function () { return q; }; return a(); }\na();",
        &["function a(", "return q"],
    ),
    (
        "function a() { var h = ['x']; a = function () { return h; }; }\na();",
        &["function a(", "\"x\""],
    ),
    (
        "function a(z) { var h = ['x']; a = function () { return h; }; return a(); }\na(1);",
        &["function a(z)"],
    ),
    (
        "function a() { var h = ['x', {}]; a = function () { return h; }; return a(); }\na();",
        &["function a(", "{}"],
    ),
    (
        "function __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c]; }\n__DECODE_0__(0);",
        &["__DECODE_0__", "__STRING_ARRAY__"],
    ),
    (
        "function __STRING_ARRAY__() { return 1; }\nfunction __DECODE_0__(c) { var e = __STRING_ARRAY__(); return e[c]; }\n__DECODE_0__(0);",
        &["__DECODE_0__", "__STRING_ARRAY__"],
    ),
    (
        "(function (c, d) { while (!![]) { try { var g = parseInt('1'); if (g === d) break; } catch (h) {} } })(__STRING_ARRAY__, 1);",
        &["__STRING_ARRAY__", "parseInt"],
    ),
    (
        "const arr = ['a'];\nconsole.log(arr[1]);",
        &["arr[1]", "\"a\""],
    ),
    ("const obj = {};\nconsole.log(obj.x);", &["obj.x"]),
    (
        "function w(a, b) { return w(a, b); }\nw(1, 2);",
        &["function w(", "w(1, 2)"],
    ),
    (
        "function a() { var h = ['x']; a = function () { return h; }; return a(); }\nfunction b(c) { var e = a(); return e[c].charAt(0); }\nb(9);",
        &[],
    ),
    (
        "function a() { var h = [void 0]; a = function () { return h; }; return a(); }\nfunction b(c) { var e = a(); return e[c].charAt(0); }\nb(0);",
        &[],
    ),
    (
        "function a() { var h = ['x']; a = function () { return h; }; return a(); }\nfunction b(c, d) { var e = a(); return e[c] + d.charAt(0); }\nfunction w(x) { return b(x); }\nw(0);",
        &["function b(c, d)", "b(0);"],
    ),
];

const UNIT: &str = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
";

const WRONG_REWRITES: &[(&str, &[&str], &str)] = &[
    (
        "function w(x, y) { return b(this.n + x, y); }\nvar o = { n: 0x1, go: function () { return w(0x0, 0x0); } };\nconsole[o.go()]('t');",
        &["function w(x, y)", "this.n + x", "w(0, 0)"],
        "other than its own parameters",
    ),
    (
        "var q = fetch;\nfunction w(x, y) { return b(x + q, y); }\nconsole[w(0x0, 0x0)]('t');",
        &["function w(x, y)", "w(0, 0)"],
        "other than its own parameters",
    ),
    (
        "function w(x, y) { return b(x + y, x); }\nvar i = 0x0;\nconsole[w(i++, 0x0)]('t');",
        &["function w(x, y)", "w(i++, 0)"],
        "side-effect-free",
    ),
    (
        "function w(x, y) { return b(x + y, x); }\nconsole[w(f(), 0x0)]('t');",
        &["function w(x, y)", "w(f(), 0)"],
        "side-effect-free",
    ),
    (
        "function w(x, y) { return b(x + y, x); }\nconsole[w(globalThis.i, 0x0)]('t');",
        &["function w(x, y)", "w(globalThis.i, 0)"],
        "side-effect-free",
    ),
    (
        "var k;\nk = 0x1;\nfunction g() { return b(k); }\nk = 0x0;\nconsole[g()]('t');",
        &["k = 1;", "k = 0;", "b(k)"],
        "not a later member of the assignment",
    ),
    (
        "function g() { return b(k); }\nconsole.log(g());\nvar k = 0x1;",
        &["var k = 1;", "b(k)"],
        "may be read before its initialiser runs",
    ),
    (
        "var o = { k: 0x1, k: 0x2 };\nconsole[b(0x0)](o.k);",
        &["k: 2", "o.k"],
        "more than once or through __proto__",
    ),
    (
        "function g() { return arr[0]; }\nconsole.log(g(), b(0x0));\nvar arr = ['EARLY'];",
        &["return arr[0];", "var arr = [\"EARLY\"];"],
        "may run before its declaration",
    ),
    (
        "function g() { return alias(0x0, 0x1); }\nconsole[g()]('t');\nvar alias = b;",
        &["var alias = b;", "alias(0, 1)"],
        "may be read before its declaration runs",
    ),
];

#[test]
fn no_wrong_rewrite_fires_and_every_refusal_is_reported() {
    for (body, survives, expected_note) in WRONG_REWRITES {
        let source = format!("{UNIT}{body}");
        let (code, notes) = common::run_passes(&source);

        let alloc = untwist_testing::Allocator::default();
        let reparsed = untwist_core::parse::parse_program(&alloc, &code);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{source}\n-> {code}\n{:?}",
            reparsed.diagnostics
        );

        for expected in *survives {
            assert!(
                code.contains(expected),
                "{expected} disappeared\n{source}\n-> {code}"
            );
        }
        assert!(
            notes.iter().any(|note| note.contains(expected_note)),
            "no note matching {expected_note}\n{source}\n-> {code}\n{notes:?}"
        );
    }
}

#[test]
fn a_wrapper_reading_an_outer_constant_decodes_to_the_value_the_outer_binding_holds() {
    let source = format!(
        "{UNIT}var s = 0x0;\nfunction w(x, y) {{ return b(x + s, y); }}\nfunction caller() {{ var s = 0x1; return w(0x0, 0x0); }}\nconsole[caller()]('t');"
    );
    let (code, _notes) = common::run_passes(&source);
    assert!(code.contains("return \"log\";"), "{code}");
    assert!(!code.contains("Hello World!"), "{code}");
}

#[test]
fn no_near_miss_panics_or_changes_the_program_shape() {
    for (source, survives) in NEAR_MISSES {
        let (code, _notes) = common::run_passes(source);

        let alloc = untwist_testing::Allocator::default();
        let reparsed = untwist_core::parse::parse_program(&alloc, &code);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{source}\n-> {code}\n{:?}",
            reparsed.diagnostics
        );

        for expected in *survives {
            assert!(
                code.contains(expected),
                "{expected} disappeared\n{source}\n-> {code}"
            );
        }
    }
}

#[test]
fn a_decoder_whose_array_function_bails_keeps_everything_and_notes_it() {
    let source = "\
function __STRING_ARRAY__() { return unresolvedGlobalThing; }
function __DECODE_0__(c, d) { var e = __STRING_ARRAY__(); return e[c]; }
console.log(__DECODE_0__(0));";
    let (code, notes) = common::run_passes(source);
    assert!(code.contains("function __STRING_ARRAY__()"), "{code}");
    assert!(code.contains("__DECODE_0__(0)"), "{code}");
    assert!(notes.iter().any(|note| note.contains("bail")), "{notes:?}");
}

#[test]
fn a_freshly_named_decoder_that_never_terminates_bails_on_fuel_rather_than_hanging() {
    let source = "\
function a() { var h = ['x']; a = function () { return h; }; return a(); }
function b(c, d) {
  var e = a();
  for (var i = 0; ; i++) { e[i]; }
}
console.log(b(0));";
    let (code, notes) = common::run_passes(source);
    assert!(!code.contains("__STRING_ARRAY__"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(code.contains("function a()"), "{code}");
    assert!(code.contains("b(0)"), "{code}");
    assert!(notes.iter().any(|note| note.contains("bail")), "{notes:?}");
}

#[test]
fn a_decoder_that_never_terminates_bails_on_fuel_rather_than_hanging() {
    let source = "\
function __STRING_ARRAY__() { var h = ['x']; __STRING_ARRAY__ = function () { return h; }; return __STRING_ARRAY__(); }
function __DECODE_0__(c, d) {
  var e = __STRING_ARRAY__();
  for (var i = 0; ; i++) { e[i]; }
}
console.log(__DECODE_0__(0));";
    let (code, notes) = common::run_passes(source);
    assert!(code.contains("function __STRING_ARRAY__()"), "{code}");
    assert!(notes.iter().any(|note| note.contains("bail")), "{notes:?}");
}

#[test]
fn a_wrapper_with_a_nested_function_is_left_alone_with_a_note() {
    let source = "\
function a() { var h = ['log', 'Hello World!']; a = function () { return h; }; return a(); }
function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; return e[f]; }; return b(c, d); }
function w(x, y) { return b(x, function (x) { return x; }); }
console[w(0x0, 0x1)](w(0x1, 0x1));";
    let (code, notes) = common::run_passes(source);
    assert!(code.contains("function w(x, y)"), "{code}");
    assert!(
        code.contains("function (x) {\n    return x;\n  }"),
        "{code}"
    );
    assert!(code.contains("w(0, 1)"), "{code}");
    assert!(code.contains("w(1, 1)"), "{code}");
    assert!(code.contains("function a()"), "{code}");
    assert!(code.contains("function b(c, d)"), "{code}");
    assert!(!code.contains("__DECODE_0__"), "{code}");
    assert!(
        notes
            .iter()
            .any(|note| note.contains("a function, arrow or class")),
        "{notes:?}"
    );
}

#[test]
fn the_whole_pipeline_survives_every_near_miss() {
    for (source, _survives) in NEAR_MISSES {
        let out = untwist::Pipeline::new(untwist::Options::default()).run(source);
        assert!(
            out.stages
                .iter()
                .all(|stage| stage.stop == untwist::StopReason::Converged),
            "{source}"
        );
    }
}

const CONTROL_FLOW_NEAR_MISSES: &[&str] = &[
    "function hi() { var d = ''.split('|'); var e = 0; while (!![]) { switch (d[e++]) {} break; } }",
    "function hi() { var d = '0|0'.split('|'); var e = 0; while (!![]) { switch (d[e++]) { case '0': a(); continue; } break; } }",
    "function hi() { var d = '0'.split('|'); var e = 0; while (!![]) { switch (d[++e]) { case '0': a(); continue; } break; } }",
    "function hi() { var d = '0'.split('|'); var e = 0; while (x) { switch (d[e++]) { case '0': a(); continue; } break; } }",
    "function hi() { var obj = { aaaaa: function (a, b) { return a + c; } }; return obj.aaaaa(1, 2); }",
    "function hi() { var obj = { aaaaa: 1 }; return obj.aaaaa; }",
    "function hi() { var obj = {}; obj['aaaaa'] = 1; var alias = obj; var other = obj; return alias.aaaaa; }",
    "var e = (function () { var f = !![]; return function (g, h) { return h; }; })();\ne(this, function () {})();",
    "function dpf(ret) { try { if (ret) { return 1; } } catch (y) {} }\ndpf(1);",
    "function f() { const obj = {}; obj[key] = 1; return obj; }",
    "if (\"a\" === \"a\") { let x = 1; } let x = 2;",
    "function hi() { return { aaaaa: 1 }.bbbbb; }",
    "function h(a, b) { if (a()) { p(); } else if (\"x\" === \"y\") { q(); } else { s(); } }",
    "atob?.(\"aGVsbG8=\");",
];

#[test]
fn no_control_flow_near_miss_panics() {
    for source in CONTROL_FLOW_NEAR_MISSES {
        let (code, _notes) = common::run_all_stages(source);
        assert!(!code.is_empty(), "{source}");
    }
}

#[test]
fn a_switch_flattener_failure_leaves_the_statement_list_intact() {
    let source = "function hi() { var d = '0'.split('|'); var e = 0; while (!![]) { switch (d[e++]) { case '0': a(); continue; } q(); } z(); }";
    let (code, _notes) = common::run_all_stages(source);
    assert!(code.contains("z();"), "{code}");
    assert!(code.contains("q();"), "{code}");
    assert!(code.contains("switch"), "{code}");
}

#[test]
fn an_object_key_fold_failure_leaves_the_statement_list_intact() {
    let source =
        "function hi() { var obj = {}; obj['aaaaa'] = 1; var alias = obj; other(obj); z(); }";
    let (code, _notes) = common::run_all_stages(source);
    assert!(code.contains("z();"), "{code}");
    assert!(code.contains("other(obj)"), "{code}");
    assert!(code.contains("\"aaaaa\": 1"), "{code}");
}

#[test]
fn a_merge_object_assignments_failure_leaves_the_statement_list_intact() {
    let source = "function hi() { const obj = {}; obj.foo = fn(); obj.bar = 1; z(); return obj; }";
    let (code, _notes) = common::run_all_stages(source);
    assert!(code.contains("z();"), "{code}");
    assert!(code.contains("obj.foo = fn()"), "{code}");
}

#[test]
fn a_dead_code_branch_with_a_colliding_binding_merges_by_renaming_it() {
    let source = "let x = 1;\nif (\"a\" === \"a\") { let x = 2; log(x); }";
    let (code, _notes) = common::run_all_stages(source);
    assert!(code.contains("let x = 1"), "{code}");
    assert!(code.contains("log("), "{code}");
    assert!(!code.contains("if (\""), "{code}");
}

#[test]
fn the_whole_pipeline_survives_every_control_flow_near_miss() {
    let options = untwist::Options {
        unsafe_transforms: true,
        ..untwist::Options::default()
    };
    for source in CONTROL_FLOW_NEAR_MISSES {
        let out = untwist::Pipeline::new(options.clone()).run(source);
        assert!(
            out.stages
                .iter()
                .all(|stage| stage.stop == untwist::StopReason::Converged),
            "{source}"
        );
    }
}

const CONTROL_FLOW_SHAPE_UNCHANGED_NEAR_MISSES: &[(&str, &[&str])] = &[
    (
        "function hi() { return { aaaaa: 1 }.bbbbb; }",
        &["{ aaaaa: 1 }", ".bbbbb"],
    ),
    (
        "function h(a, b) { if (a()) { p(); } else if (\"x\" === \"y\") { q(); } else { s(); } }",
        &["else if (\"x\" === \"y\")", "q();"],
    ),
    ("atob?.(\"aGVsbG8=\");", &["atob?.(\"aGVsbG8=\")"]),
];

#[test]
fn a_shape_unchanged_control_flow_near_miss_leaves_the_program_intact() {
    for (source, survives) in CONTROL_FLOW_SHAPE_UNCHANGED_NEAR_MISSES {
        let (code, _notes) = common::run_all_stages(source);

        let alloc = untwist_testing::Allocator::default();
        let reparsed = untwist_core::parse::parse_program(&alloc, &code);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{source}\n-> {code}\n{:?}",
            reparsed.diagnostics
        );

        for expected in *survives {
            assert!(
                code.contains(expected),
                "{expected} disappeared\n{source}\n-> {code}"
            );
        }
    }
}

#[test]
fn an_inline_object_literal_read_of_an_undefined_key_reports_no_control_flow_note() {
    let source = "function hi() { return { aaaaa: 1 }.bbbbb; }";
    let (_code, notes) = common::run_all_stages(source);
    assert!(
        !notes.iter().any(|note| note.contains("control-flow")),
        "{notes:?}"
    );
}
