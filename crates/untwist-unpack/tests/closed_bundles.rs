mod common;

use common::cpu_or_wall;
use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack_inputs, BundleKind, UnpackOptions};

const NOT_CLOSED: &str = "is not closed over its own scope; no bundle is written";
const REFUSED_SHAPES: usize = 20_000;
const LINEAR_FACTOR: u32 = 5;

const WEBPACK_4: &str = r#"
var greeting = "hello";
function boot(local) {
  return (function (modules) {
    var installedModules = {};
    function __webpack_require__(moduleId) {
      if (installedModules[moduleId]) return installedModules[moduleId].exports;
      var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
      modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
      module.l = true;
      return module.exports;
    }
    return __webpack_require__(__webpack_require__.s = 0);
  })([
    function (module, exports, __webpack_require__) { module.exports = BODY; },
    function (module, exports) { module.exports = "world"; }
  ]);
}
console.log(boot("A"));
"#;

const WEBPACK_5: &str = r#"
var greeting = "hello";
(() => {
  var __webpack_modules__ = {
    2: (module, exports, __webpack_require__) => { module.exports = BODY; },
    3: (module) => { module.exports = "world"; }
  };
  var __webpack_module_cache__ = {};
  function __webpack_require__(moduleId) {
    var cached = __webpack_module_cache__[moduleId];
    if (cached !== undefined) return cached.exports;
    var module = __webpack_module_cache__[moduleId] = { exports: {} };
    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);
    return module.exports;
  }
  console.log(__webpack_require__(__webpack_require__.s = 2));
})();
"#;

const CHUNK: &str = r#"
var greeting = "hello";
(window.webpackJsonp = window.webpackJsonp || []).push([[15], {
  a: function (module, exports, __webpack_require__) { module.exports = BODY; },
  b: function (module) { module.exports = "world"; }
}]);
"#;

const BROWSERIFY: &str = r#"
var greeting = "hello";
(function(){function r(e,n,t){function o(i,f){if(!n[i]){if(!e[i]){var c="function"==typeof require&&require;if(!f&&c)return c(i,!0);if(u)return u(i,!0);var a=new Error("Cannot find module '"+i+"'");throw a.code="MODULE_NOT_FOUND",a}var p=n[i]={exports:{}};e[i][0].call(p.exports,function(r){var n=e[i][1][r];return o(n||r)},p,p.exports,r,e,n,t)}return n[i].exports}for(var u="function"==typeof require&&require,i=0;i<t.length;i++)o(t[i]);return o}return r})()({
  1: [function (require, module, exports) { module.exports = BODY; }, { "./world": 2 }],
  2: [function (require, module, exports) { module.exports = "world"; }, {}]
}, {}, [1]);
"#;

const SHAPES: [(&str, BundleKind, &str); 4] = [
    (WEBPACK_4, BundleKind::Webpack, "0"),
    (WEBPACK_5, BundleKind::Webpack, "2"),
    (CHUNK, BundleKind::Webpack, "a"),
    (BROWSERIFY, BundleKind::Browserify, "1"),
];

const CLOSED_BODIES: [&str; 3] = [
    "\"hello\"",
    "typeof window + String(Math.max(1, 2))",
    "(function () { var greeting = arguments.length; return greeting; })()",
];

const OPEN_BODIES: [(&str, &str); 3] = [
    (
        "greeting",
        "it reads greeting, a binding of the bundle's scope",
    ),
    ("arguments.length", "it reads the factory's own arguments"),
    ("eval(\"1\")", "it calls eval directly"),
];

const CONSERVATIVE: UnpackOptions = UnpackOptions {
    split_hoisted: false,
    conservative: true,
};

struct Outcome {
    kind: Option<BundleKind>,
    notes: Vec<String>,
    unchanged: bool,
}

fn outcome(source: &str, options: UnpackOptions) -> Outcome {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let before = pretty_codegen(&parsed.program);
    let mut programs = vec![parsed.program];
    let (bundle, notes) = unpack_inputs(&alloc, &mut programs, &[], options);
    Outcome {
        kind: bundle.map(|bundle| bundle.kind),
        notes: notes.into_iter().map(|note| note.message).collect(),
        unchanged: pretty_codegen(&programs[0]) == before,
    }
}

#[test]
fn a_bundle_of_closed_factories_is_taken() {
    for (shape, kind, _) in SHAPES {
        for body in CLOSED_BODIES {
            let found = outcome(&shape.replace("BODY", body), UnpackOptions::default());
            assert_eq!(found.kind, Some(kind), "{body}\n{shape}");
            assert!(
                !found.notes.iter().any(|note| note.contains(NOT_CLOSED)),
                "{body}: {:?}",
                found.notes
            );
        }
    }
}

#[test]
fn a_factory_that_is_not_closed_refuses_the_bundle_and_names_what_it_reads() {
    for (shape, _, id) in SHAPES {
        for (body, why) in OPEN_BODIES {
            let found = outcome(&shape.replace("BODY", body), UnpackOptions::default());
            assert_eq!(found.kind, None, "{body}\n{shape}");
            assert_eq!(
                found.notes,
                [format!("module {id} {NOT_CLOSED}: {why}")],
                "{body}\n{shape}"
            );
            assert!(found.unchanged, "{body}\n{shape}");
        }
    }
}

#[test]
fn a_parameter_of_the_function_around_the_bundle_is_an_outer_binding() {
    let found = outcome(
        &WEBPACK_4.replace("BODY", "local"),
        UnpackOptions::default(),
    );
    assert_eq!(found.kind, None);
    assert_eq!(
        found.notes,
        [format!(
            "module 0 {NOT_CLOSED}: it reads local, a binding of the bundle's scope"
        )]
    );
}

#[test]
fn a_binding_of_the_bundles_own_prelude_is_an_outer_binding() {
    let found = outcome(
        &WEBPACK_5.replace("BODY", "__webpack_module_cache__[3]"),
        UnpackOptions::default(),
    );
    assert_eq!(found.kind, None);
    assert_eq!(
        found.notes,
        [format!(
            "module 2 {NOT_CLOSED}: it reads __webpack_module_cache__, a binding of the bundle's scope"
        )]
    );
}

#[test]
fn the_reference_lane_takes_a_bundle_without_the_proof() {
    for (shape, kind, _) in SHAPES {
        let found = outcome(&shape.replace("BODY", "greeting"), CONSERVATIVE);
        assert_eq!(found.kind, Some(kind), "{shape}");
    }
}

#[test]
fn a_closed_bundle_after_a_refused_one_is_the_one_taken() {
    let refused = WEBPACK_4.replace("BODY", "greeting");
    let closed = CHUNK.replace("BODY", "__webpack_require__(\"b\")");
    let source = format!("{refused}\n{closed}");
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let mut programs = vec![parsed.program];
    let (bundle, _) = unpack_inputs(&alloc, &mut programs, &[], UnpackOptions::default());
    let bundle = bundle.expect("the chunk");
    assert_eq!(
        bundle.modules.keys().collect::<Vec<_>>(),
        ["a", "b"],
        "{:?}",
        bundle.chunk_ids
    );
    assert_eq!(
        pretty_codegen(bundle.modules["b"].program()).trim_end(),
        "module.exports = \"world\";"
    );
}

#[test]
fn a_file_of_refused_bundles_is_checked_in_linear_time() {
    let shapes: String = (0..REFUSED_SHAPES)
        .map(|index| {
            format!(
                "(window.webpackJsonp = window.webpackJsonp || []).push([[{index}], {{ m{index}: function (module) {{ module.exports = greeting; }} }}]);\n"
            )
        })
        .collect();
    let source = format!("var greeting = \"hello\";\n{shapes}");
    let alloc = Allocator::default();
    let (parsed, parse) = cpu_or_wall(|| parse_program(&alloc, &source));
    let (_, scope) = cpu_or_wall(|| untwist_core::semantic::build_scoping(&parsed.program));
    let (bundle, spent) =
        cpu_or_wall(|| untwist_unpack::unpack(&alloc, &parsed.program, &[]).is_some());
    assert!(!bundle);
    let bound = (parse + scope) * LINEAR_FACTOR;
    assert!(
        spent < bound,
        "{REFUSED_SHAPES} refused bundles took {spent:?}, over {LINEAR_FACTOR} x parse {parse:?} + scoping {scope:?}"
    );
}

const UMD_WITH_EXTERNALS: &str = r#"
(function webpackUniversalModuleDefinition(root, factory) {
  if (typeof exports === "object" && typeof module === "object") module.exports = factory(require("react"));
  else if (typeof define === "function" && define.amd) define(["react"], factory);
  else if (typeof exports === "object") exports["lib"] = factory(require("react"));
  else root["lib"] = factory(root["React"]);
})(this, function (__WEBPACK_EXTERNAL_MODULE__1__) {
  return (function (modules) {
    var installedModules = {};
    function __webpack_require__(moduleId) {
      if (installedModules[moduleId]) return installedModules[moduleId].exports;
      var module = installedModules[moduleId] = { i: moduleId, l: false, exports: {} };
      modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);
      module.l = true;
      return module.exports;
    }
    return __webpack_require__(__webpack_require__.s = 0);
  })([
    function (module, exports, __webpack_require__) { module.exports = __webpack_require__(1).version; },
    function (module, exports) { module.exports = __WEBPACK_EXTERNAL_MODULE__1__; }
  ]);
});
"#;

#[test]
fn a_umd_wrapper_with_externals_is_refused() {
    let found = outcome(UMD_WITH_EXTERNALS, UnpackOptions::default());
    assert_eq!(found.kind, None);
    assert_eq!(
        found.notes,
        [format!(
            "module 1 {NOT_CLOSED}: it reads __WEBPACK_EXTERNAL_MODULE__1__, a binding of the bundle's scope"
        )]
    );
    assert!(found.unchanged);
}
