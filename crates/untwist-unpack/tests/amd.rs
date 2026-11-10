use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_testing::js_files_in;
use untwist_unpack::{holds_a_bundle, unpack, unpack_taking, BundleKind, UnpackResult};

type Files = Vec<(String, String)>;

fn files(result: Option<UnpackResult<'_>>) -> Option<Files> {
    let result = result?;
    assert_eq!(result.bundle.kind, BundleKind::Amd);
    Some(
        result
            .bundle
            .modules
            .values()
            .map(|module| {
                (
                    module.path.as_str().to_string(),
                    pretty_codegen(module.program()),
                )
            })
            .collect(),
    )
}

fn split(source: &str) -> Option<Files> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    files(unpack(&alloc, &parsed.program, &[]))
}

fn split_taken(source: &str) -> (Option<Files>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = files(unpack_taking(&alloc, &mut parsed.program, &[]));
    (found, pretty_codegen(&parsed.program))
}

fn code<'f>(files: &'f Files, path: &str) -> &'f str {
    files
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, code)| code.as_str())
        .unwrap_or_else(|| {
            panic!(
                "no {path} in {:?}",
                files.iter().map(|(p, _)| p).collect::<Vec<_>>()
            )
        })
}

fn paths(files: &Files) -> Vec<&str> {
    files.iter().map(|(path, _)| path.as_str()).collect()
}

fn refused(source: &str) {
    assert_eq!(split(source), None, "split:\n{source}");
    let alloc = Allocator::default();
    let before = pretty_codegen(&parse_program(&alloc, source).program);
    let (taken, after) = split_taken(source);
    assert_eq!(taken, None);
    assert_eq!(after, before, "the refused input is left intact");
}

#[test]
fn an_anonymous_define_becomes_the_entry_with_its_dependencies_required() {
    let files =
        split("define([\"exports\", \"path\"], function (exports, p) {\n  exports.v = p.sep;\n});")
            .expect("an amd bundle");
    assert_eq!(paths(&files), ["index.js"]);
    assert_eq!(
        code(&files, "index.js"),
        "var p = require(\"path\");\nexports.v = p.sep;"
    );
}

#[test]
fn named_defines_require_each_other_by_relative_path() {
    let files = split(
        "define(\"lib/a\", [], function () { return 1; });\n\
         define(\"lib/b\", [\"./a\"], function (a) { return a + 1; });\n\
         define(\"main\", [\"lib/b\", \"fs\"], function (b, fs) { console.log(b, fs); });",
    )
    .expect("an amd bundle");
    assert_eq!(paths(&files), ["lib/a.js", "lib/b.js", "index.js"]);
    assert_eq!(
        code(&files, "lib/b.js"),
        "var a = require(\"./a.js\");\nmodule.exports = a + 1;"
    );
    assert!(code(&files, "index.js")
        .starts_with("var b = require(\"./lib/b.js\");\nvar fs = require(\"fs\");\n"));
}

#[test]
fn a_return_is_assigned_unless_the_exports_dependency_makes_undefined_mean_the_object() {
    let pair = split(
        "define(\"a\", [], function () { var x; return x; });\ndefine(\"b\", [], function () { console.log(1); });\ndefine(\"m\", [\"a\", \"b\"], function (a, b) { var y; return y; });",
    )
    .expect("amd");
    assert_eq!(code(&pair, "a.js"), "var x;\nmodule.exports = x;");
    assert_eq!(
        code(&pair, "b.js"),
        "console.log(1);\nmodule.exports = void 0;"
    );
    assert!(code(&pair, "index.js").contains("if (__amd_exports !== undefined)"));
    let silent = split("define([], function () { console.log(1); });").expect("amd");
    assert_eq!(code(&silent, "index.js"), "console.log(1);");
    let guarded =
        split("define([\"exports\"], function (exports) { var x; return x; });").expect("amd");
    assert!(code(&guarded, "index.js")
        .contains("if (__amd_exports !== undefined) module.exports = __amd_exports;"));
    let literal =
        split("define([\"exports\"], function (exports) { return { a: 1 }; });").expect("amd");
    assert_eq!(code(&literal, "index.js"), "module.exports = { a: 1 };");
}

#[test]
fn strictness_of_the_file_or_the_factory_is_kept() {
    let kept = split("\"use strict\";\ndefine([], function () { return 1; });").expect("amd");
    assert!(code(&kept, "index.js").starts_with("\"use strict\";"));
    let own = split("define([], function () { \"use strict\"; return 1; });").expect("amd");
    assert!(code(&own, "index.js").starts_with("\"use strict\";"));
}

#[test]
fn the_probe_wants_a_leading_define_and_nothing_else_matches() {
    for (source, want) in [
        ("define([], function () { return 1; });", true),
        (";define(\"a\", [], function () {});", true),
        ("var x = 1; define([], function () {});", false),
        ("require([\"a\"], function () {});", false),
        ("(function () { define([], function () {}); })();", false),
    ] {
        let alloc = Allocator::default();
        let parsed = parse_program(&alloc, source);
        assert_eq!(holds_a_bundle(&parsed.program), want, "{source}");
    }
}

#[test]
fn mixed_anonymous_and_named_defines_are_refused() {
    refused("define(\"a\", [], function () {});\ndefine([], function () {});");
    refused("define([], function () {});\ndefine([], function () {});");
}

#[test]
fn a_shape_the_proof_does_not_cover_is_refused() {
    for source in [
        "var d = [];\ndefine([...d], function () {});",
        "define([\"a\" + \"b\"], function () {});",
        "define({ a: 1 });",
        "define([], () => 1);",
        "define([], async function () {});",
        "define([], function* () {});",
        "define(function (require) { require(\"a\"); });",
        "define([], function (a) {});",
        "define([\"require\"], function (r) {});",
        "define([\"module\"], function (m) {});",
        "define([\"text!a.txt\"], function (a) {});",
        "define([], function ({ a }) {});",
        "define([\"path\"], function (a = 1) {});",
        "define([], function (a = 1) {});",
        "define(\"a\", [], function () {});\ndefine(\"a\", [], function () {});",
        "define(\"a\", [\"./b\"], function (b) {});",
        "define([\"./b\"], function (b) {});",
        "define(\"a\", [\"../b\"], function (b) {});\ndefine(\"b\", [], function () {});",
    ] {
        refused(source);
    }
}

#[test]
fn a_dependency_cycle_is_refused() {
    refused(
        "define(\"a\", [\"b\"], function (b) {});\ndefine(\"b\", [\"a\"], function (a) {});\ndefine(\"m\", [\"a\"], function (a) {});",
    );
    refused("define(\"a\", [\"a\"], function (a) {});");
}

#[test]
fn a_factory_that_is_not_closed_is_refused() {
    for source in [
        "define([], function () { return this; });",
        "define([], function () { return () => this; });",
        "define([], function () { return arguments; });",
        "define([], function () { return eval(\"1\"); });",
        "define([], function () { return typeof module; });",
        "define([], function () { return require; });",
        "define([], function () { exports.a = 1; });",
        "define([], function () { return __dirname; });",
        "define([], function () { if (a) { return 1; } return 2; });",
        "define([], function () { return 1; console.log(2); });",
        "define([], function () { var module = 1; return module; });",
        "define([\"exports\"], function (exports) { var require = 1; });",
        "define([\"path\"], function (exports) { return exports; });",
        "define([\"path\"], function (require) { return require; });",
        "define([], function () { var __amd_exports = 1; return __amd_exports; });",
    ] {
        refused(source);
    }
}

#[test]
fn a_function_declaration_that_takes_a_parameters_name_is_refused() {
    for source in [
        "define(\"dep\", [], function () { return \"dep value\"; });\ndefine(\"main\", [\"dep\"], function (a) { function a() { return \"inner function\"; } console.log(typeof a === \"function\" ? a() : a); });",
        "define([\"path\"], function (a) { function a() {} a(); });",
        "define([\"exports\"], function (e) { function e() {} e(); });",
        "define([\"exports\"], function (exports) { function exports() {} exports(); });",
        "define([\"path\"], function (a) { if (a) { function a() {} } console.log(a); });",
        "define([\"path\"], function (a) { try { a(); } catch (error) { function a() {} } console.log(a); });",
    ] {
        refused(source);
    }
}

#[test]
fn a_parameters_name_declared_in_a_nested_function_is_not_the_parameter() {
    let files = split(
        "define([\"path\"], function (a) {\n  var f = function () { function a() {} return a; };\n  var g = function a() {};\n  console.log(a.sep, f, g);\n});",
    )
    .expect("amd");
    assert_eq!(
        code(&files, "index.js"),
        "var a = require(\"path\");\nvar f = function() {\n  function a() {}\n  return a;\n};\nvar g = function a() {};\nconsole.log(a.sep, f, g);"
    );
}

#[test]
fn a_define_that_is_bound_or_a_top_level_binding_the_factory_reads_is_refused() {
    refused("function define() {}\ndefine([], function () {});");
    refused("var shared = 1;\ndefine([], function () { return shared; });");
}

#[test]
fn a_this_inside_a_nested_function_is_fine_and_a_nested_return_does_not_count() {
    let files = split(
        "define([], function () {\n  function f() { return this; }\n  var g = () => { return 1; };\n  return { f: f, g: g };\n});",
    )
    .expect("amd");
    assert_eq!(
        code(&files, "index.js"),
        "function f() {\n  return this;\n}\nvar g = () => {\n  return 1;\n};\nmodule.exports = {\n  f,\n  g\n};"
    );
}

#[test]
fn the_fixtures_extract_alike_taken_and_cloned() {
    let mut found = 0;
    for (label, source) in js_files_in("fixtures/unpack-bundlers/amd") {
        let cloned = split(&source);
        let (taken, _) = split_taken(&source);
        assert_eq!(taken, cloned, "{label}");
        found += usize::from(cloned.is_some());
    }
    assert_eq!(found, 3);
}

#[test]
fn a_factory_holding_a_with_statement_is_refused() {
    refused(
        "define([\"require\", \"exports\"], function (r, e) {\n  var o = { e: { tag: \"inner\" } };\n  with (o) { e.x = 1; }\n  console.log(Object.keys(e), Object.keys(o.e));\n});",
    );
    refused("define([\"dep\"], function (dep) { with ({ dep: 1 }) { return dep; } });");
}

#[test]
fn a_callback_with_a_repeated_parameter_is_refused() {
    refused("define([\"fs\"], function (dep, dep) { console.log(typeof dep); });");
    refused("define([\"fs\", \"path\"], function (dep, dep) { console.log(dep.sep); });");
    refused("define(\"a\", [], function () { return 1; });\ndefine(\"main\", [\"a\"], function (x, x) { console.log(x); });");
}
