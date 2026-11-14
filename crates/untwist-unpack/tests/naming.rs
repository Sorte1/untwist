mod common;

use common::prepared;
use oxc_allocator::Allocator;
use untwist_unpack::naming::library::parse as library_comment;
use untwist_unpack::naming::stem::meaningful;
use untwist_unpack::unpack;

const HEAD: &str = "!(function (e) {\n  var a = {};\n  function n(t) {\n    if (a[t]) return a[t].exports;\n    var r = (a[t] = { exports: {} });\n    return e[t].call(r.exports, r, r.exports, n), r.exports;\n  }\n  return n((n.s = 0));\n})([\n";

fn webpack(modules: &[&str]) -> String {
    let bodies: Vec<String> = modules
        .iter()
        .map(|body| format!("  function (e, t, i) {{\n{body}\n  }}"))
        .collect();
    format!("{HEAD}{}\n]);\n", bodies.join(",\n"))
}

fn paths(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let program = prepared(&alloc, source);
    let result = unpack(&alloc, &program, &[]).expect("bundle");
    result
        .bundle
        .modules
        .values()
        .map(|module| module.path.as_str().to_string())
        .collect()
}

#[test]
fn generated_and_minified_identifiers_are_not_names() {
    for name in [
        "fn236",
        "Ctor14",
        "cls249",
        "component5",
        "set_v369",
        "a2",
        "aA",
        "iìíîîiï",
        "_0x1a2b",
        "requireModule12",
        "hasRequiredModule3",
        "obj75",
        "e",
        "x1y",
    ] {
        assert!(!meaningful(name), "{name}");
    }
    for name in [
        "Vector3",
        "MathUtils",
        "generateUUID",
        "uid",
        "Box",
        "inherits",
    ] {
        assert!(meaningful(name), "{name}");
    }
}

#[test]
fn a_library_comment_gives_the_package_path() {
    assert_eq!(
        library_comment("// core-js 3.39: internals/get-built-in.js").as_deref(),
        Some("core-js/internals/get-built-in.js")
    );
    assert_eq!(
        library_comment(
            "// @babel/runtime 7.0–7.11: helpers/inheritsLoose.js, same code in @swc/helpers 0.2–0.5: inherits_loose.js"
        )
        .as_deref(),
        Some("@babel/runtime/helpers/inheritsLoose.js")
    );
    assert_eq!(
        library_comment("// tslib 1.11–2.9: tslib.js | tslib.es6.js"),
        None
    );
    assert_eq!(library_comment("// see: notes.js"), None);
    assert_eq!(library_comment("// core-js 3.39: ../escape.js"), None);
}

#[test]
fn a_module_takes_the_class_it_exports() {
    let source = webpack(&[
        "    const x = i(1);\n    console.log(x);",
        "    class Vector3 {}\n    e.exports = Vector3;",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "Vector3.js"]);
}

#[test]
fn a_module_takes_the_name_its_importers_give_it() {
    let source = webpack(&[
        "    const inherits = i(1);\n    console.log(inherits);",
        "    e.exports = function (a, b) { a.prototype = b; };",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "inherits.js"]);
}

#[test]
fn a_used_helper_never_names_the_module() {
    let source = webpack(&[
        "    const v = i(1);\n    console.log(v);",
        "    function clamp(a) { return a; }\n    e.exports = clamp(4);",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "1.js"]);
}

#[test]
fn a_generic_name_keeps_the_module_id() {
    let source = webpack(&[
        "    const fn2 = i(1);\n    console.log(fn2);",
        "    function Ctor() {}\n    e.exports = Ctor;",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "1.js"]);
}

#[test]
fn a_library_comment_on_most_of_the_module_names_it_after_the_library() {
    let source = webpack(&[
        "    const v = i(1);\n    console.log(v);",
        "    // core-js 3.39: internals/is-callable.js\n    e.exports = function (it) { return typeof it == 'function'; };",
    ]);
    assert_eq!(
        paths(&source),
        vec!["index.js", "node_modules/core-js/internals/is-callable.js"]
    );
}

#[test]
fn an_inlined_library_helper_does_not_name_its_host() {
    let source = webpack(&[
        "    const v = i(1);\n    console.log(v);",
        "    // @babel/runtime 7.0–7.11: helpers/inheritsLoose.js\n    function h(a, b) { a.prototype = b; }\n    var codes = {};\n    function make(code, message) { codes[code] = function () { return message + code + message + code; }; }\n    make('ERR_ONE', 'first failure message'); make('ERR_TWO', 'second failure message'); make('ERR_THREE', 'third failure message');",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "1.js"]);
}

#[test]
fn a_webpack_import_binding_gives_the_source_file_stem() {
    let source = webpack(&[
        "    var _shared_js__WEBPACK_IMPORTED_MODULE_0__ = i(1);\n    console.log(_shared_js__WEBPACK_IMPORTED_MODULE_0__);",
        "    e.exports = 2;",
    ]);
    assert_eq!(paths(&source), vec!["index.js", "shared.js"]);
}
