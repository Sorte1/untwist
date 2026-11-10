#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const WHOLE_PROGRAM: &str = "deobfuscated.js";
const PRELUDE_REFUSED: &str = "is not proven to be webpack's runtime; no bundle is written";
const REQUIRE_REFUSED: &str =
    "uses its require function in a way the unpacked files cannot share; no bundle is written";
const MERGE_REFUSED: &str = "two webpack runtimes differ; no bundle is written";
const STOCK_O: &str =
    "__webpack_require__.o = function (a, b) { return Object.prototype.hasOwnProperty.call(a, b); };";
const USE_O: &str =
    r#"console.log("o:", __webpack_require__.o({ a: 1 }, "a"), __webpack_require__(1));"#;
const M1: &str = r#"module.exports = "m1";"#;
const STOCK_REQUIRE: &str = "function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }";

fn with_require(require: &str, prelude: &str, first: &str, second: &str) -> String {
    format!(
        "(function (modules) {{
  var installedModules = {{}};
  {require}
{prelude}
  return __webpack_require__(__webpack_require__.s = 0);
}})([
  function (module, exports, __webpack_require__) {{
{first}
  }},
  function (module, exports, __webpack_require__) {{
{second}
  }}
]);
"
    )
}

fn bundle(prelude: &str, first: &str, second: &str) -> String {
    with_require(STOCK_REQUIRE, prelude, first, second)
}

fn patched(extra: &str) -> String {
    bundle(&format!("{STOCK_O}\n{extra}"), USE_O, M1)
}

fn refused_with(source: &str, refusal: &str, expected: &str) {
    let unpacked = Unpacked::of(source, false);
    assert!(!unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    assert!(
        unpacked.notes().iter().any(|note| note.contains(refusal)),
        "{:?}",
        unpacked.notes()
    );
    unpacked.file_runs_like_the_bundle(WHOLE_PROGRAM, expected);
}

fn refused(source: &str, expected: &str) {
    refused_with(source, PRELUDE_REFUSED, expected);
}

fn unpacked_like_the_bundle(source: &str, expected: &str) -> Unpacked {
    let unpacked = Unpacked::of(source, false);
    assert!(unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    unpacked.runs_like_the_bundle(expected);
    unpacked
}

#[test]
fn the_stock_runtime_still_unpacks_and_routes_its_helper() {
    let unpacked = unpacked_like_the_bundle(&patched(""), "o: true m1\n");
    assert!(unpacked.has("_runtime/webpack.js"));
}

#[test]
fn a_helper_redefined_through_define_property_is_refused() {
    refused(
        &patched(
            r#"Object.defineProperty(__webpack_require__, "o", { value: function () { return "patched"; } });"#,
        ),
        "o: patched m1\n",
    );
}

#[test]
fn a_helper_written_through_an_alias_of_the_require_function_is_refused() {
    refused(
        &patched(r#"var r = __webpack_require__; r.o = function () { return "patched"; };"#),
        "o: patched m1\n",
    );
}

#[test]
fn a_helper_written_from_a_nested_function_is_refused() {
    refused(
        &patched(
            r#"function patch() { __webpack_require__.o = function () { return "patched"; }; } patch();"#,
        ),
        "o: patched m1\n",
    );
}

#[test]
fn a_helper_written_under_an_unknown_condition_is_refused() {
    refused(
        &patched(
            r#"if (installedModules) __webpack_require__.o = function () { return "patched"; };"#,
        ),
        "o: patched m1\n",
    );
}

#[test]
fn a_helper_assigned_twice_is_refused() {
    let prelude = r#"(function () { __webpack_require__.o = function () { return "first"; }; })();
__webpack_require__.o = function () { return "second"; };"#;
    refused(&bundle(prelude, USE_O, M1), "o: second m1\n");
}

#[test]
fn a_helper_behind_an_early_return_is_refused() {
    refused(
        &patched(
            r#"(function () { if (installedModules) return; __webpack_require__.o = function () { return "never"; }; })();"#,
        ),
        "o: true m1\n",
    );
}

#[test]
fn a_helper_inside_a_generator_is_refused() {
    refused(
        &patched(
            r#"(function* () { __webpack_require__.o = function () { return "never"; }; })();"#,
        ),
        "o: true m1\n",
    );
}

#[test]
fn a_helper_inside_an_async_function_is_refused() {
    refused(
        &patched(
            r#"(async function () { await 0; __webpack_require__.o = function () { return "late"; }; })();"#,
        ),
        "o: true m1\n",
    );
}

#[test]
fn a_helper_written_under_a_computed_key_is_refused() {
    refused(
        &patched(r#"__webpack_require__["o"] = function () { return "patched"; };"#),
        "o: patched m1\n",
    );
}

#[test]
fn a_prelude_statement_with_an_effect_is_refused() {
    let prelude = r#"console.log("prelude ran");
globalThis.booted = "yes";"#;
    let first = r#"console.log("booted:", globalThis.booted, __webpack_require__(1));"#;
    refused(&bundle(prelude, first, M1), "prelude ran\nbooted: yes m1\n");
}

#[test]
fn a_require_function_without_the_module_cache_is_refused() {
    let require = r#"function __webpack_require__(moduleId) {
    var module = { exports: {} };
    modules[moduleId](module, module.exports, __webpack_require__);
    return "wrapped<" + module.exports + ">";
  }"#;
    let first = "console.log(__webpack_require__(1), __webpack_require__(1));";
    let second = r#"globalThis.n = (globalThis.n || 0) + 1;
    module.exports = "run " + globalThis.n;"#;
    refused(
        &with_require(require, "", first, second),
        "wrapped<run 1> wrapped<run 2>\n",
    );
}

#[test]
fn a_bundle_defining_its_own_d_keeps_the_module_on_that_helper() {
    let prelude = r#"__webpack_require__.r = function (exports) { Object.defineProperty(exports, "__esModule", { value: true }); };
__webpack_require__.d = function (exports, name, getter) { exports[name] = "not what you think"; };"#;
    let first = "var m = __webpack_require__(1); console.log(m.value);";
    let second = r#""use strict";
    __webpack_require__.r(exports);
    __webpack_require__.d(exports, "value", function () { return value; });
    var value = 7;"#;
    unpacked_like_the_bundle(&bundle(prelude, first, second), "not what you think\n");
}

#[test]
fn a_module_writing_a_property_other_modules_read_is_refused() {
    let first = r#"__webpack_require__.foo = 42; console.log(__webpack_require__.o({a:1},"a"), __webpack_require__(1));"#;
    let second = r#"module.exports = "m1 sees " + __webpack_require__.foo;"#;
    refused_with(
        &bundle(STOCK_O, first, second),
        REQUIRE_REFUSED,
        "true m1 sees 42\n",
    );
}

#[test]
fn a_module_passing_its_require_function_on_is_refused() {
    let first = r#"var keep = __webpack_require__; console.log(keep.o({a:1},"a"), __webpack_require__(1));"#;
    refused_with(&bundle(STOCK_O, first, M1), REQUIRE_REFUSED, "true m1\n");
}

#[test]
fn a_module_overwriting_a_routed_helper_still_shares_it() {
    let first = r#"__webpack_require__.o = function () { return "patched by m0"; }; console.log(__webpack_require__(1));"#;
    let second = r#"module.exports = "m1 sees " + __webpack_require__.o({a:1},"a");"#;
    unpacked_like_the_bundle(&bundle(STOCK_O, first, second), "m1 sees patched by m0\n");
}

#[test]
fn a_helper_reading_a_name_around_the_bundle_is_refused() {
    let source = format!(
        "var secret = \"outer secret\";\n{}",
        bundle(
            "__webpack_require__.x = function () { return secret; };",
            r#"console.log("x:", __webpack_require__.x(), __webpack_require__(1));"#,
            M1,
        )
    );
    refused(&source, "x: outer secret m1\n");
}

#[test]
fn a_helper_reading_the_host_require_is_refused() {
    let prelude = r#"__webpack_require__.x = function () { return require.main === module ? "is main" : typeof require; };"#;
    let first = r#"console.log("x:", __webpack_require__.x(), __webpack_require__(1));"#;
    refused_with(
        &bundle(prelude, first, M1),
        REQUIRE_REFUSED,
        "x: is main m1\n",
    );
}

#[test]
fn an_amd_define_that_takes_no_arguments_still_unpacks() {
    let first = r#"console.log(__webpack_require__(1));"#;
    let second = r#"var result;
    if ((result = function () { return "amd value"; }.call(exports, __webpack_require__, exports, module)) !== undefined) {
      module.exports = result;
    }"#;
    unpacked_like_the_bundle(&bundle(STOCK_O, first, second), "amd value\n");
}

#[test]
fn an_arrow_helper_reading_the_prelude_this_and_arguments_is_refused() {
    let prelude =
        r#"__webpack_require__.x = () => arguments.length + ":" + (this === globalThis);"#;
    let first = r#"console.log("x:", __webpack_require__.x(), __webpack_require__(1));"#;
    refused(&bundle(prelude, first, M1), "x: 1:true m1\n");
}

#[test]
fn a_helper_whose_value_runs_code_is_refused() {
    let prelude =
        r#"__webpack_require__.x = (function () { console.log("helper init"); return 1; })();"#;
    let first = r#"console.log("m0 start"); console.log(__webpack_require__(1));"#;
    let second = r#"module.exports = "m1 " + __webpack_require__.x;"#;
    refused(
        &bundle(prelude, first, second),
        "helper init\nm0 start\nm1 1\n",
    );
}

#[test]
fn two_inputs_with_different_runtimes_are_not_merged() {
    let first = bundle(STOCK_O, USE_O, M1);
    let second = r#"(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  __webpack_require__.o = function (a, b) { return "B's own o"; };
  __webpack_require__.q = function () { return "B's q"; };
  return __webpack_require__(__webpack_require__.s = "b0");
})({
  b0: function (module, exports, __webpack_require__) {
    console.log("B o:", __webpack_require__.o({ a: 1 }, "a"), __webpack_require__.q(), __webpack_require__("b1"));
  },
  b1: function (module, exports, __webpack_require__) {
    module.exports = "b1";
  }
});
"#;
    let unpacked = Unpacked::of_many(&[&first, second], false);
    assert!(!unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    assert!(
        unpacked
            .notes()
            .iter()
            .any(|note| note.contains(MERGE_REFUSED)),
        "{:?}",
        unpacked.notes()
    );
}

#[test]
fn two_inputs_with_the_same_runtime_still_merge() {
    let first = bundle(STOCK_O, USE_O, M1);
    let second = r#"(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  __webpack_require__.o = function (a, b) { return Object.prototype.hasOwnProperty.call(a, b); };
  return __webpack_require__(__webpack_require__.s = "b0");
})({
  b0: function (module, exports, __webpack_require__) {
    console.log("B o:", __webpack_require__.o({ a: 1 }, "a"));
  }
});
"#;
    let unpacked = Unpacked::of_many(&[&first, second], false);
    assert!(unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    assert!(unpacked.has("b0.js"), "{:?}", unpacked.notes());
}
