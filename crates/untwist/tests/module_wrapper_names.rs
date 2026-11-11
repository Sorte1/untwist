#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const ESBUILD_HEAD: &str = "var __getOwnPropNames = Object.getOwnPropertyNames;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
";

const WEBPACK_REQUIRE: &str = "  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cachedModule = __webpack_module_cache__[moduleId];
    if (cachedModule !== undefined) return cachedModule.exports;
    var module = __webpack_module_cache__[moduleId] = { exports: {} };
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
";

const PARCEL_FIXTURE: &str = "../../fixtures/unpack-bundlers/parcel/min-parcel-hoist.js";
const PARCEL_FIRST_REGISTER: &str = "parcelRegister(\"d4t4A\"";

fn unpacked(bundle: &str, module: &str) -> Unpacked {
    let tree = Unpacked::of(bundle, false);
    assert!(
        tree.has(module),
        "not unpacked: {:?}\n{bundle}",
        tree.notes()
    );
    tree
}

fn esbuild(body: &str) -> String {
    format!(
        "{ESBUILD_HEAD}var require_a = __commonJS({{\n  \"a.js\"(exports, module) {{ {body} }}\n}});\nconsole.log(require_a());\n"
    )
}

fn parcel(body: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(PARCEL_FIXTURE);
    let fixture = std::fs::read_to_string(&path).expect("parcel fixture");
    let head = fixture
        .split(PARCEL_FIRST_REGISTER)
        .next()
        .expect("prelude");
    format!(
        "{head}parcelRegister(\"a\", function(module, exports) {{ {body} }});\nconsole.log(parcelRequire(\"a\"));\n"
    )
}

#[test]
fn an_esbuild_factory_with_lexical_wrapper_names_runs_like_the_bundle() {
    let bundle = esbuild(
        "let require = \"local require\"; const __dirname = \"local dirname\"; module.exports = require + \"/\" + __dirname;",
    );
    unpacked(&bundle, "a.js").runs_like_the_bundle("local require/local dirname\n");
}

#[test]
fn an_esbuild_factory_with_uninitialised_wrapper_vars_reads_undefined() {
    let bundle = esbuild(
        "var require, __filename; module.exports = typeof require + \"/\" + typeof __filename;",
    );
    unpacked(&bundle, "a.js").runs_like_the_bundle("undefined/undefined\n");
}

#[test]
fn an_esbuild_factory_var_that_redeclares_its_own_parameter_keeps_it() {
    let bundle = esbuild("var exports; exports.a = 1; module.exports = typeof exports.a;");
    unpacked(&bundle, "a.js").runs_like_the_bundle("number\n");
}

#[test]
fn an_unwrapped_esbuild_entry_with_wrapper_names_runs_like_the_bundle() {
    let bundle = format!(
        "(() => {{\n{ESBUILD_HEAD}var require_a = __commonJS({{\n  \"a.js\"(exports, module) {{ module.exports = \"module a\"; }}\n}});\nlet exports = require_a();\nconst module = {{ tag: \"local module\" }};\nconsole.log(exports, module.tag);\n}})();\n"
    );
    unpacked(&bundle, "a.js").runs_like_the_bundle("module a local module\n");
}

#[test]
fn a_parcel_factory_with_lexical_wrapper_names_runs_like_the_bundle() {
    let bundle = parcel(
        "let require = \"local require\"; const __filename = \"local filename\"; module.exports = require + \"/\" + __filename;",
    );
    unpacked(&bundle, "a.js").runs_like_the_bundle("local require/local filename\n");
}

#[test]
fn a_parcel_factory_with_an_uninitialised_require_var_reads_undefined() {
    let bundle = parcel("var require; module.exports = typeof require;");
    unpacked(&bundle, "a.js").runs_like_the_bundle("undefined\n");
}

#[test]
fn webpack_factories_and_inline_entry_with_wrapper_names_run_like_the_bundle() {
    let bundle = format!(
        "(() => {{
  var __webpack_modules__ = {{
    1: () => {{ let require = \"local require\"; const __filename = \"local filename\"; class __dirname {{}} console.log(\"module 1\", require, __filename, typeof __dirname); }},
    2: (module) => {{ let exports = \"local exports\"; module.exports = exports; }}
  }};
{WEBPACK_REQUIRE}  var __webpack_exports__ = {{}};
  (() => {{ const module = {{ tag: \"local module\" }}; let exports = __webpack_require__(2); __webpack_require__(1); console.log(\"entry\", module.tag, exports); }})();
}})();
"
    );
    unpacked(&bundle, "1.js").runs_like_the_bundle(
        "module 1 local require local filename function\nentry local module local exports\n",
    );
}

#[test]
fn a_webpack_factory_with_an_uninitialised_module_var_reads_undefined() {
    let bundle = format!(
        "(() => {{
  var __webpack_modules__ = {{
    1: (unused, exports) => {{ var module; exports.seen = typeof module; }}
  }};
{WEBPACK_REQUIRE}  var __webpack_exports__ = {{}};
  (() => {{ console.log(__webpack_require__(1).seen); }})();
}})();
"
    );
    unpacked(&bundle, "1.js").runs_like_the_bundle("undefined\n");
}

#[test]
fn a_browserify_module_with_wrapper_names_runs_like_the_bundle() {
    let bundle = "(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c=\"function\"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error(\"Cannot find module '\"+i+\"'\");throw a.code=\"MODULE_NOT_FOUND\",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u=\"function\"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({1:[function(require,module,exports){let __filename = \"local filename\"; const __dirname = \"local dirname\"; console.log(\"module 1\", __filename, __dirname, require(\"./b\"));},{\"./b\":2}],2:[function(require,module,exports){module.exports = \"b\";},{}]},{},[1]);\n";
    unpacked(bundle, "b.js").runs_like_the_bundle("module 1 local filename local dirname b\n");
}

const WEBPACK4_LOCAL_REQUIRE: &str = r#"(function(modules){
  var installedModules = {};
  function __webpack_require__(moduleId){
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  __webpack_require__.d = function(exports, name, getter){ if(!__webpack_require__.o(exports, name)) Object.defineProperty(exports, name, { enumerable: true, get: getter }); };
  __webpack_require__.r = function(exports){ Object.defineProperty(exports, '__esModule', { value: true }); };
  __webpack_require__.n = function(module){ var getter = module && module.__esModule ? function getDefault(){ return module['default']; } : function getModuleExports(){ return module; }; __webpack_require__.d(getter, 'a', getter); return getter; };
  __webpack_require__.o = function(object, property){ return Object.prototype.hasOwnProperty.call(object, property); };
  return __webpack_require__(__webpack_require__.s = 0);
})([
function(module, exports, __webpack_require__) { var lib = __webpack_require__(1); console.log(JSON.stringify(lib), typeof lib === "function" ? lib("q") : ""); },
function(module, exports, __webpack_require__) {var table = { "./utils": function () { return "utils"; }, 7: function () { return "seven"; }, 1: function () { return "one"; } }; var require = function (n) { return table[n](); }; exports.v = require("./utils") + require(1); }
]);
"#;

#[test]
fn a_webpack_factory_var_named_require_still_runs_after_unminify() {
    let tree = Unpacked::with(WEBPACK4_LOCAL_REQUIRE, untwist::Options::default());
    assert!(tree.has("1.js"), "not unpacked: {:?}", tree.notes());
    assert!(
        !tree.file("1.js").contains("const require"),
        "{}",
        tree.file("1.js")
    );
    tree.runs_like_the_bundle("{\"v\":\"utilsone\"} \n");
}
