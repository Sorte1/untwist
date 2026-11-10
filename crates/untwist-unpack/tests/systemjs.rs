mod common;

use common::{parse, print_module};
use oxc_allocator::Allocator;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking};

fn files(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let result = unpack(&alloc, &program, &[])?;
    assert_eq!(result.bundle.kind.as_str(), "systemjs");
    let mut files: Vec<(String, String)> = result
        .bundle
        .modules
        .values()
        .map(|module| (module.path.as_str().to_string(), print_module(module)))
        .collect();
    files.sort();
    Some(files)
}

fn accepted(source: &str) -> Vec<(String, String)> {
    files(source).unwrap_or_else(|| panic!("refused:\n{source}"))
}

fn refused(source: &str) {
    assert_eq!(files(source), None, "accepted:\n{source}");
}

fn code_of(files: &[(String, String)], path: &str) -> String {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map(|(_, code)| code.clone())
        .unwrap_or_else(|| panic!("no {path} in {files:?}"))
}

fn single(vars: &str, body: &str) -> String {
    format!(
        "System.register([], function (_export, _context) {{\n  \"use strict\";\n  {vars}\n  return {{\n    execute: function () {{\n{body}\n    }}\n  }};\n}});\n"
    )
}

fn named(name: &str, deps: &str, vars: &str, setters: &str, body: &str) -> String {
    format!(
        "System.register(\"{name}\", {deps}, function (_export, _context) {{\n  \"use strict\";\n  {vars}\n  return {{\n    setters: [{setters}],\n    execute: function () {{\n{body}\n    }}\n  }};\n}});\n"
    )
}

const LEAF: &str = "System.register(\"leaf\", [], function (_export) {\n  \"use strict\";\n  return {\n    execute: function () {\n      _export(\"a\", 1);\n      _export(\"b\", 2);\n    }\n  };\n});\n";

fn importing(setter: &str, body: &str) -> String {
    format!(
        "{}{LEAF}",
        named("main", "[\"leaf\"]", "var a, b, ns;", setter, body)
    )
}

#[test]
fn an_anonymous_register_becomes_an_esm_entry() {
    let files = accepted(&single(
        "var value;",
        "console.log(1); _export(\"value\", value = 3); console.log(value);",
    ));
    let index = code_of(&files, "index.js");
    assert!(index.starts_with("\"use strict\";\nvar value;"), "{index}");
    assert!(index.contains("value = 3;"), "{index}");
    assert!(index.ends_with("export { value };"), "{index}");
}

#[test]
fn named_registers_link_by_path() {
    let main = named(
        "main",
        "[\"lib\", \"lazy\", \"path\"]",
        "var greet, counter, ns, sep;",
        "function (m) { greet = m.greet; counter = m.counter; _export(\"total\", m.total); }, function (m) { ns = m; }, function (m) { sep = m.default; }",
        "      var v = ns.value;\n      console.log(greet(\"x\"), counter, v, sep);\n      _context.import(\"lazy\").then(function (n) { console.log(n.value); });",
    );
    let lib = named(
        "lib",
        "[]",
        "var total;",
        "",
        "      _export({ greet: greet });\n      let counter = _export(\"counter\", 0);\n      function greet(n) { _export(\"counter\", counter += 1); return n; }\n      _export(\"total\", total = 9);\n      _export(\"double\", function (n) { return n * 2; });",
    );
    let lazy = named(
        "lazy",
        "[]",
        "var value;",
        "",
        "      _export(\"value\", value = 42);",
    );
    let files = accepted(&format!("{main}{lib}{lazy}"));
    let index = code_of(&files, "index.js");
    assert!(
        index.contains("import { greet, counter } from \"./lib.js\";"),
        "{index}"
    );
    assert!(
        index.contains("export { total } from \"./lib.js\";"),
        "{index}"
    );
    assert!(
        index.contains("import * as ns from \"./lazy.js\";"),
        "{index}"
    );
    assert!(
        index.contains("import { default as sep } from \"path\";"),
        "{index}"
    );
    assert!(index.contains("import(\"./lazy.js\")"), "{index}");
    let lib = code_of(&files, "lib.js");
    assert!(lib.contains("let counter = 0;"), "{lib}");
    assert!(lib.contains("var total, double;"), "{lib}");
    assert!(lib.contains("double = function"), "{lib}");
    assert!(
        lib.ends_with("export { greet, counter, total, double };"),
        "{lib}"
    );
}

#[test]
fn taken_bodies_extract_like_cloned_ones() {
    let source = format!(
        "{}{LEAF}",
        named(
            "main",
            "[\"leaf\"]",
            "var a;",
            "function (m) { a = m.a; }",
            "      console.log(a);"
        )
    );
    let cloned_arena = Allocator::default();
    let cloned = parse(&cloned_arena, &source);
    let expected: Vec<String> = unpack(&cloned_arena, &cloned, &[])
        .expect("cloned")
        .bundle
        .modules
        .values()
        .map(print_module)
        .collect();
    let taken_arena = Allocator::default();
    let mut taken = parse(&taken_arena, &source);
    let found: Vec<String> = unpack_taking(&taken_arena, &mut taken, &[])
        .expect("taken")
        .bundle
        .modules
        .values()
        .map(print_module)
        .collect();
    assert_eq!(found, expected);
    assert!(!pretty_codegen(&taken).contains("console.log"));
}

#[test]
fn a_wrapper_iife_and_dead_anonymous_register_are_handled() {
    let source = format!(
        "(function () {{\n{}{}}})();\n",
        single("", "console.log(\"dead\");"),
        LEAF
    );
    let files = accepted(&source);
    assert_eq!(files.len(), 1, "{files:?}");
    assert!(code_of(&files, "index.js").contains("a = 1"));
}

#[test]
fn object_form_class_and_function_exports() {
    let files = accepted(&single(
        "",
        "_export({ f: f });\nfunction f() {}\nclass C {}\n_export(\"g\", C);",
    ));
    assert!(code_of(&files, "index.js").ends_with("export { f, C as g };"));
}

#[test]
fn layout_refusals() {
    refused(&format!("var x = 1;\n{}", single("", "")));
    refused("var d = [];\nSystem.register(d, function (_export) { \"use strict\"; return { execute: function () {} }; });");
    refused("System.register([], function (_export = 1) { \"use strict\"; return { execute: function () {} }; });");
    refused("System.register([], function ({ e }) { \"use strict\"; return { execute: function () {} }; });");
    refused("System.register([\"a\" + \"\"], function (_export) { \"use strict\"; return { execute: function () {} }; });");
    refused(&format!(
        "(function (System) {{\n{}}})();\n",
        single("", "")
    ));
    accepted(&format!(
        "(function () {{\n{}}}).call(this);\n",
        single("", "")
    ));
}

#[test]
fn declare_body_holds_only_vars_and_the_return() {
    refused(&single("function f() {}", ""));
    refused(&single("var x = 1;", ""));
}

#[test]
fn setters_must_only_bind() {
    refused(&importing("function (m) { a = m.a; console.log(m); }", ""));
    refused(&importing("function (m) { a = m.a; }, function (m) {}", ""));
    refused(&importing("function (m) { a = m.a; a = m.b; }", ""));
    refused(&importing("function (m) { zz = m.a; }", ""));
}

#[test]
fn names_and_deps() {
    refused(&format!("{LEAF}{LEAF}"));
    refused(&format!("{}{}", single("", ""), single("", "")));
    refused(&named("main", "[\"./x.js\"]", "", "", ""));
    refused(&named(
        "main",
        "[\"path\"]",
        "var p;",
        "function (m) { p = m.sep; }",
        "",
    ));
    refused(&named("main", "[\"path\", \"path\"]", "", "", ""));
}

#[test]
fn cycles_are_refused() {
    let a = named("a", "[\"b\"]", "", "", "");
    let b = named("b", "[\"a\"]", "", "", "");
    refused(&format!("{a}{b}"));
}

#[test]
fn sloppy_declare_is_refused() {
    refused("System.register([], function (_export) { return { execute: function () {} }; });");
}

#[test]
fn execute_shape() {
    refused("System.register([], function (_export) { \"use strict\"; return { execute: function (x) {} }; });");
    refused("System.register([], function (_export) { \"use strict\"; return { execute: async function () {} }; });");
}

#[test]
fn host_names_are_refused() {
    refused(&single("", "require(\"path\");"));
    refused(&single("", "console.log(System);"));
    refused(&single("", "console.log(__dirname);"));
}

#[test]
fn top_level_semantics() {
    refused(&single("", "console.log(this);"));
    refused(&single("", "console.log(arguments);"));
    refused(&single("", "eval(\"1\");"));
    refused(&single("", "return;"));
    refused(&single("", "console.log(import.meta);"));
    refused(&single("", "import(\"path\");"));
    refused(&single("", "console.log(new.target);"));
    refused(&single("", "var await = 1;"));
}

#[test]
fn a_field_decorator_is_scanned_in_the_enclosing_frame() {
    accepted(&single("", "class A { x = 1; }\nconsole.log(new A().x);"));
    for decorator in [
        "@(console.log(this), Object)",
        "@(console.log(__dirname), Object)",
    ] {
        refused(&single(
            "",
            &format!("class A {{ {decorator} x = 1; }}\nconsole.log(new A().x);"),
        ));
        refused(&single(
            "",
            &format!("class A {{ {decorator} accessor y = 1; }}\nconsole.log(new A().y);"),
        ));
    }
}

#[test]
fn an_object_export_only_counts_as_a_whole_statement() {
    accepted(&single("", "_export({ c: 1 });"));
    refused(&single("", "_export({ c: 1 }), 0;"));
    refused(&single("", "if (1) _export({ c: 1 });").replace("if (1) ", "if (1) 0, "));
}

#[test]
fn inner_functions_keep_their_own_semantics() {
    accepted(&single(
        "",
        "function f() { return this && arguments && new.target; }\nvar g = () => { return 1; };",
    ));
}

#[test]
fn context_and_export_params() {
    refused(&single("", "console.log(_context.meta);"));
    refused(&single("", "_context.import(\"./x.js\");"));
    refused(&single("", "var e = _export;"));
    refused(&single("", "function f(_export) {}"));
    refused(&single("var value;", "var value = 1;"));
}

#[test]
fn imports_are_read_only() {
    refused(&importing("function (m) { a = m.a; }", "a = 2;"));
    refused(&importing("function (m) { ns = m; }", "ns.a();"));
    refused(&importing("function (m) { ns = m; }", "console.log(ns);"));
    refused(&importing("function (m) { ns = m; }", "ns.a = 1;"));
    accepted(&importing(
        "function (m) { ns = m; }",
        "console.log(ns.a, ns[\"b\"]);",
    ));
}

#[test]
fn export_locals_are_written_only_through_export() {
    refused(&single("var c;", "_export(\"c\", c = 1); c = 2;"));
    refused(&single("", "let c = 0; _export(\"c\", c += 1);"));
    refused(&single("", "let c = _export(\"c\", 0); c++;"));
    refused(&single(
        "",
        "let c = _export(\"c\", 0); _export(\"c\", c++);",
    ));
    refused(&single("", "var c = 0;\n_export(\"c\", c++);"));
    accepted(&single(
        "",
        "let c = _export(\"c\", 0); _export(\"c\", ++c);",
    ));
}

#[test]
fn identifier_exports_need_a_top_level_export_after_the_declaration() {
    refused(&format!(
        "{}{}",
        named(
            "main",
            "[\"lib\"]",
            "var f;",
            "function (m) { f = m.f; }",
            ""
        ),
        named(
            "lib",
            "[]",
            "",
            "",
            "function f() {}\nfunction g() { _export(\"f\", f); }"
        )
    ));
    refused(&single("", "_export(\"x\", x);\nvar x = 1;"));
    refused(&single("", "var x = 1;\n_export(\"x\", x);\nx = 2;"));
    refused(&single("", "{ var x = 1; }\n_export(\"x\", x);"));
    accepted(&single("", "var x = 1;\n_export(\"x\", x);"));
    accepted(&format!(
        "{}{}",
        named("main", "[\"lib\"]", "var c;", "function (m) { c = m.c; }", ""),
        named(
            "lib",
            "[]",
            "",
            "",
            "let c = _export(\"c\", 0);\nfunction f() { _export(\"c\", c += 1); }\n_export(\"c\", c);"
        )
    ));
    refused(&single("", "var c;\nvar c;\n_export(\"c\", c = 1);"));
}

#[test]
fn export_forms() {
    refused(&single(
        "var c;",
        "_export(\"c\", c = 1); _export(\"c\", 2);",
    ));
    refused(&importing("function (m) { a = m.a; }", "").replace(
        "_export(\"b\", 2);",
        "_export(\"b\", 2);\n      console.log(_export({ c: 1 }));",
    ));
    refused(&single("", "_export(\"__esModule\", true);"));
    refused(&single("", "_export(\"a-b\", 1);"));
    refused(&single("", "_export(\"a\");"));
}

#[test]
fn imported_names_must_be_exported() {
    refused(&importing("function (m) { a = m.c; }", ""));
    refused(&importing("function (m) { _export(\"c\", m.c); }", ""));
    refused(&importing(
        "function (m) { _export(\"a\", m.a); }",
        "_export(\"a\", 1);",
    ));
    accepted(&importing(
        "function (m) { a = m.a; _export(\"b\", m.b); }",
        "",
    ));
}

#[test]
fn observable_namespaces_keep_their_key_order() {
    refused(&single("", "_export(\"b\", 1); _export(\"a\", 2);"));
    refused(&single(
        "",
        "_export(\"a\", 1); function f() { _export(\"b\", 2); }",
    ));
    refused(&single(
        "",
        "_export(\"a\", 1); function f() { let b = _export(\"b\", 2); }",
    ));
    let lazy = named(
        "lazy",
        "[]",
        "",
        "",
        "_export(\"b\", 1); _export(\"a\", 2);",
    );
    refused(&format!(
        "{}{lazy}",
        named("main", "[]", "", "", "_context.import(\"lazy\");")
    ));
    accepted(&format!(
        "{}{}",
        named("main", "[]", "", "", "_context.import(\"lazy\");"),
        named(
            "lazy",
            "[]",
            "",
            "",
            "_export(\"a\", 1); _export(\"b\", 2);"
        )
    ));
    accepted(&format!(
        "{}{}",
        named(
            "main",
            "[\"leaf2\"]",
            "var x;",
            "function (m) { x = m.b; }",
            ""
        ),
        named(
            "leaf2",
            "[]",
            "",
            "",
            "_export(\"b\", 1); _export(\"a\", 2);"
        )
    ));
}

#[test]
fn a_webpack_bundle_inside_execute_is_found_by_reference_first() {
    let webpack = "(function (modules) {\n  var cache = {};\n  function __webpack_require__(id) {\n    if (cache[id]) return cache[id].exports;\n    var module = cache[id] = { exports: {} };\n    modules[id].call(module.exports, module, module.exports, __webpack_require__);\n    return module.exports;\n  }\n  return __webpack_require__(0);\n})([function (module, exports, __webpack_require__) {\n  __webpack_require__(1);\n}, function (module, exports) {\n  exports.a = 1;\n}]);";
    let source = single("", webpack);
    let alloc = Allocator::default();
    let program = parse(&alloc, &source);
    let result = unpack(&alloc, &program, &[]).expect("webpack inside execute");
    assert_eq!(result.bundle.kind.as_str(), "webpack");
}

#[test]
fn fresh_locals_avoid_every_name_the_module_uses() {
    let files = accepted(&format!(
        "{}{}",
        named("main", "[\"lib\"]", "var k, d, f, m;", "function (n) { k = n.k; d = n.default; f = n.f; m = n.m; }", ""),
        named(
            "lib",
            "[]",
            "var q;",
            "",
            "console.log(typeof k);\n_export(\"k\", 1);\n_export(\"default\", 2);\nfunction g(f) { _export(\"f\", 3); }\n_export(\"f\", 4);\n_export(\"m\", 5);\n_export(\"q\", 6);"
        )
    ));
    let lib = code_of(&files, "lib.js");
    assert!(lib.contains("var q, $k, $default, $f, m, $q;"), "{lib}");
    assert!(lib.contains("function g(f) {\n  $f = 3;"), "{lib}");
    assert!(
        lib.ends_with("export { $k as k, $default as default, $f as f, m, $q as q };"),
        "{lib}"
    );
}

#[test]
fn a_sequence_around_the_wrapper_iife_is_refused() {
    let register = single("", "console.log(\"x\");");
    let plain = format!("(function () {{\n{register}}})();\n");
    accepted(&plain);
    refused(&format!("f(), function () {{\n{register}}}();\n"));
    refused(&format!("!(f(), function () {{\n{register}}}());\n"));
}

#[test]
fn super_in_a_method_form_execute_refuses() {
    let method = "System.register(\"main\", [], function (_export) {\n  \"use strict\";\n  return { execute() { BODY } };\n});\n";
    let source = method.replace("BODY", "console.log(typeof super.hasOwnProperty);");
    refused(&source);
    let alloc = Allocator::default();
    let program = parse(&alloc, &source);
    assert_eq!(
        untwist_unpack::detect::refused_kind(&program, Default::default()),
        Some(untwist_unpack::BundleKind::SystemJs)
    );
    refused(&method.replace("BODY", "var f = () => super.x;\nconsole.log(f());"));
    for nested in [
        "class A { m() { return super.toString; } }\nconsole.log(new A().m());",
        "var o = { m() { return super.toString; } };\nconsole.log(o.m());",
    ] {
        accepted(&method.replace("BODY", nested));
    }
}
