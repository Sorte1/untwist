mod common;

use common::prepared;
use oxc_allocator::Allocator;
use untwist_unpack::{unpack_program_with, UnpackOptions};

fn modules(source: &str, conservative: bool) -> Vec<(String, String)> {
    let alloc = Allocator::default();
    let program = prepared(&alloc, source);
    let options = UnpackOptions {
        conservative,
        ..UnpackOptions::default()
    };
    let mut bundle = unpack_program_with(&alloc, &program, options).expect("a bundle");
    untwist_unpack::finish_with(&alloc, &mut bundle, &[], options);
    bundle
        .modules
        .values()
        .map(|module| {
            let code = untwist_core::print::print(
                module.program(),
                untwist_core::print::PrintStyle::Pretty,
            );
            (
                module.path.as_str().to_string(),
                code.trim_end().to_string(),
            )
        })
        .collect()
}

const PRELUDE: &str = "(function (modules) {\n  var installed = {};\n  function r(id) {\n    if (installed[id]) return installed[id].exports;\n    var m = (installed[id] = { exports: {} });\n    modules[id].call(m.exports, m, m.exports, r);\n    return m.exports;\n  }\n  r.o = function (o, k) { return Object.prototype.hasOwnProperty.call(o, k); };\n  r.d = function (e, n, g) { if (!r.o(e, n)) Object.defineProperty(e, n, { enumerable: true, get: g }); };\n  r.r = function (e) { if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) { Object.defineProperty(e, Symbol.toStringTag, { value: \"Module\" }); } Object.defineProperty(e, \"__esModule\", { value: true }); };\n  r.t = function (v, m) { if (m & 1) v = r(v); if (m & 8) return v; var ns = Object.create(null); r.r(ns); Object.defineProperty(ns, \"default\", { enumerable: true, value: v }); return ns; };\n  return r(r.s = 0);\n})([\n";

fn bundle(entry: &str) -> String {
    format!("{PRELUDE}  function (e, t, n) {{ {entry} }},\n  function (e, t) {{ e.exports = {{ a: 1 }}; }},\n  function (e, t) {{ e.exports = require(\"path\"); }}\n]);\n")
}

#[test]
fn mode_nine_is_a_plain_require() {
    let out = modules(&bundle("console.log(n.t(1, 9).a);"), false);
    assert_eq!(out[0].1, "console.log(require(\"./1.js\").a);");
    assert_eq!(out.len(), 3);
}

#[test]
fn other_modes_call_webpacks_own_t_from_the_runtime_module() {
    let out = modules(
        &bundle("var ns = n.t(1, 3); Promise.resolve().then(n.t.bind(null, 1, 7));"),
        false,
    );
    let (path, runtime) = out.last().expect("a runtime module");
    assert_eq!(path, "_runtime/webpack.js");
    assert!(
        runtime.contains(
            "var r = function (id) {\n  return require(__webpack_module_paths__[id]);\n};"
        ),
        "{runtime}"
    );
    assert!(runtime.contains("r.t = function"), "{runtime}");
    assert!(!runtime.contains("r.d = "), "{runtime}");
    assert!(runtime.ends_with("module.exports = r;"), "{runtime}");
    assert!(
        out[0]
            .1
            .starts_with("const __webpack_require__ = require(\"./_runtime/webpack.js\");"),
        "{}",
        out[0].1
    );
    assert!(
        out[0].1.contains("__webpack_require__.t(1, 3)"),
        "{}",
        out[0].1
    );
    assert!(
        out[0].1.contains("__webpack_require__.t.bind(null, 1, 7)"),
        "{}",
        out[0].1
    );
}

#[test]
fn the_hosts_require_in_a_factory_without_a_runtime_parameter_is_kept() {
    let out = modules(&bundle("n(2);"), false);
    assert_eq!(out[2].1, "module.exports = require(\"path\");");
    let conservative = modules(&bundle("n(2);"), true);
    assert_eq!(
        conservative[2].1,
        "module.exports = require(\"./path.js\");"
    );
}

#[test]
fn a_bound_require_id_becomes_a_path() {
    let out = modules(&bundle("Promise.resolve().then(n.bind(null, 1));"), false);
    assert_eq!(
        out[0].1,
        "Promise.resolve().then(require.bind(null, \"./1.js\"));"
    );
}

#[test]
fn conservative_leaves_t_to_the_unhandled_note() {
    let out = modules(&bundle("var ns = n.t(1, 3);"), true);
    assert_eq!(out.len(), 3);
    assert!(out[0].1.contains("require.t(1, 3)"), "{}", out[0].1);
}
