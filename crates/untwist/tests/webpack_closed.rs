#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const WHOLE_PROGRAM: &str = "deobfuscated.js";
const REFUSAL: &str = "module 0 is not closed over its own scope; no bundle is written";

const READS_ITS_SURROUNDINGS: &str = r#"var greeting = "hello";
console.log("before bundle");
function boot(local) {
  return (function (modules) {
    var installedModules = {};
    function __webpack_require__(moduleId) {
      if (installedModules[moduleId]) return installedModules[moduleId].exports;
      var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });
      modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
      module.l = true;
      return module.exports;
    }
    return __webpack_require__((__webpack_require__.s = 0));
  })([
    function (module, exports, __webpack_require__) {
      module.exports = greeting + " " + local + " " + __webpack_require__(1);
    },
    function (module, exports) {
      module.exports = "world";
    },
  ]);
}
console.log(boot("A"), boot("B"));
console.log("after bundle");
"#;

const BROWSERIFY_READS_ITS_SURROUNDINGS: &str = r#"var greeting = "hello";
(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c="function"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error("Cannot find module '"+i+"'");throw a.code="MODULE_NOT_FOUND",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u="function"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({
  1: [function (require, module, exports) {
    console.log(greeting + " " + require("./world"));
  }, { "./world": 2 }],
  2: [function (require, module, exports) {
    module.exports = "world";
  }, {}]
}, {}, [1]);
"#;

fn refused(bundle: &str, id: &str, expected: &str) {
    let unpacked = Unpacked::of(bundle, false);
    assert!(!unpacked.has("index.js"), "{:?}", unpacked.notes());
    assert!(!unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    let refusal = REFUSAL.replace("module 0", &format!("module {id}"));
    assert!(
        unpacked.notes().iter().any(|note| note.contains(&refusal)),
        "{:?}",
        unpacked.notes()
    );
    unpacked.file_runs_like_the_bundle(WHOLE_PROGRAM, expected);
}

#[test]
fn a_webpack_bundle_whose_factory_reads_the_scope_around_it_is_not_unpacked() {
    refused(
        READS_ITS_SURROUNDINGS,
        "0",
        "before bundle\nhello A world hello B world\nafter bundle\n",
    );
}

#[test]
fn a_browserify_bundle_whose_factory_reads_the_scope_around_it_is_not_unpacked() {
    refused(BROWSERIFY_READS_ITS_SURROUNDINGS, "1", "hello world\n");
}

#[test]
fn the_reference_lane_still_writes_the_modules() {
    let unpacked = Unpacked::of(READS_ITS_SURROUNDINGS, true);
    assert!(unpacked.has("index.js"));
    assert!(unpacked.has("bundle.json"));
}

const ARROW_FACTORIES_SHARE_THIS: &str = r#"(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  return __webpack_require__((__webpack_require__.s = 0));
})([
  (module, exports, __webpack_require__) => { this.tag = "t"; console.log(__webpack_require__(1).read()); },
  (module, exports) => { exports.read = () => this.tag; },
]);
"#;

#[test]
fn a_webpack_bundle_whose_arrow_factory_uses_the_bundles_this_is_not_unpacked() {
    refused(ARROW_FACTORIES_SHARE_THIS, "0", "t\n");
}

const CLOSED_EVAL_FACTORY: &str = r#"(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  return __webpack_require__((__webpack_require__.s = 0));
})([
  function (e, t, n) {
    var seen;
    try { seen = typeof eval("require;").cache; } catch (error) { seen = error.name; }
    t.marker = 1;
    console.log(seen, eval("typeof exports.marker"), n(1));
  },
  function (e) { e.exports = "world"; },
]);
"#;

const OPEN_EVAL_FACTORY: &str = r#"(function (modules) {
  var installedModules = {};
  function __webpack_require__(moduleId) {
    if (installedModules[moduleId]) return installedModules[moduleId].exports;
    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });
    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
    module.l = true;
    return module.exports;
  }
  return __webpack_require__((__webpack_require__.s = 0));
})([
  function (e, t, n) { eval("var made = n(1)"); console.log(made); },
  function (e) { e.exports = "world"; },
]);
"#;

#[test]
fn a_webpack_factory_whose_eval_reads_only_globals_is_unpacked_without_capturing_them() {
    let unpacked = Unpacked::of(CLOSED_EVAL_FACTORY, false);
    assert!(unpacked.has("bundle.json"), "{:?}", unpacked.notes());
    unpacked.runs_like_the_bundle("object undefined world\n");
}

#[test]
fn a_webpack_factory_whose_eval_declares_a_var_is_not_unpacked() {
    refused(OPEN_EVAL_FACTORY, "0", "world\n");
}
