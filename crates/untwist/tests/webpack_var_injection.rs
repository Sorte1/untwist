#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const PRELUDE: &str = "(function (modules) {\n  var installedModules = {};\n  function __webpack_require__(moduleId) {\n    if (installedModules[moduleId]) return installedModules[moduleId].exports;\n    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });\n    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);\n    module.l = true;\n    return module.exports;\n  }\n  return __webpack_require__((__webpack_require__.s = 0));\n})";

const BESIDE_OTHER_STATEMENTS: &str = "([\n  function (module, exports, __webpack_require__) {\n    var lib = __webpack_require__(1);\n    console.log(lib.outer, lib.inner, lib.after, lib.strict);\n  },\n  function (module, exports) {\n    var x = \"outer\";\n    (function (a, b) {\n      \"use strict\";\n      var x = \"inner\";\n      exports.inner = x + a + b;\n      exports.strict = (function () {\n        return this === undefined;\n      })();\n      if (a) return;\n      exports.never = 1;\n    }).call(this, 1, 2);\n    exports.outer = x;\n    exports.after = \"set\";\n  },\n]);\n";

const WHOLE_MODULE: &str = "([\n  function (module, exports, __webpack_require__) {\n    var lib = __webpack_require__(1);\n    console.log(lib.kind, lib.dir, lib.self);\n  },\n  function (module, exports, __webpack_require__) {\n    (function (global, __dirname) {\n      exports.kind = typeof global.setTimeout;\n      exports.dir = __dirname;\n      exports.self = this === exports;\n    }).call(this, __webpack_require__(2), \"/\");\n  },\n  function (module, exports) {\n    var g;\n    g = (function () {\n      return this;\n    })();\n    module.exports = g;\n  },\n]);\n";

const UNPROVEN: &str = "([\n  function (module, exports, __webpack_require__) {\n    var hoisted = __webpack_require__(1);\n    var extra = __webpack_require__(2);\n    var counted = __webpack_require__(3);\n    console.log(hoisted.kind, extra.a, counted.n);\n  },\n  function (module, exports) {\n    (function (a) {\n      function a() {}\n      exports.kind = typeof a;\n    }).call(this, 1);\n  },\n  function (module, exports) {\n    (function (a) {\n      exports.a = a;\n    }).call(this, 1, console.log(\"extra\"));\n  },\n  function (module, exports) {\n    (function (a) {\n      exports.n = arguments.length;\n    }).call(this, 5);\n  },\n]);\n";

fn unpacked(modules: &str) -> Unpacked {
    Unpacked::of(&format!("{PRELUDE}{modules}"), false)
}

#[test]
fn a_wrapper_beside_other_statements_keeps_its_own_scope() {
    let unpacked = unpacked(BESIDE_OTHER_STATEMENTS);
    let module = unpacked.file("lib.js");
    assert!(module.contains(".call(this, 1, 2)"), "{module}");
    unpacked.runs_like_the_bundle("outer inner12 set true\n");
}

#[test]
fn the_wrapper_around_a_whole_module_is_spliced() {
    let unpacked = unpacked(WHOLE_MODULE);
    let module = unpacked.file("lib.js");
    assert!(!module.contains(".call("), "{module}");
    assert!(module.contains("var __dirname = \"/\";"), "{module}");
    unpacked.runs_like_the_bundle("function / true\n");
}

#[test]
fn a_wrapper_the_splice_would_change_stays_a_call() {
    let unpacked = unpacked(UNPROVEN);
    for path in ["hoisted.js", "extra.js", "counted.js"] {
        let module = unpacked.file(path);
        assert!(module.contains(".call(this, "), "{path}: {module}");
    }
    unpacked.runs_like_the_bundle("extra\nfunction 1 1\n");
}
