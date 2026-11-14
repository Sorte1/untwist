use untwist::{Options, Pipeline};
use untwist_testing::node_rows::{self, node_or_skip, Mode};

const FOLDED: &str =
    "console.log([1, 2].join('+'), 'abc'.split('').reverse().join(''), [3, 4].join());";

const HAZARDS: [(&str, &str); 6] = [
    (
        "a direct eval reassigns the declared object",
        r#"var o = {k: "x"}; eval("o = {k: 'join'}"); Array.prototype[o.k] = function () { return "X"; };"#,
    ),
    (
        "a direct eval in a nested function reassigns the declared object",
        r#"var o = {k: "x"}; (function () { eval("o = {k: 'join'}"); })(); Array.prototype[o.k] = function () { return "X"; };"#,
    ),
    (
        "a direct eval reads the object through a computed name",
        r#"var o = {k: "x"}; var n = "o"; eval(n + ".k = 'join'"); Array.prototype[o.k] = function () { return "X"; };"#,
    ),
    (
        "a with statement shadows the declared object",
        r#"var o = {k: "x"}; with ({o: {k: "join"}}) { Array.prototype[o.k] = function () { return "X"; }; }"#,
    ),
    (
        "a with statement shadows the declared object in a function",
        r#"var o = {k: "x"}; function f(s) { with (s) { return o.k; } } Array.prototype[f({o: {k: "join"}})] = function () { return "X"; };"#,
    ),
    (
        "no hazard",
        r#"var o = {k: "x"}; Array.prototype[o.k] = function () { return "X"; }; console.log(o.k);"#,
    ),
];

fn transformed(options: &Options, source: &str) -> String {
    Pipeline::new(options.clone())
        .run(source)
        .code()
        .to_string()
}

#[test]
fn an_object_a_scope_hazard_may_reach_is_not_inlined() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let safe = Options::default();
    let unsafe_transforms = Options {
        unsafe_transforms: true,
        ..Options::default()
    };
    let conservative = Options {
        extended: false,
        ..Options::default()
    };
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let run_conservative = |source: &str| transformed(&conservative, source);
    let modes: [Mode<'_>; 3] = [
        ("default", &run_safe),
        ("unsafe", &run_unsafe),
        ("conservative", &run_conservative),
    ];
    let programs: Vec<(String, String)> = HAZARDS
        .iter()
        .map(|(label, patch)| (label.to_string(), format!("{patch}\n{FOLDED}\n")))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    let (wrong, witnesses) = node_rows::unsafe_dynamic_code_witnesses(wrong, &programs);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(witnesses.is_empty(), "{witnesses:?}");
}

const KEYED_HAZARDS: [(&str, &str); 10] = [
    (
        "a variable-key callee passes the object as this",
        r#"function run(pick) { var O = {a: 1, b: 2}; var key = pick(); O[key]().a = 5; return [O.a, O.b].join(","); }
console.log(run(function () { return "valueOf"; }));"#,
    ),
    (
        "a variable-key tag passes the object as this",
        r#"function run(key) { var O = {a: 1, b: 2}; O[key]``.a = 6; return [O.a, O.b].join(","); }
console.log(run("valueOf"));"#,
    ),
    (
        "a variable-key write then a static read",
        r#"function run(key) { var O = {a: 1, b: 2}; O[key] = 9; return [O.a, O.b].join(","); }
console.log(run("a"));"#,
    ),
    (
        "a variable-key delete then a static read",
        r#"function run(key) { var O = {a: 1, b: 2}; var kept = O.b; delete O[key]; return [kept, O.a].join(","); }
console.log(run("a"));"#,
    ),
    (
        "a constant-key delete then a static read",
        r#"function run() { const key = "a"; var O = {a: 1, b: 2}; var kept = O.b; delete O[key]; return [kept, O.a].join(","); }
console.log(run());"#,
    ),
    (
        "for-in over the object after a static read",
        r#"function run() { var O = {a: 1, b: 2}; var seen = []; for (var key in O) { seen.push(key + "=" + O[key]); } return seen.join(",") + ";" + O.a; }
console.log(run());"#,
    ),
    (
        "a constant key read before its let initialises",
        r#"function run() { var O = {a: 1, b: 2}; var read; try { read = O[key]; } catch (error) { read = error.constructor.name; } const key = "a"; return [read, O.b].join(","); }
console.log(run());"#,
    ),
    (
        "a constant key read before its var initialises",
        r#"function run() { var O = {a: 1, b: 2}; var read = O[key]; var key = "a"; return [String(read), O.b].join(","); }
console.log(run());"#,
    ),
    (
        "a script-level constant key rewritten through the global object",
        r#"var key = "a"; function run() { var O = {a: 1, b: 2}; var read = O[key]; return [read, O.b].join(","); }
globalThis.key = "b"; console.log(run());
if (typeof require === "function" && typeof module === "object") { require("vm").runInThisContext(require("fs").readFileSync(__filename, "utf8")); }"#,
    ),
    (
        "a key whose toString writes the object through the global object",
        r#"var O = {a: 1, b: 2}; var key = {toString: function () { if (globalThis.O) { globalThis.O.a = 7; } return "b"; }}; var read = O[key]; console.log([read, O.a, O.b].join(","));
if (typeof require === "function" && typeof module === "object") { require("vm").runInThisContext(require("fs").readFileSync(__filename, "utf8")); }"#,
    ),
];

const INHERITED_GETTER: &str = r#"Object.defineProperty(Object.prototype, "probe", {configurable: true, get: function () { this.a = 3; return 0; }});
function run(key) { var O = {a: 1, b: 2}; var read = O[key]; return [read, O.a, O.b].join(","); }
console.log(run("probe"));"#;

fn three_modes() -> [Options; 3] {
    [
        Options::default(),
        Options {
            unsafe_transforms: true,
            ..Options::default()
        },
        Options {
            extended: false,
            ..Options::default()
        },
    ]
}

#[test]
fn a_variable_key_read_keeps_the_object_only_where_nothing_can_tell() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let [safe, unsafe_transforms, conservative] = three_modes();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_unsafe = |source: &str| transformed(&unsafe_transforms, source);
    let run_conservative = |source: &str| transformed(&conservative, source);
    let modes: [Mode<'_>; 3] = [
        ("default", &run_safe),
        ("unsafe", &run_unsafe),
        ("conservative", &run_conservative),
    ];
    let programs: Vec<(String, String)> = KEYED_HAZARDS
        .iter()
        .map(|(label, source)| (label.to_string(), format!("{source}\n")))
        .collect();
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_inherited_getter_is_the_unsafe_assumption() {
    let Some(node) = node_or_skip() else {
        return;
    };
    let [safe, unsafe_transforms, conservative] = three_modes();
    let run_safe = |source: &str| transformed(&safe, source);
    let run_conservative = |source: &str| transformed(&conservative, source);
    let modes: [Mode<'_>; 2] = [("default", &run_safe), ("conservative", &run_conservative)];
    let programs = vec![(
        "inherited getter".to_string(),
        format!("{INHERITED_GETTER}\n"),
    )];
    let wrong = node_rows::mismatches(&node, &programs, &modes);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    let out = transformed(&unsafe_transforms, INHERITED_GETTER);
    assert!(
        out.contains("[read, 1, 2]"),
        "--unsafe no longer inlines the static reads beside a variable-key read: {out}"
    );
}

#[test]
fn static_reads_beside_a_variable_key_read_inline_under_unsafe() {
    let source = "function modes(mode) { var MODES = {POINTS: 0, LINES: 1}; return [MODES.LINES, MODES[mode]]; }";
    let [safe, unsafe_transforms, conservative] = three_modes();
    for (label, options) in [("default", &safe), ("conservative", &conservative)] {
        let out = transformed(options, source);
        assert!(out.contains("MODES.LINES"), "{label} inlined a read: {out}");
    }
    let out = transformed(&unsafe_transforms, source);
    assert!(
        !out.contains("MODES.LINES") && out.contains("POINTS: 0") && out.contains("[mode]"),
        "--unsafe should inline the static read and keep the map for the variable-key read: {out}"
    );
}

#[test]
fn a_constant_literal_key_resolves_outside_conservative() {
    let source = "function pick() { const key = \"b\"; var TABLE = {a: \"first\", b: \"second\"}; return [TABLE[key], TABLE.a]; }";
    let [safe, unsafe_transforms, conservative] = three_modes();
    for (label, options) in [("default", &safe), ("unsafe", &unsafe_transforms)] {
        let out = transformed(options, source);
        assert!(
            out.contains("[\"second\", \"first\"]") && !out.contains("TABLE"),
            "{label} did not resolve the constant key and inline the map: {out}"
        );
    }
    let out = transformed(&conservative, source);
    assert!(
        out.contains("TABLE[key]"),
        "--conservative resolved the key: {out}"
    );
}
