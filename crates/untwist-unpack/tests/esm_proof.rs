mod common;

use common::prepared;
use oxc_allocator::Allocator;
use untwist_core::print::{print, PrintStyle};
use untwist_unpack::{finish_with, unpack_program_taking_with, unpack_program_with, UnpackOptions};

const HEAD: &str = "(function (modules) {\n  var installed = {};\n  function r(id) {\n    if (installed[id]) return installed[id].exports;\n    var m = (installed[id] = { exports: {} });\n    modules[id].call(m.exports, m, m.exports, r);\n    return m.exports;\n  }\n";
const HELPERS: &str = "  r.o = function (o, k) { return Object.prototype.hasOwnProperty.call(o, k); };\n  r.d = function (e, n, g) { if (!r.o(e, n)) Object.defineProperty(e, n, { enumerable: true, get: g }); };\n  r.r = function (e) { if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) { Object.defineProperty(e, Symbol.toStringTag, { value: \"Module\" }); } Object.defineProperty(e, \"__esModule\", { value: true }); };\n  r.n = function (m) { var g = m && m.__esModule ? function () { return m[\"default\"]; } : function () { return m; }; r.d(g, \"a\", g); return g; };\n  r.t = function (v, mode) { if (mode & 1) v = r(v); if (mode & 8) return v; if (mode & 4 && typeof v === \"object\" && v && v.__esModule) return v; var ns = Object.create(null); r.r(ns); Object.defineProperty(ns, \"default\", { enumerable: true, value: v }); if (mode & 2 && typeof v != \"string\") for (var key in v) r.d(ns, key, function (key) { return v[key]; }.bind(null, key)); return ns; };\n  r.e = function () { return Promise.resolve(); };\n";
const TAIL: &str = "  return r(r.s = 0);\n})([\n";
const TARGET: &str =
    "\"use strict\"; n.r(t); n.d(t, \"a\", function () { return a; }); function a() { return 1; }";

fn source(helpers: &str, factories: &[&str]) -> String {
    let list: Vec<String> = factories
        .iter()
        .map(|body| format!("  function (e, t, n) {{ {body} }}"))
        .collect();
    format!("{HEAD}{helpers}{TAIL}{}\n]);\n", list.join(",\n"))
}

fn printed_with(text: &str, options: UnpackOptions) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let program = prepared(&alloc, text);
    let mut bundle = unpack_program_with(&alloc, &program, options)?;
    finish_with(&alloc, &mut bundle, &[], options);
    let out = bundle.modules.values();
    Some(
        out.map(|module| {
            print(module.program(), PrintStyle::Pretty)
                .trim_end()
                .to_string()
        })
        .collect(),
    )
}

fn unpacked(factories: &[&str]) -> Vec<String> {
    printed_with(&source(HELPERS, factories), UnpackOptions::default()).expect("a bundle")
}

fn converted(code: &str) -> bool {
    code.contains("export {")
}

fn stays_commonjs(code: &str) {
    assert!(!converted(code), "{code}");
    assert!(
        code.contains("const __webpack_require__ = require(\"./_runtime/webpack.js\");"),
        "{code}"
    );
    assert!(code.contains("__webpack_require__.r(exports)"), "{code}");
}

fn consumed(consumer: &str) -> String {
    unpacked(&[consumer, TARGET]).swap_remove(1)
}

#[test]
fn sorted_binding_reads_become_live_export_specifiers() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.b);",
        "\"use strict\"; n.r(t); n.d(t, \"a\", function () { return a; }); n.d(t, \"b\", function () { return b; }); function a() { return 1; } var b = 2;",
    ]);
    assert!(
        out[1].ends_with("var b = 2;\nexport { a, b };"),
        "{}",
        out[1]
    );
    assert!(!out[1].contains("__webpack_require__"), "{}", out[1]);
}

#[test]
fn chained_export_names_keep_each_binding() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.b, m.c, m.Map);",
        "\"use strict\"; n.r(t); n.d(t, { Map: () => a, b: () => a, c: () => b }); var a = 1, b = 2; function g(b) { return a + b + new Map().size; }",
    ]);
    assert!(
        out[1].contains("export { a as Map, a as b, b as c };"),
        "{}",
        out[1]
    );
    assert!(
        out[1].contains("return a + b + new Map().size;"),
        "{}",
        out[1]
    );
    assert!(out[1].contains("var a = 1;\nvar b = 2;"), "{}", out[1]);
}

#[test]
fn a_default_of_a_variable_stays_a_live_binding() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.default);",
        "\"use strict\"; n.r(t); n.d(t, \"default\", function () { return x; }); let x = {}; x.y = 1; x = 2;",
    ]);
    assert!(out[1].contains("let x = {};"), "{}", out[1]);
    assert!(out[1].ends_with("export { x as default };"), "{}", out[1]);
}

#[test]
fn reserved_words_and_repeated_locals_are_valid_export_names() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.class, m.k);",
        "\"use strict\"; n.r(t); n.d(t, { class: () => a, k: () => a }); function a() {}",
    ]);
    assert!(
        out[1].contains("export { a as class, a as k };"),
        "{}",
        out[1]
    );
}

#[test]
fn a_this_inside_a_class_field_does_not_refuse() {
    let out = consumed("var m = n(1); console.log(m.a);");
    assert!(converted(&out), "{out}");
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "\"use strict\"; n.r(t); n.d(t, \"a\", function () { return a; }); class a { x = this; }",
    ]);
    assert!(converted(&out[1]), "{}", out[1]);
}

#[test]
fn a_converted_module_keeps_its_commonjs_imports_as_require_calls() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "\"use strict\"; n.r(t); var d = n(2); n.d(t, \"a\", function () { return a; }); var a = d.x;",
        "e.exports = { x: 1 };",
    ]);
    assert!(converted(&out[1]), "{}", out[1]);
    assert!(out[1].contains("require(\"./2.js\")"), "{}", out[1]);
    assert!(!out[1].contains("import * as"), "{}", out[1]);
}

#[test]
fn a_loading_require_t_with_mode_8_is_a_plain_require() {
    let out = consumed("var b = n.t(1, 9); console.log(b.a);");
    assert!(converted(&out), "{out}");
}

#[test]
fn refuses_a_sloppy_module() {
    let out = consumed_target("n.r(t); n.d(t, \"a\", function () { return a; }); function a() {}");
    stays_commonjs(&out);
}

fn consumed_target(target: &str) -> String {
    unpacked(&["var m = n(1); console.log(m.a);", target]).swap_remove(1)
}

fn refused_target(rest: &str) {
    let out = consumed_target(&format!(
        "\"use strict\"; n.r(t); n.d(t, \"a\", function () {{ return a; }}); {rest}"
    ));
    stays_commonjs(&out);
}

fn unbundled_target(rest: &str) {
    let target =
        format!("\"use strict\"; n.r(t); n.d(t, \"a\", function () {{ return a; }}); {rest}");
    let text = source(HELPERS, &["var m = n(1); console.log(m.a);", &target]);
    assert_eq!(
        printed_with(&text, UnpackOptions::default()),
        None,
        "{text}"
    );
}

#[test]
fn refuses_define_on_a_namespace_object() {
    refused_target("function a() {} var ns = {}; n.d(ns, \"z\", function () { return a; });");
}

#[test]
fn refuses_an_exports_write_left_after_conversion() {
    refused_target("function a() {} t[\"default\"] = 2;");
}

#[test]
fn refuses_a_module_reference() {
    refused_target("function a() {} console.log(e.id);");
}

#[test]
fn refuses_top_level_this() {
    refused_target("function a() {} console.log(this);");
}

#[test]
fn top_level_arguments_refuses_the_bundle() {
    unbundled_target("function a() {} console.log(arguments.length);");
}

#[test]
fn refuses_this_in_a_class_heritage() {
    refused_target("function a() {} class K extends (this ? Object : Object) {}");
}

#[test]
fn refuses_a_top_level_return() {
    refused_target("function a() {} if (!a) return;");
}

#[test]
fn refuses_new_target() {
    refused_target("function a() {} console.log(new.target);");
}

#[test]
fn refuses_await_as_an_identifier() {
    refused_target("function a() {} var await = 1;");
}

#[test]
fn direct_eval_refuses_the_bundle() {
    unbundled_target("function a() {} eval(\"1\");");
}

#[test]
fn refuses_a_getter_that_is_not_a_binding_read() {
    let out = consumed_target(
        "\"use strict\"; n.r(t); var obj = { v: 1 }; n.d(t, \"v\", function () { return obj.v; });",
    );
    stays_commonjs(&out);
}

#[test]
fn refuses_a_getter_of_a_binding_outside_the_top_level() {
    let out = consumed_target(
        "\"use strict\"; n.r(t); n.d(t, \"a\", function a() { return a; }); { let a = 1; }",
    );
    stays_commonjs(&out);
}

#[test]
fn refuses_exports_out_of_key_order() {
    let out = consumed_target(
        "\"use strict\"; n.r(t); n.d(t, \"b\", function () { return b; }); n.d(t, \"a\", function () { return a; }); function a() {} function b() {}",
    );
    stays_commonjs(&out);
}

#[test]
fn refuses_a_non_ascii_export_name() {
    let out = consumed_target(
        "\"use strict\"; n.r(t); n.d(t, \"\u{e9}\", function () { return a; }); function a() {}",
    );
    stays_commonjs(&out);
}

#[test]
fn refuses_a_module_in_a_require_cycle() {
    let out = consumed_target(
        "\"use strict\"; n.r(t); var o = n(0); n.d(t, \"a\", function () { return a; }); function a() { return o; }",
    );
    stays_commonjs(&out);
}

#[test]
fn refuses_a_target_read_through_default_interop() {
    stays_commonjs(&consumed("var b = n(1); console.log(String(n.n(b)));"));
}

#[test]
fn refuses_a_target_read_through_require_t() {
    stays_commonjs(&consumed("var b = n.t(1, 7); console.log(b.a);"));
}

#[test]
fn refuses_a_target_read_through_a_bound_require_t() {
    stays_commonjs(&consumed(
        "Promise.resolve().then(n.t.bind(null, 1, 7)).then(function (m) { console.log(m.a); });",
    ));
}

#[test]
fn refuses_a_target_loaded_by_a_bound_require() {
    stays_commonjs(&consumed(
        "Promise.resolve().then(n.bind(null, 1)).then(function (m) { console.log(m.a); });",
    ));
}

#[test]
fn refuses_a_require_t_mode_beyond_a_small_integer() {
    stays_commonjs(&consumed("var b = n.t(1, 4294967304); console.log(b.a);"));
}

#[test]
fn refuses_a_reassigned_import_binding() {
    stays_commonjs(&consumed(
        "var b = n(1); b = { a: 2 }; console.log(String(n.n(b)), b.a);",
    ));
}

#[test]
fn refuses_a_property_write_through_an_alias() {
    stays_commonjs(&consumed("var m = n(1); var q = m; q.a = 2;"));
}

#[test]
fn refuses_object_assign_on_a_required_module() {
    stays_commonjs(&consumed("Object.assign(n(1), { z: 1 });"));
}

#[test]
fn refuses_a_redeclared_import_binding() {
    stays_commonjs(&consumed(
        "var m = n(1); var m = { a: 2 }; console.log(m.a);",
    ));
}

#[test]
fn refuses_a_delete_through_a_required_module() {
    stays_commonjs(&consumed("var m = n(1); delete m.a;"));
}

#[test]
fn refuses_a_tagged_template_on_a_required_module() {
    stays_commonjs(&consumed("var m = n(1); m.a`x`;"));
}

#[test]
fn refuses_a_method_call_on_a_required_module() {
    stays_commonjs(&consumed("var m = n(1); m.a();"));
}

#[test]
fn refuses_an_inherited_property_read() {
    stays_commonjs(&consumed("var m = n(1); console.log(m.constructor);"));
}

#[test]
fn refuses_the_bundle_on_an_unattributable_interop() {
    stays_commonjs(&consumed(
        "var b = n(1); function f(b) { return String(n.n(b)); } console.log(f(2), b.a);",
    ));
}

#[test]
fn refuses_the_bundle_on_a_computed_runtime_member() {
    let text = source(
        HELPERS,
        &[
            "var k = \"t\"; var h = n[k]; console.log(h(1, 7).a);",
            TARGET,
        ],
    );
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn refuses_the_bundle_on_an_unknown_runtime_member() {
    stays_commonjs(&consumed("var h = n.t; console.log(h(1, 7).a);"));
}

#[test]
fn refuses_the_bundle_on_a_non_literal_require() {
    stays_commonjs(&consumed("var k = 1; var b = n(k); console.log(b.a);"));
}

#[test]
fn refuses_the_bundle_on_an_escaping_require() {
    let text = source(HELPERS, &["var q = n; console.log(q(1).a);", TARGET]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn refuses_the_bundle_on_an_es_module_flag_read() {
    stays_commonjs(&consumed("var other = {}; console.log(other.__esModule);"));
}

#[test]
fn a_shadowing_require_parameter_is_not_routed() {
    let out = unpacked(&[
        "\"use strict\"; n.r(t); var ns = {}; n.r(ns); function f(require) { return require.r; } console.log(f);",
    ]);
    assert!(out[0].contains("return require.r;"), "{}", out[0]);
    assert!(out[0].contains("__webpack_require__.r(ns)"), "{}", out[0]);
}

#[test]
fn a_prelude_less_chunk_with_a_harmony_module_is_refused_and_left_intact() {
    let text = "(window.webpackJsonp = window.webpackJsonp || []).push([[1], { a: function (e, t, n) { \"use strict\"; n.r(t); var b = 1; } }]);\n";
    assert!(printed_with(text, UnpackOptions::default()).is_none());
    let conservative = UnpackOptions {
        conservative: true,
        ..UnpackOptions::default()
    };
    assert!(printed_with(text, conservative).is_some());
    let alloc = Allocator::default();
    let mut program = prepared(&alloc, text);
    let before = print(&program, PrintStyle::Pretty);
    assert!(unpack_program_taking_with(&alloc, &mut program, UnpackOptions::default()).is_none());
    assert_eq!(print(&program, PrintStyle::Pretty), before);
}

#[test]
fn a_harmony_module_whose_helper_is_missing_refuses_the_bundle() {
    let helpers =
        "  r.r = function (e) { if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) { Object.defineProperty(e, Symbol.toStringTag, { value: \"Module\" }); } Object.defineProperty(e, \"__esModule\", { value: true }); };\n";
    let text = source(helpers, &["var m = n(1); console.log(m.a);", TARGET]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

fn refused_by_prelude(extra: &str) {
    let helpers = format!("{extra}{HELPERS}  r.d = function (e, n, g) {{ own; Object.defineProperty(e, n, {{ enumerable: true, get: g }}); }};\n");
    let text = source(&helpers, &["var m = n(1); console.log(m.a);", TARGET]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn a_helper_reading_a_destructured_prelude_name_is_not_movable() {
    refused_by_prelude("  var { own } = { own: 1 };\n");
}

#[test]
fn a_helper_reading_a_prelude_class_is_not_movable() {
    refused_by_prelude("  class own {}\n");
}

#[test]
fn a_helper_reading_a_nested_prelude_var_is_not_movable() {
    refused_by_prelude("  if (modules) { var own = 1; }\n");
}

#[test]
fn webpack_5_arrow_helpers_route_a_refused_module() {
    let text = "(() => {\n  var __webpack_modules__ = {\n    1: (module, exports, require) => { \"use strict\"; require.r(exports); var ns = {}; require.d(ns, { z: () => z }); function z() {} console.log(ns.z); },\n  };\n  var cache = {};\n  function __webpack_require__(id) {\n    if (cache[id]) return cache[id].exports;\n    var module = (cache[id] = { exports: {} });\n    __webpack_modules__[id](module, module.exports, __webpack_require__);\n    return module.exports;\n  }\n  (() => {\n    __webpack_require__.d = (exports, definition) => { for (var key in definition) { if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, { enumerable: true, get: definition[key] }); } };\n  })();\n  (() => {\n    __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);\n  })();\n  (() => {\n    __webpack_require__.r = (exports) => { if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) { Object.defineProperty(exports, Symbol.toStringTag, { value: \"Module\" }); } Object.defineProperty(exports, \"__esModule\", { value: true }); };\n  })();\n  __webpack_require__(1);\n})();\n";
    let out = printed_with(text, UnpackOptions::default()).expect("a bundle");
    stays_commonjs(&out[0]);
    assert!(
        out[0].contains("__webpack_require__.d(ns, { z: () => z })"),
        "{}",
        out[0]
    );
    let runtime = out.last().expect("the runtime module");
    assert!(
        runtime.contains("__webpack_require__.o = (obj, prop) =>"),
        "{runtime}"
    );
}

#[test]
fn conservative_keeps_the_references_conversion() {
    let text = source(
        HELPERS,
        &["\"use strict\"; n.r(t); var obj = { v: 1 }; n.d(t, \"v\", function () { return obj.v; });"],
    );
    let conservative = UnpackOptions {
        conservative: true,
        ..UnpackOptions::default()
    };
    let out = printed_with(&text, conservative).expect("a bundle");
    assert!(out[0].contains("export let v = obj.v;"), "{}", out[0]);
}

const SECOND: &str =
    "\"use strict\"; n.r(t); n.d(t, \"b\", function () { return b; }); function b() { return 2; }";

fn beside(consumer: &str) -> Vec<String> {
    unpacked(&[consumer, TARGET, SECOND])
}

#[test]
fn a_return_inside_an_arrow_is_not_top_level() {
    refused_target_not("function a() {} const f = () => { return 1; }; f();");
}

fn refused_target_not(rest: &str) {
    let out = consumed_target(&format!(
        "\"use strict\"; n.r(t); n.d(t, \"a\", function () {{ return a; }}); {rest}"
    ));
    assert!(converted(&out), "{out}");
}

#[test]
fn a_dynamic_import_through_a_bound_require_refuses_only_its_target() {
    let out = beside(
        "var m = n(2); console.log(m.b); Promise.resolve().then(n.bind(n, 1)).then(function (x) { console.log(x.a); });",
    );
    stays_commonjs(&out[1]);
    assert!(converted(&out[2]), "{}", out[2]);
    assert!(
        out[0].contains("require.bind(require, \"./1.js\")"),
        "{}",
        out[0]
    );
}

#[test]
fn a_require_t_bound_to_another_this_refuses_the_bundle() {
    let out = beside(
        "var m = n(2); console.log(m.b); var other = {}; Promise.resolve().then(n.t.bind(other, 1, 7));",
    );
    stays_commonjs(&out[2]);
}

#[test]
fn a_require_t_bound_to_the_runtime_keeps_its_this() {
    let out = beside("var m = n(2); console.log(m.b); Promise.resolve().then(n.t.bind(n, 1, 7));");
    assert!(
        out[0].contains("__webpack_require__.t.bind(__webpack_require__, 1, 7)"),
        "{}",
        out[0]
    );
    assert!(converted(&out[2]), "{}", out[2]);
}

#[test]
fn an_identifier_key_es_module_refuses_the_bundle() {
    stays_commonjs(&consumed(
        "var o = { __esModule: true }; var m = n(1); console.log(o, m.a);",
    ));
}

#[test]
fn a_parenthesized_method_call_refuses_its_target() {
    stays_commonjs(&consumed("var m = n(1); (m.a)();"));
}

#[test]
fn a_var_named_require_stops_routing() {
    let out = unpacked(&[
        "\"use strict\"; n.r(t); var ns = {}; n.r(ns); function f() { var require = { r: function () {} }; require.r(1); { let require = 2; } } f();",
    ]);
    assert!(out[0].contains("require.r(1);"), "{}", out[0]);
    assert!(out[0].contains("__webpack_require__.r(ns)"), "{}", out[0]);
}

#[test]
fn a_block_scoped_require_shadows_only_its_block() {
    let out = unpacked(&[
        "\"use strict\"; n.r(t); var ns = {}; function f() { { let require = { r: function () {} }; require.r(1); } n.r(ns); } f();",
    ]);
    assert!(out[0].contains("require.r(1);"), "{}", out[0]);
    assert!(out[0].contains("__webpack_require__.r(ns)"), "{}", out[0]);
}

#[test]
fn a_non_harmony_chunk_using_a_runtime_member_is_refused() {
    let text = "(window.webpackJsonp = window.webpackJsonp || []).push([[1], { a: function (e, t, n) { Promise.resolve().then(n.t.bind(null, \"b\", 7)); }, b: function (e) { e.exports = 1; } }]);\n";
    assert!(printed_with(text, UnpackOptions::default()).is_none());
}

#[test]
fn a_member_the_runtime_cannot_move_refuses_the_bundle() {
    let helpers = format!("{HELPERS}  r.g = installed;\n");
    let text = source(&helpers, &["console.log(n.g);"]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn a_member_the_prelude_never_defines_refuses_the_bundle() {
    let text = source(HELPERS, &["console.log(n.g);"]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn a_literal_runtime_member_is_routed() {
    let helpers = format!("{HELPERS}  r.p = \"/assets/\";\n");
    let out = printed_with(
        &source(&helpers, &["console.log(n.p);"]),
        UnpackOptions::default(),
    )
    .expect("a bundle");
    assert!(
        out[0].contains("console.log(__webpack_require__.p)"),
        "{}",
        out[0]
    );
    let runtime = out.last().expect("the runtime module");
    assert!(runtime.contains("r.p = \"/assets/\""), "{runtime}");
}

const WEBPACK5_HEAD: &str = "(() => {\n  var __webpack_modules__ = {\n    1: (module, exports, require) => { \"use strict\"; require.r(exports); var ns = {}; require.d(ns, { z: () => z }); function z() {} console.log(ns.z, require.t({ q: 1 }, 2).q); },\n  };\n  var cache = {};\n  function __webpack_require__(id) {\n    if (cache[id]) return cache[id].exports;\n    var module = (cache[id] = { exports: {} });\n    __webpack_modules__[id](module, module.exports, __webpack_require__);\n    return module.exports;\n  }\n";

#[test]
fn an_iife_local_require_t_moves_with_its_locals() {
    let text = format!("{WEBPACK5_HEAD}  (() => {{\n    var getProto = Object.getPrototypeOf ? (obj) => Object.getPrototypeOf(obj) : (obj) => obj.__proto__;\n    var leafPrototypes;\n    __webpack_require__.t = function (value, mode) {{ if (mode & 1) value = this(value); if (mode & 8) return value; var ns = Object.create(null); __webpack_require__.r(ns); var def = {{}}; leafPrototypes = leafPrototypes || [null, getProto({{}}), getProto([]), getProto(getProto)]; for (var current = mode & 2 && value; typeof current == \"object\" && !~leafPrototypes.indexOf(current); current = getProto(current)) {{ Object.getOwnPropertyNames(current).forEach((key) => (def[key] = () => value[key])); }} def[\"default\"] = () => value; __webpack_require__.d(ns, def); return ns; }};\n  }})();\n  (() => {{\n    __webpack_require__.d = (exports, definition) => {{ for (var key in definition) {{ if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, {{ enumerable: true, get: definition[key] }}); }} }};\n  }})();\n  (() => {{\n    __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);\n  }})();\n  (() => {{\n    __webpack_require__.r = (exports) => {{ if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) {{ Object.defineProperty(exports, Symbol.toStringTag, {{ value: \"Module\" }}); }} Object.defineProperty(exports, \"__esModule\", {{ value: true }}); }};\n  }})();\n  __webpack_require__(1);\n}})();\n");
    let out = printed_with(&text, UnpackOptions::default()).expect("a bundle");
    assert!(
        out[0].contains("__webpack_require__.t({ q: 1 }, 2)"),
        "{}",
        out[0]
    );
    let runtime = out.last().expect("the runtime module");
    assert!(runtime.contains("var getProto"), "{runtime}");
    assert!(runtime.contains("var leafPrototypes"), "{runtime}");
}

#[test]
fn an_iife_local_require_t_reading_an_outer_name_is_not_movable() {
    let text = format!("{WEBPACK5_HEAD}  var outer = 1;\n  (() => {{\n    var local = outer;\n    __webpack_require__.t = function (value) {{ return local; }};\n  }})();\n  (() => {{\n    __webpack_require__.d = (exports, definition) => {{ for (var key in definition) {{ if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, {{ enumerable: true, get: definition[key] }}); }} }};\n  }})();\n  (() => {{\n    __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);\n  }})();\n  (() => {{\n    __webpack_require__.r = (exports) => {{ if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) {{ Object.defineProperty(exports, Symbol.toStringTag, {{ value: \"Module\" }}); }} Object.defineProperty(exports, \"__esModule\", {{ value: true }}); }};\n  }})();\n  __webpack_require__(1);\n}})();\n");
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn a_parenthesized_method_call_refuses_its_target_without_prepare() {
    let text = source(HELPERS, &["var m = n(1); (m.a)();", TARGET]);
    let alloc = Allocator::default();
    let program = common::parse(&alloc, &text);
    let options = UnpackOptions::default();
    let mut bundle = unpack_program_with(&alloc, &program, options).expect("a bundle");
    finish_with(&alloc, &mut bundle, &[], options);
    let target = bundle.modules.values().nth(1).expect("the target");
    stays_commonjs(print(target.program(), PrintStyle::Pretty).trim_end());
}

#[test]
fn a_member_behind_a_comment_is_still_gated() {
    let helpers = format!("{HELPERS}  r.g = installed;\n");
    let text = source(&helpers, &["console.log(n /* x */ .g);"]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn an_optional_member_is_still_gated() {
    let helpers = format!("{HELPERS}  r.g = installed;\n");
    let text = source(&helpers, &["console.log(n?.g);"]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

#[test]
fn a_quoted_define_key_is_an_export_name() {
    let out = unpacked_raw(&[
        "var m = n(1); console.log(m.default, m.z);",
        "\"use strict\"; n.r(t); n.d(t, { \"default\": () => a, z: () => a }); function a() {}",
    ]);
    assert!(
        out[1].contains("export { a as default, a as z };"),
        "{}",
        out[1]
    );
}

fn unpacked_raw(factories: &[&str]) -> Vec<String> {
    let text = source(HELPERS, factories);
    let alloc = Allocator::default();
    let program = common::parse(&alloc, &text);
    let options = UnpackOptions::default();
    let mut bundle = unpack_program_with(&alloc, &program, options).expect("a bundle");
    finish_with(&alloc, &mut bundle, &[], options);
    let out = bundle.modules.values();
    out.map(|module| {
        print(module.program(), PrintStyle::Pretty)
            .trim_end()
            .to_string()
    })
    .collect()
}

#[test]
fn an_iife_local_require_t_moves_with_its_locals_when_taken() {
    let text = format!("{WEBPACK5_HEAD}  (() => {{\n    var getProto = (obj) => Object.getPrototypeOf(obj);\n    __webpack_require__.t = function (value, mode) {{ var ns = Object.create(null); __webpack_require__.r(ns); __webpack_require__.d(ns, {{ default: () => getProto(value) }}); return ns; }};\n  }})();\n  (() => {{\n    __webpack_require__.d = (exports, definition) => {{ for (var key in definition) {{ if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, {{ enumerable: true, get: definition[key] }}); }} }};\n  }})();\n  (() => {{\n    __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop);\n  }})();\n  (() => {{\n    __webpack_require__.r = (exports) => {{ if (typeof Symbol !== \"undefined\" && Symbol.toStringTag) {{ Object.defineProperty(exports, Symbol.toStringTag, {{ value: \"Module\" }}); }} Object.defineProperty(exports, \"__esModule\", {{ value: true }}); }};\n  }})();\n  __webpack_require__(1);\n}})();\n");
    let alloc = Allocator::default();
    let mut program = prepared(&alloc, &text);
    let options = UnpackOptions::default();
    let mut bundle = unpack_program_taking_with(&alloc, &mut program, options).expect("a bundle");
    finish_with(&alloc, &mut bundle, &[], options);
    let runtime = bundle.modules.values().last().expect("the runtime module");
    let runtime = print(runtime.program(), PrintStyle::Pretty);
    assert!(runtime.contains("var getProto"), "{runtime}");
    assert!(
        runtime.contains("__webpack_require__.t = function"),
        "{runtime}"
    );
}

fn gated(extra: &str, consumer: &str) -> bool {
    let helpers = format!("{HELPERS}{extra}");
    printed_with(&source(&helpers, &[consumer]), UnpackOptions::default()).is_none()
}

#[test]
fn a_host_require_property_is_never_passed_through() {
    assert!(gated("", "console.log(n.resolve);"));
}

#[test]
fn a_member_assigned_inside_a_prelude_branch_is_gated() {
    assert!(gated(
        "  if (installed) { r.ab = \"/x/\"; }\n",
        "console.log(n.ab);"
    ));
}

#[test]
fn a_helper_reading_the_host_dirname_is_not_movable() {
    assert!(gated(
        "  r.ab = function () { return __dirname; };\n",
        "console.log(n.ab());"
    ));
}

#[test]
fn a_helper_with_an_effectful_value_is_not_movable() {
    assert!(gated("  r.x = Math.random();\n", "console.log(n.x);"));
}

const BARE_TARGET: &str =
    "\"use strict\"; n.d(t, { a: () => a, b: () => b }); function a() { return 1; } var b = 2;";

fn bare(consumer: &str) -> String {
    unpacked(&[consumer, BARE_TARGET]).swap_remove(1)
}

fn bare_stays_commonjs(code: &str) {
    assert!(!converted(code), "{code}");
    assert!(code.contains("__webpack_require__.d(exports,"), "{code}");
}

#[test]
fn a_strict_define_only_module_is_proven_without_the_harmony_flag() {
    let out = bare("var m = n(1); console.log((0, m.a)(), m.b);");
    assert!(out.ends_with("var b = 2;\nexport { a, b };"), "{out}");
    assert!(!out.contains("__webpack_require__"), "{out}");
}

#[test]
fn a_sloppy_define_only_module_stays_commonjs() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "n.d(t, { a: () => a }); function a() { return 1; }",
    ]);
    assert!(!converted(&out[1]), "{}", out[1]);
}

#[test]
fn a_define_only_module_used_as_a_value_stays_commonjs() {
    bare_stays_commonjs(&bare("var m = n(1); console.log(Object.keys(m));"));
}

#[test]
fn a_define_only_module_with_an_exports_write_stays_commonjs() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "\"use strict\"; n.d(t, { a: () => a }); function a() { return 1; } t.c = 3;",
    ]);
    assert!(!converted(&out[1]), "{}", out[1]);
}

const IMPORTED: &str = "var m = n(2); console.log(m.b);";

fn imported(tail: &str) -> Vec<String> {
    beside(&format!("{IMPORTED} {tail}"))
}

#[test]
fn a_dynamic_import_read_by_member_converts_its_target() {
    let out = imported("Promise.resolve().then(n.bind(n, 1)).then((x) => console.log(x.a));");
    assert!(converted(&out[1]), "{}", out[1]);
    assert!(
        out[0].contains("require.bind(require, \"./1.js\")"),
        "{}",
        out[0]
    );
}

#[test]
fn a_chunk_load_then_bound_require_converts_its_target() {
    let out = imported("n.e(3).then(n.bind(null, 1)).then((x) => console.log(x.a));");
    assert!(converted(&out[1]), "{}", out[1]);
    let all = imported(
        "Promise.all([n.e(3), n.e(4)]).then(n.bind(n, 1)).then((x) => console.log(x.a), (e) => e);",
    );
    assert!(converted(&all[1]), "{}", all[1]);
}

#[test]
fn a_dynamic_import_whose_value_escapes_refuses_its_target() {
    for tail in [
        "Promise.resolve().then(n.bind(n, 1)).then((x) => console.log(x));",
        "Promise.resolve().then(n.bind(n, 1)).then((x) => x);",
        "Promise.resolve().then(n.bind(n, 1)).then((x) => { x = 1; });",
        "Promise.resolve().then(n.bind(n, 1)).then((x) => x.hasOwnProperty(\"a\"));",
        "Promise.resolve().then(n.bind(n, 1)).then((x, y) => console.log(x.a));",
        "Promise.resolve().then(n.bind(n, 1)).then(({ a }) => console.log(a));",
        "Promise.resolve().then(n.bind(n, 1)).catch((x) => console.log(x.a));",
        "var p = Promise.resolve().then(n.bind(n, 1)); p.then((x) => console.log(x.a));",
        "Promise.resolve().then(n.bind(n, 1), 0).then((x) => console.log(x.a));",
        "Promise.resolve(1).then(n.bind(n, 1)).then((x) => console.log(x.a));",
        "thenable.then(n.bind(n, 1)).then((x) => console.log(x.a));",
        "Q.resolve().then(n.bind(n, 1)).then((x) => console.log(x.a));",
        "n.e(other).then(n.bind(n, 1)).then((x) => console.log(x.a));",
        "Promise.all([n.e(3), more]).then(n.bind(n, 1)).then((x) => console.log(x.a));",
    ] {
        let out = imported(tail);
        stays_commonjs(&out[1]);
        assert!(converted(&out[2]), "{tail}\n{}", out[2]);
    }
}

#[test]
fn a_shadowed_promise_refuses_the_dynamic_import_target() {
    let out = imported(
        "var Promise = { resolve: () => thenable }; Promise.resolve().then(n.bind(n, 1)).then((x) => console.log(x.a));",
    );
    stays_commonjs(&out[1]);
}

fn helpers_without_t() -> String {
    HELPERS
        .split_inclusive('\n')
        .filter(|line| !line.trim_start().starts_with("r.t ="))
        .collect()
}

fn runtime_of(extra: &str, consumer: &str) -> Option<String> {
    let helpers = format!("{}{extra}", helpers_without_t());
    let out = printed_with(&source(&helpers, &[consumer]), UnpackOptions::default())?;
    out.last().cloned()
}

#[test]
fn a_guarded_prelude_assignment_moves_when_the_guard_is_the_runtime() {
    for guard in [
        "if (r) r.ab = \"/x/\";",
        "if (r !== undefined) { r.ab = \"/x/\"; }",
        "if (void 0 != r) r.ab = \"/x/\";",
        "if (r != null) r.ab = \"/x/\";",
        "if (typeof r !== \"undefined\") r.ab = \"/x/\";",
        "if (\"undefined\" != typeof r) { r.ab = \"/x/\"; }",
    ] {
        let runtime = runtime_of(&format!("  {guard}\n"), "console.log(n.ab);")
            .unwrap_or_else(|| panic!("{guard}: no bundle"));
        assert!(runtime.contains("r.ab = \"/x/\";"), "{guard}\n{runtime}");
    }
}

#[test]
fn an_unproven_guard_is_gated() {
    for guard in [
        "if (installed) r.ab = \"/x/\";",
        "if (r) r.ab = \"/x/\"; else r.ab = \"/y/\";",
        "if (r) { r.ab = \"/x/\"; r.cd = 1; }",
        "if (typeof r === \"undefined\") r.ab = \"/x/\";",
        "if (r != 0) r.ab = \"/x/\";",
        "if (r !== null) r.ab = \"/x/\";",
        "if (typeof r !== \"function\") r.ab = \"/x/\";",
        "if (r) r.ab = installed;",
        "r = function () {}; if (r) r.ab = \"/x/\";",
        "var undefined = 1; if (r !== undefined) r.ab = \"/x/\";",
    ] {
        assert!(
            gated(&format!("  {guard}\n"), "console.log(n.ab);"),
            "{guard}"
        );
    }
}

#[test]
fn a_runtime_written_inside_a_prelude_function_is_gated() {
    assert!(gated(
        "  function reset() { r = null; }\n  if (r) r.ab = \"/x/\";\n",
        "console.log(n.ab);"
    ));
}

#[test]
fn the_bundle_directory_maps_to_the_output_root() {
    let runtime = runtime_of(
        "  if (typeof r !== \"undefined\") r.ab = __dirname + \"/native_modules/\";\n",
        "console.log(n.ab);",
    )
    .expect("a bundle");
    assert!(
        runtime.contains("r.ab = require(\"path\").dirname(__dirname) + \"/native_modules/\";"),
        "{runtime}"
    );
}

#[test]
fn other_host_paths_are_gated() {
    for extra in [
        "  r.ab = __dirname + suffix;\n",
        "  r.ab = __filename + \"/x/\";\n",
        "  r.ab = \"/x/\" + __dirname;\n",
        "  r.ab = __dirname - \"/x/\";\n",
        "  var __dirname = \"/\"; r.ab = __dirname + \"/x/\";\n",
        "  var require = 1; r.ab = __dirname + \"/x/\";\n",
    ] {
        assert!(gated(extra, "console.log(n.ab);"), "{extra}");
    }
}

const GET_PROTO: &str =
    "t = Object.getPrototypeOf ? (o) => Object.getPrototypeOf(o) : (o) => o.__proto__";
const LOCAL_T: &str =
    "r.t = function (v, m) { e = e || [null, t({}), t([])]; return e.indexOf(t(v)) + m; }";

fn local_t(prelude: &str) -> Option<String> {
    runtime_of(prelude, "console.log(n.t({}, 2));")
}

#[test]
fn a_helper_reading_bare_prelude_vars_moves_with_them() {
    let runtime = local_t(&format!(
        "  var e, t;\n  {GET_PROTO}, {LOCAL_T}, r.x = 1;\n"
    ))
    .expect("a bundle");
    assert!(runtime.contains("var e, t;"), "{runtime}");
    assert!(runtime.contains("t = Object.getPrototypeOf ?"), "{runtime}");
    assert!(runtime.contains("r.t = function (v, m)"), "{runtime}");
    let runtime = local_t("  var e;\n  r.t = function (v) { e = e || [v]; return e[0]; };\n")
        .expect("a bundle without initializer");
    assert!(runtime.contains("var e;"), "{runtime}");
    let runtime = local_t(&format!(
        "  var e;\n  var t;\n  {GET_PROTO};\n  r.x = 1;\n  {LOCAL_T};\n"
    ))
    .expect("a bundle with split statements");
    assert!(runtime.contains("var e, t;"), "{runtime}");
}

#[test]
fn a_shared_or_unsafe_prelude_var_is_gated() {
    for prelude in [
        format!("  var e, t;\n  {GET_PROTO}, {LOCAL_T};\n  console.log(t);\n"),
        format!("  var e, t;\n  {LOCAL_T}, {GET_PROTO};\n"),
        format!("  var e, t;\n  {GET_PROTO}, console.log(1), {LOCAL_T};\n"),
        format!("  var e, t;\n  {GET_PROTO};\n  console.log(1);\n  {LOCAL_T};\n"),
        format!("  var e, t;\n  {GET_PROTO};\n  var x;\n  {LOCAL_T};\n"),
        format!("  var e, t;\n  t = installed, {LOCAL_T};\n"),
        format!("  var e, t;\n  t = Math.random(), {LOCAL_T};\n"),
        format!("  var e, t = 1;\n  {LOCAL_T};\n"),
        format!("  var e, t;\n  {GET_PROTO}, t = 2, {LOCAL_T};\n"),
        format!("  var e, t;\n  {GET_PROTO}, {LOCAL_T}, r.y = function () {{ return t; }};\n"),
        format!("  var e, t;\n  function t() {{}}\n  {LOCAL_T};\n"),
        format!("  var e, t;\n  t = __dirname, {LOCAL_T};\n"),
        "  var e;\n  r.t = function (v) { return e || __dirname; };\n".to_string(),
    ] {
        assert!(local_t(&prelude).is_none(), "{prelude}");
    }
}

fn with_member(extra: &str, consumer: &str) -> Vec<String> {
    let helpers = format!("{HELPERS}{extra}");
    printed_with(
        &source(&helpers, &[consumer, TARGET]),
        UnpackOptions::default(),
    )
    .expect("a bundle")
}

#[test]
fn a_literal_runtime_member_read_keeps_conversions() {
    let out = with_member(
        "  r.q = \"/assets/\";\n",
        "var m = n(1); console.log(m.a, n.q);",
    );
    assert!(converted(&out[1]), "{}", out[1]);
    let out = with_member(
        "  if (r) r.ab = __dirname + \"/native_modules/\";\n",
        "var m = n(1); console.log(m.a, n.ab);",
    );
    assert!(converted(&out[1]), "{}", out[1]);
}

#[test]
fn a_function_runtime_member_read_refuses_conversions() {
    let out = with_member(
        "  r.g = function () { return 1; };\n",
        "var m = n(1); console.log(m.a, n.g);",
    );
    assert!(!converted(&out[1]), "{}", out[1]);
    let helpers = format!("{HELPERS}  r.q = \"/assets/\";\n  r.q = function () {{ return 1; }};\n");
    let text = source(&helpers, &["var m = n(1); console.log(m.a, n.q);", TARGET]);
    assert!(printed_with(&text, UnpackOptions::default()).is_none());
}

fn local_t_raw(prelude: &str) -> Option<String> {
    let helpers = format!("{}{prelude}", helpers_without_t());
    let text = source(&helpers, &["console.log(n.t({}, 2));"]);
    let alloc = Allocator::default();
    let program = common::parse(&alloc, &text);
    let options = UnpackOptions::default();
    let mut bundle = unpack_program_with(&alloc, &program, options)?;
    finish_with(&alloc, &mut bundle, &[], options);
    let runtime = bundle.modules.values().last()?;
    Some(print(runtime.program(), PrintStyle::Pretty))
}

#[test]
fn the_unminified_sequence_shape_moves_and_its_order_is_checked() {
    let runtime = local_t_raw(&format!(
        "  var e, t;\n  {GET_PROTO}, {LOCAL_T}, r.x = 1;\n"
    ))
    .expect("a bundle");
    assert!(runtime.contains("var e, t;"), "{runtime}");
    for prelude in [
        format!("  var e, t;\n  {LOCAL_T}, {GET_PROTO};\n"),
        format!("  var e, t;\n  {GET_PROTO}, console.log(1), {LOCAL_T};\n"),
        format!("  var e, t;\n  {GET_PROTO}, t = 2, {LOCAL_T};\n"),
    ] {
        assert!(local_t_raw(&prelude).is_none(), "{prelude}");
    }
}

#[test]
fn the_define_must_lead_the_module() {
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "\"use strict\"; function a() { return 1; } n.d(t, { a: () => a });",
    ]);
    assert!(converted(&out[1]), "{}", out[1]);
    let out = unpacked(&[
        "var m = n(1); console.log(m.a);",
        "\"use strict\"; console.log(1); n.d(t, { a: () => a }); function a() { return 1; }",
    ]);
    assert!(!converted(&out[1]), "{}", out[1]);
}
