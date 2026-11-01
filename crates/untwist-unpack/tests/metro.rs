mod common;

use common::{parse, print_module};
use oxc_allocator::Allocator;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking};

const PARAMS: &str =
    "global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap";
const LOADER: &str = "var modules = Object.create(null);
function __d(factory, moduleId, dependencyMap) {
  modules[moduleId] = { factory: factory, dependencyMap: dependencyMap, module: null };
}
function __r(moduleId) {
  var record = modules[moduleId];
  if (record.module) return record.module.exports;
  var module = (record.module = { exports: {} });
  record.factory(globalThis, __r, null, null, module, module.exports, record.dependencyMap);
  return module.exports;
}
";
const LEAF: &str = "__d(function (global, require, _$$_IMPORT_DEFAULT, _$$_IMPORT_ALL, module, exports, _dependencyMap) {
  module.exports = { value: 2 };
}, 1, []);
";

fn def(id: &str, deps: &str, body: &str) -> String {
    format!("__d(function ({PARAMS}) {{\n{body}\n}}, {id}, {deps});\n")
}

fn bundle_with(prelude: &str, entry_body: &str) -> String {
    format!(
        "{LOADER}{prelude}{LEAF}{}__r(0);\n",
        def("0", "[1]", entry_body)
    )
}

fn modules(source: &str) -> Option<Vec<(String, String)>> {
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let result = unpack(&alloc, &program, &[])?;
    assert_eq!(result.bundle.kind.as_str(), "metro");
    let mut files: Vec<(String, String)> = result
        .bundle
        .modules
        .values()
        .map(|module| (module.path.as_str().to_string(), print_module(module)))
        .collect();
    files.sort();
    Some(files)
}

fn code_of(files: &[(String, String)], path: &str) -> String {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map(|(_, code)| code.clone())
        .unwrap_or_else(|| panic!("no {path} in {files:?}"))
}

fn refused(prelude: &str, entry_body: &str) -> bool {
    modules(&bundle_with(prelude, entry_body)).is_none()
}

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/unpack-bundlers/metro")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"))
}

#[test]
fn the_smallest_bundle_splits_with_paths_and_the_loader_in_the_runtime() {
    let files = modules(&fixture("min-metro.js")).expect("a metro bundle");
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, ["1.js", "_runtime/metro.js", "index.js"]);
    assert_eq!(
        code_of(&files, "index.js"),
        "require(\"./_runtime/metro.js\");\nconsole.log(\"eval 0\", require(\"./1.js\").value);"
    );
    assert!(code_of(&files, "_runtime/metro.js").starts_with("var modules = Object.create(null);"));
}

#[test]
fn the_template_is_accepted() {
    let files = modules(&bundle_with(
        "",
        "console.log(require(_dependencyMap[0]).value);",
    ))
    .expect("accepted");
    assert_eq!(
        code_of(&files, "index.js"),
        "require(\"./_runtime/metro.js\");\nconsole.log(require(\"./1.js\").value);"
    );
}

#[test]
fn old_factories_and_a_bad_call_are_refused() {
    assert!(modules(&fixture("neg-metro-five-params.js")).is_none());
    assert!(modules(&fixture("neg-metro-bad-call.js")).is_none());
    for params in [
        "global, require, module, exports, _dependencyMap",
        "global, require, module, exports, _dependencyMap, x",
    ] {
        let old =
            format!("{LOADER}__d(function ({params}) {{\n  console.log(1);\n}}, 0);\n__r(0);\n");
        assert!(modules(&old).is_none(), "{params}");
    }
}

#[test]
fn prelude_bindings_move_to_the_runtime_and_are_imported() {
    let files = modules(&bundle_with(
        "var __DEV__ = false, prefix = '';\n",
        "console.log(__DEV__, `${prefix}x`);",
    ))
    .expect("accepted");
    let index = code_of(&files, "index.js");
    assert!(
        index.starts_with("var { __DEV__, prefix } = require(\"./_runtime/metro.js\");"),
        "{index}"
    );
    let runtime = code_of(&files, "_runtime/metro.js");
    assert!(
        runtime.ends_with("module.exports = {\n  __DEV__,\n  prefix\n};"),
        "{runtime}"
    );
}

#[test]
fn a_prelude_binding_that_is_written_or_mutated_by_the_loader_is_refused() {
    assert!(refused("var flag = 1;\nflag = 2;\n", "console.log(flag);"));
    assert!(refused("var flag = 1;\n", "flag = 3;"));
    assert!(refused(
        "var flag = 1;\nvar flag = 2;\n",
        "console.log(flag);"
    ));
    assert!(refused("", "console.log(modules);"));
    assert!(refused(
        "var seen = {};\nfunction note(k) { seen[k] = 1; }\n",
        "console.log(seen);"
    ));
    assert!(refused(
        "var seen = {};\nfunction alias() { return seen; }\n",
        "console.log(seen);"
    ));
    assert!(refused(
        "var seen = {};\nvar note = (k) => { seen[k] = 1; };\n",
        "console.log(seen);"
    ));
    assert!(refused(
        "var seen = {};\nclass Alias { held = seen; }\n",
        "console.log(seen);"
    ));
}

#[test]
fn a_factory_reading_a_bundle_function_is_refused() {
    assert!(refused("function helper() { return 1; }\n", "helper();"));
    assert!(refused("", "__r(1);"));
    assert!(refused("", "__d(function () {}, 9);"));
    assert!(refused("", "console.log(arguments);"));
    assert!(modules(&bundle_with("", "function f() { return arguments; }")).is_some());
    assert!(refused("", "eval('1');"));
    assert!(modules(&bundle_with("", "var eval = String;\neval('1');")).is_some());
    assert!(modules(&bundle_with("", "(0, eval)('1');")).is_some());
    let global_loader = format!(
        "{LEAF}{}__r(0);\n",
        def("0", "[1]", "require(_dependencyMap[0]);")
    );
    assert!(modules(&global_loader).is_some());
    assert!(modules(&global_loader.replace("require(_dependencyMap[0]);", "__r(1);")).is_none());
    assert!(modules(&global_loader.replace("require(_dependencyMap[0]);", "__d(f, 2);")).is_none());
}

#[test]
fn the_dependency_map_is_inlined_or_copied_and_never_escapes() {
    let files = modules(&bundle_with(
        "",
        "console.log(_dependencyMap[0], _dependencyMap.paths);",
    ))
    .expect("accepted");
    let index = code_of(&files, "index.js");
    assert!(index.contains("var _dependencyMap = [1];"), "{index}");
    assert!(
        index.contains("console.log(1, _dependencyMap.paths);"),
        "{index}"
    );
    assert!(refused("", "console.log(_dependencyMap);"));
    assert!(refused("", "console.log(_dependencyMap[1]);"));
    assert!(refused("", "console.log(_dependencyMap[0 + 0]);"));
    assert!(refused("", "_dependencyMap[0] = 2;"));
    assert!(refused("", "_dependencyMap.push(2);"));
    assert!(modules(&format!(
        "{LOADER}{}__r(0);\n",
        def("0", "[]", "_dependencyMap.paths;").replace(", []);", ");")
    ))
    .is_none());
}

#[test]
fn module_is_read_only_through_exports_or_typeof() {
    assert!(modules(&bundle_with("", "module.exports.x = typeof module;")).is_some());
    assert!(refused("", "console.log(module.id);"));
    assert!(refused("", "console.log(module);"));
}

#[test]
fn an_escaped_require_routes_to_the_runtime_and_others_refuse() {
    let files = modules(&bundle_with(
        "",
        "var load = function (id) { return require(id); };\nrequire.importAll(1);",
    ))
    .expect("accepted");
    let index = code_of(&files, "index.js");
    assert!(
        index.starts_with("var { metroRequire: _require } = require(\"./_runtime/metro.js\");"),
        "{index}"
    );
    assert!(index.contains("return _require(id);"), "{index}");
    assert!(index.contains("_require.importAll(1);"), "{index}");
    let runtime = code_of(&files, "_runtime/metro.js");
    assert!(
        runtime.contains(
            "var metroModulePaths = {\n  \"1\": \"../1.js\",\n  \"0\": \"../index.js\"\n};"
        ),
        "{runtime}"
    );
    assert!(refused("", "var r = require;"));
    let shadowed = bundle_with(
        "var _require = 1;\n",
        "console.log(_require, require.importAll(1));",
    );
    let index = code_of(&modules(&shadowed).expect("accepted"), "index.js");
    assert!(
        index.starts_with("var { metroRequire: _require2, _require }"),
        "{index}"
    );
    assert!(refused("", "require.context('.');"));
    assert!(refused("", "require.importAll = null;"));
}

#[test]
fn import_helpers_come_from_the_runtime() {
    let files = modules(&bundle_with(
        "",
        "var a = _$$_IMPORT_DEFAULT(_dependencyMap[0]);\nvar b = _$$_IMPORT_ALL(_dependencyMap[0]);",
    ))
    .expect("accepted");
    let index = code_of(&files, "index.js");
    assert!(index.starts_with("var { metroImportDefault: _$$_IMPORT_DEFAULT, metroImportAll: _$$_IMPORT_ALL } = require(\"./_runtime/metro.js\");"), "{index}");
    assert!(index.contains("var a = _$$_IMPORT_DEFAULT(1);"), "{index}");
}

#[test]
fn top_level_this_host_names_and_shadowing_refuse() {
    assert!(refused("", "console.log(this);"));
    assert!(refused("", "class C { @__r x = 1; }"));
    assert!(modules(&bundle_with(
        "",
        "function f() { return this; }\nclass C { x = this; }"
    ))
    .is_some());
    assert!(refused("", "console.log(__filename);"));
    assert!(refused("", "function f(require) { return require; }"));
    assert!(refused("", "var globalThis = 1;"));
    assert!(refused("", "with ({}) { require(_dependencyMap[0]); }"));
    assert!(refused("", "var _dependencyMap = 1;"));
    assert!(refused(
        "",
        "function f(_dependencyMap) { return _dependencyMap; }"
    ));
    assert!(refused("", "var global;\nconsole.log(global);"));
    let bound_only = modules(&bundle_with(
        "var flag = 1;\n",
        "function f(global, flag) { return [global, flag]; }\nconsole.log(f(1, 2));",
    ))
    .expect("accepted");
    let index = code_of(&bound_only, "index.js");
    assert!(
        !index.contains("var global") && !index.contains("flag }"),
        "{index}"
    );
    let shadow = modules(&bundle_with(
        "var flag = 1;\n",
        "function f(global, flag) { return [global, flag]; }\nconsole.log(f(global, flag));",
    ))
    .expect("accepted");
    let index = code_of(&shadow, "index.js");
    assert!(index.contains("var global = globalThis;"), "{index}");
    assert!(index.contains("return [global, flag];"), "{index}");
    assert!(index.starts_with("var { flag } = require("), "{index}");
    assert!(refused("if (typeof module === 'object') {}\n", ""));
    assert!(refused("var exports = {};\n", ""));
}

#[test]
fn minified_parameters_are_renamed_without_capturing_host_names() {
    let minified = format!(
        "{LOADER}{LEAF}__d(function (g, r, d, a, m, e, n) {{\n  m.exports = r(n[0]);\n  g.x = 1;\n}}, 0, [1]);\n__r(0);\n"
    );
    let files = modules(&minified).expect("accepted");
    let index = code_of(&files, "index.js");
    assert!(index.contains("var global = globalThis;"), "{index}");
    assert!(
        index.contains("module.exports = require(\"./1.js\");"),
        "{index}"
    );
    let captured = minified.replace("g.x = 1;", "require('fs');");
    assert!(modules(&captured).is_none());
    let global = minified.replace("g.x = 1;", "global.x = 1;");
    assert!(modules(&global).is_none());
}

#[test]
fn layout_refusals() {
    let base = format!("{LOADER}{LEAF}{}", def("0", "[1]", ""));
    assert!(modules(&format!("{base}__r(0);\n")).is_some());
    assert!(modules(&format!("{base}{LEAF}__r(0);\n")).is_none());
    assert!(modules(&format!(
        "{base}__r(0);\n{}",
        LEAF.replace("}, 1, []);", "}, 2, []);")
    ))
    .is_none());
    assert!(modules(&format!("{base}__r(7);\n")).is_none());
    assert!(modules(&format!("{base}console.log(1);\n__r(0);\n")).is_none());
    assert!(modules(&format!("\"use strict\";\n{base}__r(0);\n")).is_none());
    let extra = base.replace("}, 0, [1]);", "}, 0, [1], f());");
    assert!(modules(&format!("{extra}__r(0);\n")).is_none());
    let named = base.replace("}, 0, [1]);", "}, 0, [1], \"src/index.js\");");
    assert!(modules(&format!("{named}__r(0);\n")).is_some());
    let computed = base.replace("}, 0, [1]);", "}, 0, [x]);");
    assert!(modules(&format!("{computed}__r(0);\n")).is_none());
}

#[test]
fn several_run_calls_get_a_synthesized_entry_in_order() {
    let source = format!(
        "{LOADER}{LEAF}{}__r(1);\n__r(0);\n",
        def("0", "[]", "console.log(0);")
    );
    let files = modules(&source).expect("accepted");
    assert_eq!(
        code_of(&files, "index.js"),
        "require(\"./_runtime/metro.js\");\nrequire(\"./1.js\");\nrequire(\"./0.js\");"
    );
}

fn described(source: &str, taking: bool) -> Option<Vec<String>> {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    let result = match taking {
        true => unpack_taking(&alloc, &mut program, &[])?,
        false => unpack(&alloc, &program, &[])?,
    };
    let mut lines: Vec<String> = result
        .bundle
        .modules
        .values()
        .map(|m| {
            format!(
                "{} {} {}\n{}",
                m.id,
                m.path.as_str(),
                m.is_entry,
                pretty_codegen(m.program())
            )
        })
        .collect();
    lines.extend(result.notes.iter().map(|note| note.message.clone()));
    Some(lines)
}

#[test]
fn taken_bodies_extract_like_cloned_ones() {
    for name in ["metro.js", "min-metro.js"] {
        let source = fixture(name);
        let taken = described(&source, true);
        assert!(taken.is_some(), "{name}");
        assert_eq!(taken, described(&source, false), "{name}");
    }
}

#[test]
fn a_module_id_that_names_the_runtime_refuses() {
    let source = |id: &str| {
        format!(
            "{LOADER}{}{}__r(0);\n",
            def(id, "[]", "module.exports = { value: 2 };"),
            def(
                "0",
                &format!("[{id}]"),
                "console.log(require(_dependencyMap[0]).value);"
            )
        )
    };
    assert!(modules(&source("\"leaf\"")).is_some());
    assert!(modules(&source("\"_runtime/metro\"")).is_none());
}
