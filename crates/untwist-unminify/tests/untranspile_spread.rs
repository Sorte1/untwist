use untwist_core::pass::{Note, PassCtor, PassOptions};
use untwist_testing::{transform_report, Allocator};
use untwist_unminify::notes;
use untwist_unminify::untranspile::{self, PASS};
use untwist_unminify::{transpile_ctors, unminify_ctors, untranspile_ctors};

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> String {
    let path = root().join("fixtures/untranspile").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn ctors<'a>() -> Vec<PassCtor<'a>> {
    let mut ctors = transpile_ctors();
    ctors.extend(unminify_ctors());
    ctors.extend(untranspile_ctors());
    ctors
}

fn run_with(source: &str, unsafe_transforms: bool, extended: bool) -> (String, Vec<Note>) {
    let alloc = Allocator::default();
    let (code, report) = transform_report(
        &alloc,
        source,
        &ctors(),
        PassOptions {
            unsafe_transforms,
            extended,
            ..PassOptions::default()
        },
    );
    (code, report.notes)
}

fn run(source: &str, unsafe_transforms: bool) -> String {
    run_with(source, unsafe_transforms, true).0
}

fn as_module(source: &str) -> String {
    format!("{source}\nexport {{}};\n")
}

fn notes_of(source: &str) -> Vec<String> {
    run_with(source, true, true)
        .1
        .into_iter()
        .filter(|note| note.pass == PASS)
        .map(|note| note.message)
        .collect()
}

#[test]
fn the_pass_is_named_extended_and_listed() {
    let pass = untranspile::boxed();
    assert_eq!(pass.name(), "untranspile-helpers");
    assert!(pass.requires_extended());
    assert_eq!(untranspile_ctors().len(), 1);
}

#[test]
fn babel_spreads_become_spread_syntax_under_unsafe() {
    let code = run(&fixture("babel/spread.7.29.js"), true);
    assert!(
        code.contains("Math.max.apply(Math, [...new Set([3, 9, 1])])"),
        "{code}"
    );
    assert!(code.contains("[...tap(1), ...tap(2)]"), "{code}");
    assert!(
        code.contains("var o2 = {\n  ...o1,\n  y: 5,\n  w: 6\n};"),
        "{code}"
    );
    assert!(
        code.contains("var o4 = {\n  ...o3,\n  ...\"pq\"\n};"),
        "{code}"
    );
    assert!(!code.contains("function _toConsumableArray"), "{code}");
    assert!(!code.contains("function _objectSpread"), "{code}");
}

#[test]
fn babel_keeps_the_runtime_dependent_spreads_in_the_default_mode() {
    let code = run(&fixture("babel/spread.7.29.js"), false);
    assert!(
        code.contains("_toConsumableArray(new Set([3, 9, 1]))"),
        "{code}"
    );
    assert!(
        code.contains("_objectSpread(_objectSpread({}, o1)"),
        "{code}"
    );
    assert!(code.contains("[...\"xy\"].join(\"-\")"), "{code}");
    assert!(code.contains("function _toConsumableArray"), "{code}");
}

#[test]
fn a_babel_array_spread_through_concat_becomes_one_literal() {
    let code = run(&fixture("babel/spread.7.29.js"), true);
    assert!(
        code.contains("var b = [].concat(a, [4], [5, 6]);"),
        "{code}"
    );
    let swc = run(&fixture("swc/spread.es5.js"), true);
    assert!(swc.contains("var b = [...a, 4, 5, 6];"), "{swc}");
    assert!(swc.contains("[...a, 10, 20]"), "{swc}");
}

#[test]
fn typescript_spread_array_keeps_holes_and_non_arrays() {
    let code = run(&fixture("ts/spread.es5.js"), true);
    assert!(code.contains("var b = [...a, 4, 5, 6];"), "{code}");
    assert!(
        code.contains("__spreadArray(__spreadArray([], a, false), a, false)"),
        "{code}"
    );
    assert!(code.contains("__spreadArray([], \"xy\", true)"), "{code}");
    assert!(
        code.contains("__spreadArray([], undefined, true)"),
        "{code}"
    );
    let iterated = run(&fixture("ts/spread.es5-iter.js"), true);
    assert!(iterated.contains("[...a, ...a]"), "{iterated}");
    assert!(iterated.contains("[...tap(1), ...tap(2)]"), "{iterated}");
}

#[test]
fn typescript_inlines_a_literal_array_in_the_default_mode() {
    let script = run(&fixture("ts/spread.es5.js"), false);
    assert!(
        script.contains("__spreadArray([undefined], [2020, 0, 2], false)"),
        "{script}"
    );
    let code = run(&as_module(&fixture("ts/spread.es5.js")), false);
    assert!(code.contains("[undefined, 2020, 0, 2]"), "{code}");
    assert!(code.contains("__spreadArray([], a, true)"), "{code}");
}

#[test]
fn esbuild_spread_values_and_props_become_object_spread() {
    let code = run(&fixture("esbuild/spread.0.28.1-es2015.js"), true);
    assert!(
        code.contains("const o2 = {\n  ...o1,\n  y: 5,\n  w: 6\n};"),
        "{code}"
    );
    assert!(!code.contains("__spreadValues("), "{code}");
}

#[test]
fn a_babel_computed_key_after_a_spread_becomes_a_computed_member() {
    let code = run(&fixture("terser/spread.safari9.js"), true);
    assert!(
        code.contains("...base,\n      [key]: 1,\n      plain: 2"),
        "{code}"
    );
    assert!(!code.contains("function _objectSpread"), "{code}");
}

#[test]
fn a_helper_whose_body_differs_is_left_alone() {
    let code = run(&fixture("hand/helper-body-differs.js"), true);
    assert!(code.contains("_toConsumableArray(list)"), "{code}");
    assert!(code.contains("__assign(__assign({}"), "{code}");
}

#[test]
fn a_reassigned_helper_is_left_alone() {
    let code = run(&fixture("hand/helper-reassigned.js"), true);
    assert_eq!(
        code.matches("_toConsumableArray(list)").count(),
        2,
        "{code}"
    );
}

#[test]
fn a_shadowing_local_function_is_not_the_helper() {
    let code = run(&fixture("hand/helper-shadowed.js"), true);
    assert!(code.contains("console.log([...list].join"), "{code}");
    assert!(code.contains("return _toConsumableArray(input);"), "{code}");
}

#[test]
fn a_source_that_may_hold_an_own_proto_is_kept_in_the_default_mode() {
    let source = fixture("hand/assign-proto-source.js");
    let safe = run(&as_module(&source), false);
    assert!(safe.contains("__assign(__assign({}, source)"), "{safe}");
    assert!(
        safe.contains("var plain = {\n  a: 1,\n  b: 2\n};"),
        "{safe}"
    );
    let unsafe_code = run(&source, true);
    assert!(unsafe_code.contains("...source"), "{unsafe_code}");
}

#[test]
fn conservative_runs_none_of_it() {
    let source = fixture("babel/spread.7.29.js");
    let (code, _) = run_with(&source, true, false);
    assert!(
        code.contains("_toConsumableArray(new Set([3, 9, 1]))"),
        "{code}"
    );
}

const PATCHED: &str = "function _arrayLikeToArray(r, a) { (null == a || a > r.length) && (a = r.length); for (var e = 0, n = Array(a); e < a; e++) n[e] = r[e]; return n; }
function _arrayWithoutHoles(r) { if (Array.isArray(r)) return _arrayLikeToArray(r); }
function _iterableToArray(r) { if (\"undefined\" != typeof Symbol && null != r[Symbol.iterator] || null != r[\"@@iterator\"]) return Array.from(r); }
function _nonIterableSpread() { throw new TypeError(\"Invalid attempt to spread non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.\"); }
function _unsupportedIterableToArray(r, a) { if (r) { if (\"string\" == typeof r) return _arrayLikeToArray(r, a); var t = {}.toString.call(r).slice(8, -1); return \"Object\" === t && r.constructor && (t = r.constructor.name), \"Map\" === t || \"Set\" === t ? Array.from(r) : \"Arguments\" === t || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(t) ? _arrayLikeToArray(r, a) : void 0; } }
function _toConsumableArray(r) { return _arrayWithoutHoles(r) || _iterableToArray(r) || _unsupportedIterableToArray(r) || _nonIterableSpread(); }
";

#[test]
fn a_program_that_patches_array_prototype_keeps_its_helpers() {
    let source = format!(
        "{PATCHED}Array.prototype.concat = function () {{ return [!0]; }};\nconsole.log(_toConsumableArray([1]).concat([2]));\n"
    );
    let code = run(&source, true);
    assert!(code.contains("_toConsumableArray([1])"), "{code}");
    assert!(notes_of(&source)
        .iter()
        .any(|note| note == notes::UNTRANSPILE_BUILTINS_WRITTEN));
}

#[test]
fn a_new_prototype_method_the_helpers_never_call_does_not_count_as_a_patch() {
    let source = format!(
        "{PATCHED}String.prototype.shout = function () {{ return this + \"!\"; }};\nconsole.log(_toConsumableArray(list()), !0);\nfunction list() {{ return [1]; }}\n"
    );
    let code = run(&source, true);
    assert!(code.contains("console.log([...list()], true);"), "{code}");
}

#[test]
fn a_direct_eval_keeps_every_helper_with_one_note() {
    let source = format!("{PATCHED}console.log(_toConsumableArray(eval(\"[1]\")), !0);\n");
    let code = run(&source, true);
    assert!(code.contains("_toConsumableArray(eval"), "{code}");
    assert!(notes_of(&source)
        .iter()
        .any(|note| note == notes::UNTRANSPILE_SCOPE_HAZARD));
}

#[test]
fn a_helper_named_like_babel_with_another_body_is_noted() {
    let source =
        "function _toConsumableArray(r) { return [r, !0]; }\nconsole.log(_toConsumableArray(1));\n";
    assert!(notes_of(source)
        .iter()
        .any(|note| note.starts_with(notes::UNTRANSPILE_UNKNOWN_HELPER)));
}

#[test]
fn helpers_inside_a_function_are_swept_in_the_default_mode() {
    let source =
        format!("(function () {{\n{PATCHED}console.log(_toConsumableArray([1, 2]), !0);\n}})();\n");
    let code = run(&source, false);
    assert!(code.contains("console.log([1, 2], true);"), "{code}");
    assert!(!code.contains("function _"), "{code}");
}

#[test]
fn script_level_helpers_stay_in_the_default_mode() {
    let source = format!("{PATCHED}console.log(_toConsumableArray([1, 2]), !0);\n");
    let code = run(&source, false);
    assert!(code.contains("console.log([1, 2], true);"), "{code}");
    assert!(code.contains("function _toConsumableArray"), "{code}");
}

#[test]
fn a_safe_site_after_unsafe_ones_still_fires_in_the_default_mode() {
    let source = format!(
        "(function () {{\n{PATCHED}console.log(_toConsumableArray(list()));\nconsole.log(_toConsumableArray(list()), !0);\nconsole.log(_toConsumableArray([3, 4]));\nfunction list() {{ return [1]; }}\n}})();\n"
    );
    let code = run(&source, false);
    assert!(code.contains("console.log([3, 4]);"), "{code}");
    assert!(code.contains("_toConsumableArray(list())"), "{code}");
}

#[test]
fn a_formatted_program_converts_its_safe_spreads_in_the_default_mode() {
    let code = run(&as_module(&fixture("hand/formatted-spread.js")), false);
    assert!(code.contains("var safeArray = [0, 3, 4];"), "{code}");
    assert!(
        code.contains("var safeObject = {\n  a: 1,\n  b: 2\n};"),
        "{code}"
    );
    assert!(
        code.contains("var array = __spreadArray(__spreadArray([], xs, true), [5], false);"),
        "{code}"
    );
}

#[test]
fn a_formatted_program_converts_every_spread_under_unsafe() {
    let code = run(&fixture("hand/formatted-spread.js"), true);
    assert!(code.contains("var array = [...xs, 5];"), "{code}");
    assert!(
        code.contains("var object = {\n  ...safeObject,\n  c: 3\n};"),
        "{code}"
    );
    assert!(!code.contains("__spreadArray"), "{code}");
    assert!(!code.contains("__spreadValues"), "{code}");
}

const CONCAT_REACHED_THROUGH_VALUES: [&str; 6] = [
    "[].__proto__.concat = function () { return [\"evil\"]; };",
    "[].constructor.prototype.concat = function () { return [\"evil\"]; };",
    "Object.getPrototypeOf([]).concat = function () { return [\"evil\"]; };",
    "(function (A) { A.prototype.concat = function () { return [\"evil\"]; }; })([].constructor);",
    "Object.defineProperty([].__proto__, \"concat\", { value: function () { return [\"evil\"]; } });",
    "if (!Array.prototype.concat) { Array.prototype.concat = 1; } [].__proto__[\"concat\"] = function () { return [\"evil\"]; };",
];

#[test]
fn a_prototype_patched_through_a_value_keeps_the_helpers_in_the_default_mode() {
    for patch in CONCAT_REACHED_THROUGH_VALUES {
        let source = format!(
            "{PATCHED}{patch}\nfunction run(a, b) {{ return [a].concat([b]).join(\",\") + \"|\" + _toConsumableArray([a, b]).length; }}\nconsole.log(run(1, 2));\n"
        );
        let code = run(&source, false);
        assert!(
            code.contains("_toConsumableArray([a, b])"),
            "{patch}\n{code}"
        );
        assert!(code.contains("[a].concat([b])"), "{patch}\n{code}");
        untwist_testing::node::assert_node_agrees(&source, &code);
    }
}

const TSLIB_ASSIGN: &str = r#"var __assign = (this && this.__assign) || function () {
    __assign = Object.assign || function(t) {
        for (var s, i = 1, n = arguments.length; i < n; i++) {
            s = arguments[i];
            for (var p in s) if (Object.prototype.hasOwnProperty.call(s, p))
                t[p] = s[p];
        }
        return t;
    };
    return __assign.apply(this, arguments);
};
"#;

const TSLIB_SPREAD_ARRAY: &str = r#"var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};"#;

fn default_lane(source: &str) -> String {
    run(source, false)
}

#[test]
fn a_shared_helper_in_a_script_whose_this_holds_one_is_left_alone_in_every_lane() {
    let source = format!(
        "this.__assign = function () {{ return {{ evil: 1 }}; }};\n{TSLIB_ASSIGN}function run(v) {{ var r = __assign({{ a: 1 }}, {{ b: v }}); return JSON.stringify(r); }}\nconsole.log(run(2));\n"
    );
    for unsafe_transforms in [false, true] {
        let code = run(&source, unsafe_transforms);
        assert!(code.contains("__assign({"), "{code}");
        untwist_testing::node::assert_node_agrees(&source, &code);
    }
}

#[test]
fn a_shared_helper_in_a_script_converts_only_under_unsafe() {
    let source = format!(
        "{TSLIB_ASSIGN}function run(v) {{ var r = __assign({{ a: 1 }}, {{ b: v }}); return JSON.stringify(r); }}\nconsole.log(run(2));\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("__assign({"), "{code}");
    let code = run(&source, true);
    assert!(code.contains("a: 1,\n    b: v\n"), "{code}");
    untwist_testing::node::assert_node_agrees(&source, &code);
}

#[test]
fn a_shared_helper_in_a_module_is_still_recognised() {
    let source = format!(
        "{TSLIB_ASSIGN}export function run(v) {{ var r = __assign({{ a: 1 }}, {{ b: v }}); return JSON.stringify(r); }}\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("a: 1,\n    b: v\n"), "{code}");
    assert!(!code.contains("__assign("), "{code}");
}

const GLOBAL_REPLACEMENTS: [&str; 5] = [
    "globalThis._toConsumableArray = function () { return [\"evil\"]; };",
    "window._toConsumableArray = function () { return [\"evil\"]; };",
    "this[\"_toConsumableArray\"] = function () { return [\"evil\"]; };",
    "var k = \"_toConsumable\" + \"Array\"; globalThis[k] = function () { return [\"evil\"]; };",
    "Object.defineProperty(globalThis, \"_toConsumableArray\", { value: function () { return [\"evil\"]; } });",
];

#[test]
fn a_script_helper_the_global_object_can_replace_is_left_alone() {
    for patch in GLOBAL_REPLACEMENTS {
        let source = format!(
            "{PATCHED}{patch}\nfunction run(a, b) {{ return _toConsumableArray([a, b]).join(\",\"); }}\n"
        );
        for unsafe_transforms in [false, true] {
            let code = run(&source, unsafe_transforms);
            assert!(
                code.contains("_toConsumableArray([a, b])"),
                "{patch}\n{code}"
            );
        }
    }
}

#[test]
fn a_script_helper_nothing_replaces_still_converts() {
    let source = format!(
        "{PATCHED}globalThis.other = 1;\nfunction run(a, b) {{ return _toConsumableArray([a, b]).join(\",\"); }}\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("return [a, b].join"), "{code}");
}

#[test]
fn a_var_helper_declared_in_a_branch_is_left_alone() {
    let source = format!(
        "if (typeof nope !== \"undefined\") {{\n{TSLIB_SPREAD_ARRAY}\n}}\nexport function run(a, b) {{ return __spreadArray([a], [b], false).length; }}\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("__spreadArray([a], [b], false)"), "{code}");
}

#[test]
fn a_var_helper_a_call_can_reach_before_it_runs_is_left_alone() {
    let source = format!(
        "console.log(run(1, 2));\n{TSLIB_SPREAD_ARRAY}\nexport function run(a, b) {{ return __spreadArray([a], [b], false).length; }}\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("__spreadArray([a], [b], false)"), "{code}");
}

#[test]
fn a_var_helper_at_the_top_of_a_module_still_converts() {
    let source = format!(
        "import x from \"x\";\nvar a = 1, b = {{ c: [2] }};\n{TSLIB_SPREAD_ARRAY}\nexport function run(a, b) {{ return __spreadArray([a], [b], false).length + x; }}\n"
    );
    let code = default_lane(&source);
    assert!(code.contains("return [a, b].length"), "{code}");
}
