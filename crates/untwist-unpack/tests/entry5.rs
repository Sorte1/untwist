mod common;

use common::{parse, prepared};
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use untwist_core::print::{print, PrintStyle};
use untwist_unpack::{finish_with, unpack_program_taking_with, unpack_program_with, UnpackOptions};

const FACTORIES: &str = "  var __webpack_modules__ = {\n    1: (__unused, __webpack_exports__, __webpack_require__) => {\n      \"use strict\";\n      __webpack_require__.d(__webpack_exports__, { a: () => a });\n      const a = 1;\n    },\n  };\n";
const REQUIRE: &str = "  var __webpack_module_cache__ = {};\n  function __webpack_require__(moduleId) {\n    var cached = __webpack_module_cache__[moduleId];\n    if (cached !== undefined) return cached.exports;\n    var module = (__webpack_module_cache__[moduleId] = { exports: {} });\n    __webpack_modules__[moduleId](module, module.exports, __webpack_require__);\n    return module.exports;\n  }\n";
const HELPERS: &str = "  (() => { __webpack_require__.d = (exports, definition) => { for (var key in definition) { if (__webpack_require__.o(definition, key) && !__webpack_require__.o(exports, key)) Object.defineProperty(exports, key, { enumerable: true, get: definition[key] }); } }; })();\n  (() => { __webpack_require__.o = (obj, prop) => Object.prototype.hasOwnProperty.call(obj, prop); })();\n  (() => { __webpack_require__.r = (exports) => { Object.defineProperty(exports, \"__esModule\", { value: true }); }; })();\n";
const ENTRY: &str = "  var __webpack_exports__ = {};\n  (() => {\n    \"use strict\";\n    __webpack_require__.r(__webpack_exports__);\n    var m = __webpack_require__(1);\n    console.log(m.a);\n  })();\n";

fn bundle_with(prelude: &str, entry: &str) -> String {
    format!("(() => {{\n{FACTORIES}{REQUIRE}{HELPERS}{prelude}{entry}}})();\n")
}

fn entry_body(body: &str) -> String {
    format!("  var __webpack_exports__ = {{}};\n  (() => {{\n    \"use strict\";\n    {body}\n  }})();\n")
}

#[derive(Debug, PartialEq)]
struct Out {
    entry: Option<String>,
    modules: Vec<(String, String)>,
}

type Parser = for<'a> fn(&'a Allocator, &'a str) -> Program<'a>;

fn extracted(text: &str, options: UnpackOptions, (parser, taking): (Parser, bool)) -> Option<Out> {
    let alloc = Allocator::default();
    let mut program = parser(&alloc, text);
    let mut bundle = match taking {
        true => unpack_program_taking_with(&alloc, &mut program, options),
        false => unpack_program_with(&alloc, &program, options),
    }?;
    finish_with(&alloc, &mut bundle, &[], options);
    Some(Out {
        entry: bundle.entry_id.clone(),
        modules: bundle
            .modules
            .values()
            .map(|module| {
                let code = print(module.program(), PrintStyle::Pretty);
                (
                    module.path.as_str().to_string(),
                    code.trim_end().to_string(),
                )
            })
            .collect(),
    })
}

fn unpacked_by(text: &str, options: UnpackOptions, parser: Parser) -> Out {
    extracted(text, options, (parser, false)).expect("a webpack bundle")
}

fn unpacked_with(text: &str, options: UnpackOptions) -> Out {
    unpacked_by(text, options, parse)
}

fn unpacked(text: &str) -> Out {
    unpacked_with(text, UnpackOptions::default())
}

fn index(out: &Out) -> Option<&str> {
    out.modules
        .iter()
        .find(|(path, _)| path == "index.js")
        .map(|(_, code)| code.as_str())
}

fn refused(text: &str) {
    let out = unpacked(text);
    assert_eq!(out.entry, None, "{text}");
    assert!(index(&out).is_none(), "{text}");
}

fn refused_body(body: &str) {
    refused(&bundle_with("", &entry_body(body)));
}

fn taken_matches_cloned_by(text: &str, parser: Parser) -> Option<Out> {
    let options = UnpackOptions::default();
    let cloned = extracted(text, options, (parser, false));
    let taken = extracted(text, options, (parser, true));
    assert_eq!(taken, cloned, "{text}");
    cloned
}

fn taken_matches_cloned(text: &str) -> Option<Out> {
    taken_matches_cloned_by(text, parse)
}

#[test]
fn the_strict_entry_iife_becomes_the_index_module() {
    let out = unpacked(&bundle_with("", ENTRY));
    assert_eq!(out.entry.as_deref(), Some("entry"));
    assert_eq!(out.modules[0].0, "index.js");
    let code = index(&out).expect("an index module");
    assert!(code.starts_with("\"use strict\";"), "{code}");
    assert!(code.contains("var m = require(\"./1.js\");"), "{code}");
    assert!(code.contains("export {};"), "{code}");
    assert!(!code.contains("__webpack_exports__"), "{code}");
    assert!(!code.contains("__webpack_require__.r"), "{code}");
}

#[test]
fn an_entry_that_stays_commonjs_routes_its_helpers() {
    let out = unpacked(&bundle_with(
        "",
        &entry_body(
            "var ns = {}; __webpack_require__.r(ns); console.log(__webpack_require__(1).a, ns);",
        ),
    ));
    let code = index(&out).expect("an index module");
    assert!(
        code.contains("const __webpack_require__ = require(\"./_runtime/webpack.js\");"),
        "{code}"
    );
    assert!(code.contains("__webpack_require__.r(ns);"), "{code}");
    assert!(code.contains("require(\"./1.js\").a"), "{code}");
}

#[test]
fn the_minified_entry_in_a_helper_sequence_is_written() {
    let text = "(()=>{var e={1:(e,t,r)=>{\"use strict\";r.d(t,{a:()=>o});const o=1}};var t={};function r(o){var n=t[o];if(void 0!==n)return n.exports;var u=t[o]={exports:{}};return e[o](u,u.exports,r),u.exports}r.d=(e,t)=>{for(var o in t)r.o(t,o)&&!r.o(e,o)&&Object.defineProperty(e,o,{enumerable:!0,get:t[o]})},r.o=(e,t)=>Object.prototype.hasOwnProperty.call(e,t),(()=>{\"use strict\";var e=r(1);console.log(e.a)})()})();";
    let out = unpacked_by(text, UnpackOptions::default(), prepared);
    assert_eq!(out.entry.as_deref(), Some("entry"));
    let code = index(&out).expect("an index module");
    assert!(code.contains("require(\"./1.js\")"), "{code}");
    assert!(taken_matches_cloned_by(text, prepared).is_some());
}

#[test]
fn the_entry_id_avoids_module_ids_and_string_literals() {
    let text = bundle_with("", ENTRY).replace(
        "    1: (",
        "    entry: (__unused) => { console.log(\"entry2\"); },\n    1: (",
    );
    let out = unpacked(&text);
    assert_eq!(out.entry.as_deref(), Some("entry3"));
    assert_eq!(out.modules[0].0, "index.js");
}

#[test]
fn taking_matches_cloning_for_written_and_refused_entries() {
    let written = taken_matches_cloned(&bundle_with("", ENTRY)).expect("a bundle");
    assert!(index(&written).is_some());
    let refused = taken_matches_cloned(&bundle_with(
        "",
        &entry_body("console.log(__webpack_module_cache__);"),
    ))
    .expect("a bundle");
    assert!(index(&refused).is_none());
}

#[test]
fn entry_locals_do_not_pin_a_helper_reading_the_same_global() {
    let text = bundle_with(
        "  (() => { __webpack_require__.g = () => typeof counter; })();\n",
        &entry_body("let counter = 1; console.log(__webpack_require__.g(), counter, __webpack_require__(1).a);"),
    );
    let out = taken_matches_cloned(&text).expect("a bundle");
    let code = index(&out).expect("an index module");
    assert!(code.contains("__webpack_require__.g()"), "{code}");
}

const SEQUENCE: &str = "(()=>{var e={1:(e,t,r)=>{\"use strict\";r.d(t,{a:()=>o});const o=1},2:(e,t,r)=>{e.exports=r.k()}};var t={};function r(o){var n=t[o];if(void 0!==n)return n.exports;var u=t[o]={exports:{}};e[o](u,u.exports,r);return u.exports}var l;l=1,r.k=function(){return l},r.d=(e,t)=>{for(var o in t)r.o(t,o)&&!r.o(e,o)&&Object.defineProperty(e,o,{enumerable:!0,get:t[o]})},r.o=(e,t)=>Object.prototype.hasOwnProperty.call(e,t),(()=>{\"use strict\";console.log(ENTRY)})()})();";

#[test]
fn the_entry_in_a_helper_sequence_is_written() {
    let text = SEQUENCE.replace("ENTRY", "r(1).a,r(2)");
    let out = taken_matches_cloned(&text).expect("a bundle");
    let code = index(&out).expect("an index module");
    assert!(code.contains("require(\"./1.js\").a"), "{code}");
}

#[test]
fn a_refused_sequence_entry_reading_a_moved_local_extracts_alike() {
    let text = SEQUENCE.replace("ENTRY", "r(1).a,r(2),l");
    let out = taken_matches_cloned(&text).expect("a bundle");
    assert!(index(&out).is_none());
}

#[test]
fn conservative_keeps_the_entry_unwritten() {
    let options = UnpackOptions {
        conservative: true,
        ..UnpackOptions::default()
    };
    let out = unpacked_with(&bundle_with("", ENTRY), options);
    assert_eq!(out.entry, None);
}

#[test]
fn an_entry_reading_a_prelude_binding_is_refused() {
    refused_body("console.log(__webpack_module_cache__);");
}

#[test]
fn an_entry_reading_top_level_this_is_refused() {
    refused_body("console.log(this);");
}

#[test]
fn this_inside_a_function_of_the_entry_is_kept() {
    let out = unpacked(&bundle_with(
        "",
        &entry_body("function f() { return this; } console.log(__webpack_require__(1).a, f);"),
    ));
    assert!(index(&out).is_some());
}

#[test]
fn an_entry_reading_top_level_arguments_is_refused() {
    refused_body("console.log(arguments);");
}

#[test]
fn an_entry_reading_a_host_local_is_refused() {
    for name in [
        "module",
        "exports",
        "require",
        "__filename",
        "__dirname",
        "eval",
    ] {
        refused_body(&format!("console.log(typeof {name});"));
    }
}

#[test]
fn an_entry_writing_a_runtime_member_is_refused() {
    refused_body("__webpack_require__.d = null;");
}

#[test]
fn an_entry_deleting_a_runtime_member_is_refused() {
    refused_body("delete __webpack_require__.d;");
}

#[test]
fn an_entry_writing_the_runtime_require_is_refused() {
    refused_body("__webpack_require__ = null;");
}

#[test]
fn an_entry_using_the_runtime_require_as_a_value_is_refused() {
    refused_body("console.log(__webpack_require__);");
}

#[test]
fn an_entry_requiring_a_computed_id_is_refused() {
    refused_body("var id = 1; console.log(__webpack_require__(id));");
}

#[test]
fn an_entry_binding_a_computed_id_is_refused() {
    refused_body(
        "var id = 1; Promise.resolve().then(__webpack_require__.bind(__webpack_require__, id));",
    );
}

#[test]
fn a_bound_namespace_load_needs_a_literal_mode() {
    let helper = "  (() => { __webpack_require__.t = function (value, mode) { if (mode & 1) value = this(value); return value; }; })();\n";
    let bound = |mode: &str| {
        entry_body(&format!(
            "var m = 3; Promise.resolve().then(__webpack_require__.t.bind(__webpack_require__, 1, {mode}));"
        ))
    };
    let out = unpacked(&bundle_with(helper, &bound("3")));
    assert!(index(&out).is_some());
    refused(&bundle_with(helper, &bound("m")));
}

#[test]
fn only_the_namespace_helper_may_be_bound_with_a_mode() {
    refused_body("Promise.resolve().then(__webpack_require__.d.bind(__webpack_require__, 1, 3));");
}

#[test]
fn an_entry_require_call_takes_exactly_one_argument() {
    refused_body("console.log(__webpack_require__(1, 2).a);");
}

#[test]
fn an_entry_reading_an_unroutable_member_is_refused() {
    refused_body("console.log(__webpack_require__.zz);");
}

#[test]
fn a_statement_after_the_entry_is_refused() {
    refused(&bundle_with("", &format!("{ENTRY}  console.log(1);\n")));
    refused(&bundle_with("", &format!("{ENTRY}  var after = 1;\n")));
}

#[test]
fn an_expression_after_the_entry_in_its_sequence_is_refused() {
    refused(&bundle_with(
        "",
        "  (() => { \"use strict\"; console.log(__webpack_require__(1).a); })(), console.log(1);\n",
    ));
}

#[test]
fn a_sloppy_entry_in_a_strict_bootstrap_is_refused() {
    let text = bundle_with(
        "",
        "  (() => { console.log(__webpack_require__(1).a); })();\n",
    )
    .replacen("(() => {\n", "(() => {\n  \"use strict\";\n", 1);
    refused(&text);
}

#[test]
fn a_sloppy_entry_in_a_strict_file_is_refused() {
    let text = bundle_with(
        "",
        "  (() => { console.log(__webpack_require__(1).a); })();\n",
    );
    refused(&format!("\"use strict\";\n{text}"));
}

#[test]
fn a_with_statement_in_the_entry_is_refused() {
    refused(&bundle_with(
        "",
        "  (() => { with (Math) { console.log(max(__webpack_require__(1).a)); } })();\n",
    ));
}

#[test]
fn an_exports_object_read_by_the_prelude_is_refused() {
    refused(&bundle_with(
        "  (() => { __webpack_require__.x = () => __webpack_exports__; })();\n",
        ENTRY,
    ));
}

#[test]
fn an_exports_object_read_by_a_factory_without_that_parameter_is_refused() {
    let text = bundle_with("", ENTRY).replace(
        "    1: (",
        "    2: (module) => { module.exports = typeof __webpack_exports__; },\n    1: (",
    );
    refused(&text);
}

#[test]
fn an_exports_object_with_initial_properties_is_refused() {
    refused(&bundle_with(
        "",
        &ENTRY.replace("__webpack_exports__ = {}", "__webpack_exports__ = { a: 1 }"),
    ));
}

#[test]
fn a_second_exports_object_is_refused() {
    let entry = ENTRY.replace(
        "    var m =",
        "    __webpack_require__.r(second);\n    var m =",
    );
    refused(&bundle_with("  var second = {};\n", &entry));
}

#[test]
fn a_bootstrap_outside_the_frame_is_refused() {
    let text = format!(
        "(() => {{\n  function start() {{\n{FACTORIES}{REQUIRE}{HELPERS}{ENTRY}}}\n  start();\n}})();\n"
    );
    refused(&text);
}

#[test]
fn an_entry_with_parameters_arguments_or_async_is_refused() {
    refused(&bundle_with(
        "",
        "  ((x) => { \"use strict\"; console.log(__webpack_require__(1).a, x); })();\n",
    ));
    refused(&bundle_with(
        "",
        "  (() => { \"use strict\"; console.log(__webpack_require__(1).a); })(1);\n",
    ));
    refused(&bundle_with(
        "",
        "  (async () => { \"use strict\"; console.log(__webpack_require__(1).a); })();\n",
    ));
}

#[test]
fn an_entry_reading_an_outer_binding_is_refused() {
    let text = format!(
        "var shared = 1;\n{}",
        bundle_with(
            "",
            &entry_body("console.log(shared, __webpack_require__(1).a);")
        )
    );
    refused(&text);
}

#[test]
fn an_entry_reading_a_wrapper_parameter_is_refused() {
    let entry = entry_body("console.log(g, __webpack_require__(1).a);");
    refused(&format!(
        "(function (g) {{\n{FACTORIES}{REQUIRE}{HELPERS}{entry}}})(1);\n"
    ));
}

#[test]
fn an_eval_in_the_bootstrap_or_file_scope_is_refused() {
    refused(&bundle_with("  eval(\"var made = 1\");\n", ENTRY));
    refused(&format!(
        "eval(\"var made = 1\");\n{}",
        bundle_with("", ENTRY)
    ));
}

#[test]
fn an_entry_reading_a_var_declared_in_a_with_block_is_refused() {
    let entry = entry_body("console.log(hidden, __webpack_require__(1).a);");
    refused(&bundle_with("  with ({}) { var hidden = 1; }\n", &entry));
}

#[test]
fn a_module_declaration_beside_the_bootstrap_is_refused() {
    let entry = entry_body("console.log(z, __webpack_require__(1).a);");
    refused(&format!("export const z = 1;\n{}", bundle_with("", &entry)));
}

#[test]
fn the_webpack5_fixtures_extract_alike_taken_and_cloned() {
    let mut written = 0;
    for (label, text) in untwist_testing::js_files_in("fixtures/unpack-bundlers/webpack") {
        if !label.contains("webpack5-") {
            continue;
        }
        for parser in [parse as Parser, prepared] {
            let out = taken_matches_cloned_by(&text, parser);
            written += usize::from(out.is_some_and(|out| index(&out).is_some()));
        }
    }
    assert!(written >= 5, "{written} entries written");
}
