#[path = "common/unpacked_tree.rs"]
mod unpacked_tree;

use unpacked_tree::Unpacked;

const NAMED: &str = "(function (modules) {\n  var installedModules = {};\n  function __webpack_require__(moduleId) {\n    if (installedModules[moduleId]) return installedModules[moduleId].exports;\n    var module = (installedModules[moduleId] = { i: moduleId, l: false, exports: {} });\n    modules[moduleId].call(module.exports, module, module.exports, __webpack_require__);\n    module.l = true;\n    return module.exports;\n  }\n  return __webpack_require__((__webpack_require__.s = 0));\n})([\n  function (module, exports, __webpack_require__) {\n    var lib = __webpack_require__(1);\n    console.log(lib.run());\n  },\n  function (module, exports) {\n    var table = {\n      \"./utils\": function () {\n        return \"utils\";\n      },\n      7: function () {\n        return \"seven\";\n      },\n    };\n    function load(require) {\n      return require(\"./utils\") + require(7);\n    }\n    exports.run = function () {\n      return load(function (name) {\n        return table[name]();\n      });\n    };\n  },\n]);\n";

const MINIFIED: &str = "(function (modules) {\n  var cache = {};\n  function r(id) {\n    if (cache[id]) return cache[id].exports;\n    var m = (cache[id] = { exports: {} });\n    modules[id].call(m.exports, m, m.exports, r);\n    return m.exports;\n  }\n  r.hmd = function (m) { return m; };\n  return r(0);\n})([\n  function (e, t, i) {\n    console.log(typeof require.hmd, typeof i.hmd);\n    function load(require) {\n      return require(\"./utils\") + require(1);\n    }\n    t.x = i(1);\n    console.log(load(function (name) { return \"<\" + name + \">\"; }), t.x);\n  },\n  function (e, t) {\n    e.exports = 7;\n  },\n]);\n";

#[test]
fn a_parameter_named_require_keeps_its_arguments() {
    let unpacked = Unpacked::of(NAMED, false);
    let module = unpacked.file("1.js");
    assert!(
        module.contains("require(\"./utils\") + require(7)"),
        "{module}"
    );
    unpacked.runs_like_the_bundle("utilsseven\n");
}

#[test]
fn a_parameter_named_require_keeps_its_arguments_in_the_reference_lane() {
    let unpacked = Unpacked::of(NAMED, true);
    let module = unpacked.file("1.js");
    assert!(
        module.contains("require(\"./utils\") + require(7)"),
        "{module}"
    );
    unpacked.runs_like_the_bundle("utilsseven\n");
}

#[test]
fn a_parameter_named_require_is_not_renamed_to_the_runtime() {
    let unpacked = Unpacked::of(MINIFIED, false);
    let entry = unpacked.entry();
    assert!(
        entry.contains("require(\"./utils\") + require(1)"),
        "{entry}"
    );
    unpacked.runs_like_the_bundle("undefined function\n<./utils><1> 7\n");
}
