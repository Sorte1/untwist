#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const PRELUDE: &str = "(function (modules) {\n  var installedModules = {};\n  function __webpack_require__(moduleId) {\n    if (installedModules[moduleId]) return installedModules[moduleId].exports;\n    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });\n    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);\n    module.l = true;\n    return module.exports;\n  }\n  __webpack_require__.d = function (exports, name, getter) {\n    if (!__webpack_require__.o(exports, name)) Object.defineProperty(exports, name, { enumerable: true, get: getter });\n  };\n  __webpack_require__.r = function (exports) {\n    Object.defineProperty(exports, \"__esModule\", { value: true });\n  };\n  __webpack_require__.n = function (module) {\n    var getter = module && module.__esModule ? function getDefault() { return module[\"default\"]; } : function getModuleExports() { return module; };\n    __webpack_require__.d(getter, \"a\", getter);\n    return getter;\n  };\n  __webpack_require__.o = function (object, property) {\n    return Object.prototype.hasOwnProperty.call(object, property);\n  };\n  return __webpack_require__((__webpack_require__.s = ENTRY));\n})";

const FLAGGED_COMMONJS: &str = "([\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var lib = __webpack_require__(1);\n    var lib_default = __webpack_require__.n(lib);\n    console.log(lib_default()(\"x\"), lib_default.a(\"y\"));\n  },\n  function (module, exports) {\n    Object.defineProperty(exports, \"__esModule\", { value: true });\n    exports.default = function (s) {\n      return \"hello \" + s;\n    };\n  },\n]);\n";

const PLAIN_COMMONJS: &str = "([\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var lib = __webpack_require__(1);\n    var lib_default = __webpack_require__.n(lib);\n    console.log(lib_default()(\"x\"), typeof lib_default.a);\n  },\n  function (module, exports) {\n    module.exports = function (s) {\n      return \"plain \" + s;\n    };\n  },\n]);\n";

const HARMONY: &str = "([\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var lib = __webpack_require__(1);\n    var lib_default = __webpack_require__.n(lib);\n    console.log(lib_default()(\"x\"), typeof lib_default.a);\n  },\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    __webpack_exports__[\"default\"] = function (s) {\n      return \"hello \" + s + \" \" + (this === undefined);\n    };\n  },\n]);\n";

const CYCLE: &str = "([\n  function (module, exports, __webpack_require__) {\n    var a = __webpack_require__(1);\n    console.log(a.show()());\n  },\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    __webpack_require__.d(__webpack_exports__, \"show\", function () {\n      return show;\n    });\n    var b = __webpack_require__(2);\n    var b_default = __webpack_require__.n(b);\n    __webpack_exports__[\"default\"] = \"a-default\";\n    function show() {\n      return b_default();\n    }\n  },\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var a = __webpack_require__(1);\n    var a_default = __webpack_require__.n(a);\n    __webpack_exports__[\"default\"] = function () {\n      return \"b sees \" + a_default.a;\n    };\n  },\n]);\n";

const STRING_IDS: &str = "({\n  \"./src/index.js\": function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var lib = __webpack_require__(\"./src/lib.js\");\n    var lib_default = __webpack_require__.n(lib);\n    console.log(lib_default()(\"x\"), typeof lib_default.a);\n  },\n  \"./src/lib.js\": function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    __webpack_exports__[\"default\"] = function (s) {\n      return \"hello \" + s;\n    };\n  },\n});\n";

const UNRESOLVED: &str = "([\n  function (module, exports, __webpack_require__) {\n    var pick = function (m) {\n      return __webpack_require__.n(m)();\n    };\n    console.log(pick(__webpack_require__(1)), pick({ plain: 1 }).plain);\n  },\n  function (module, __webpack_exports__, __webpack_require__) {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    __webpack_exports__[\"default\"] = \"the default\";\n  },\n]);\n";

fn unpacked(modules: &str, entry: &str, conservative: bool) -> Unpacked {
    let bundle = format!("{}{modules}", PRELUDE.replace("ENTRY", entry));
    Unpacked::of(&bundle, conservative)
}

#[test]
fn a_commonjs_target_that_flags_itself_keeps_the_interop_helper() {
    let unpacked = unpacked(FLAGGED_COMMONJS, "0", false);
    assert!(
        unpacked.entry().contains("__webpack_require__.n(lib)"),
        "{}",
        unpacked.entry()
    );
    unpacked.runs_like_the_bundle("hello x hello y\n");
}

#[test]
fn a_plain_commonjs_target_keeps_the_interop_helper() {
    let unpacked = unpacked(PLAIN_COMMONJS, "0", false);
    assert!(
        unpacked.entry().contains("__webpack_require__.n(lib)"),
        "{}",
        unpacked.entry()
    );
    unpacked.runs_like_the_bundle("plain x function\n");
}

#[test]
fn a_proven_harmony_target_is_read_through_its_default_export_at_each_use() {
    let unpacked = unpacked(HARMONY, "0", false);
    assert!(!unpacked.entry().contains(".n("), "{}", unpacked.entry());
    assert!(
        unpacked.entry().contains("(0, lib.default)(\"x\")"),
        "{}",
        unpacked.entry()
    );
    assert!(
        unpacked.entry().contains("typeof lib.default"),
        "{}",
        unpacked.entry()
    );
    unpacked.runs_like_the_bundle("hello x true function\n");
}

#[test]
fn a_circular_import_reads_the_default_export_when_it_is_used() {
    let unpacked = unpacked(CYCLE, "0", false);
    let dependent = unpacked.file("2.js");
    assert!(dependent.contains("a.default"), "{dependent}");
    assert!(!dependent.contains(".n("), "{dependent}");
    unpacked.runs_like_the_bundle("b sees a-default\n");
}

#[test]
fn string_module_ids_resolve_like_numeric_ones() {
    let unpacked = unpacked(STRING_IDS, "\"./src/index.js\"", false);
    assert!(!unpacked.entry().contains(".n("), "{}", unpacked.entry());
    assert!(
        unpacked.entry().contains("lib.default"),
        "{}",
        unpacked.entry()
    );
    unpacked.runs_like_the_bundle("hello x function\n");
}

#[test]
fn an_argument_that_is_not_a_known_import_keeps_the_interop_helper() {
    let unpacked = unpacked(UNRESOLVED, "0", false);
    assert!(
        unpacked.entry().contains("__webpack_require__.n(m)()"),
        "{}",
        unpacked.entry()
    );
    unpacked.runs_like_the_bundle("the default 1\n");
}
