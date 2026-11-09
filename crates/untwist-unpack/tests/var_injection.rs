mod common;

use common::parse;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::model::Module;
use untwist_unpack::webpack::var_injection::{inline_proven_injection, inline_var_injections};

fn inline(source: &str) -> String {
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let cloned = untwist_core::parse::parse_program(&alloc, source).program;
    let _ = program;
    let mut module = Module::new("0".to_string(), cloned, false);
    inline_var_injections(&alloc, &mut module);
    module.code().trim_end().to_string()
}

fn printed(source: &str) -> String {
    let alloc = Allocator::default();
    let program = untwist_core::parse::parse_program(&alloc, source).program;
    let module = Module::new("0".to_string(), program, false);
    module.code().trim_end().to_string()
}

#[test]
fn splices_injected_variables_and_the_body() {
    let source = "require.r(exports);\n(function (m, n) {\n  console.log(m, n);\n}).call(this, require(1), require(2));\n";
    assert_eq!(
        inline(source),
        "require.r(exports);\nvar m = require(1);\nvar n = require(2);\nconsole.log(m, n);"
    );
}

#[test]
fn accepts_exports_as_the_this_argument() {
    let source = "(function (global) {\n  global.x = 1;\n}).call(exports, window);\n";
    assert_eq!(inline(source), "var global = window;\nglobal.x = 1;");
}

#[test]
fn a_missing_argument_yields_an_uninitialised_var() {
    let source = "(function (a, b) {\n  a();\n}).call(this, 1);\n";
    assert_eq!(inline(source), "var a = 1;\nvar b;\na();");
}

#[test]
fn unrelated_statements_are_untouched() {
    for source in [
        "(function (m) {\n  m();\n}).call(other, 1);\n",
        "(function (m) {\n  m();\n}).call(this);\n",
        "(function ({ m }) {\n  m();\n}).call(this, 1);\n",
        "(function (m) {\n  m();\n}).apply(this, [1]);\n",
    ] {
        let expected = printed(source);
        assert_eq!(inline(source), expected);
    }
}

#[test]
fn nested_injections_are_not_touched() {
    let source = "function f() {\n  (function (m) {\n    m();\n  }).call(this, 1);\n}\n";
    assert!(inline(source).contains(".call(this, 1)"));
}

#[test]
fn a_spread_argument_after_the_receiver_is_left_untouched() {
    let source = "(function (m) {\n  m();\n}).call(this, ...x);\n";
    assert_eq!(inline(source), printed(source));
}

#[test]
fn a_spread_among_later_arguments_is_left_untouched() {
    let source = "(function (a, b, c) {}).call(this, 1, ...y, 3);\n";
    assert_eq!(inline(source), printed(source));
}

#[test]
fn extra_arguments_beyond_the_parameter_count_are_ignored() {
    let source = "(function (a) {}).call(this, 1, 2);\n";
    assert_eq!(inline(source), "var a = 1;");
}

#[test]
fn a_computed_call_property_is_accepted() {
    let source = "(function (m) {})[\"call\"](this, 1);\n";
    assert_eq!(inline(source), "var m = 1;");
}

#[test]
fn a_named_function_expression_callee_is_accepted() {
    let source = "(function foo(m) {}).call(this, 1);\n";
    assert_eq!(inline(source), "var m = 1;");
}

fn proven(source: &str) -> String {
    let alloc = Allocator::default();
    let program = untwist_core::parse::parse_program(&alloc, source).program;
    let mut module = Module::new("0".to_string(), program, false);
    inline_proven_injection(&alloc, &mut module);
    module.code().trim_end().to_string()
}

#[test]
fn the_wrapper_around_a_whole_module_is_spliced() {
    let source = "require.r(exports);\n(function (global, tag) {\n  exports.default = tag + typeof global;\n}).call(this, require(2), \"t\");\n";
    assert_eq!(
        proven(source),
        "require.r(exports);\nvar global = require(2);\nvar tag = \"t\";\nexports.default = tag + typeof global;"
    );
}

#[test]
fn a_wrapper_called_on_exports_is_spliced_when_the_body_never_reads_this() {
    let source = "(function (__dirname) {\n  exports.dir = __dirname;\n}).call(exports, \"/\");\n";
    assert_eq!(
        proven(source),
        "var __dirname = \"/\";\nexports.dir = __dirname;"
    );
}

#[test]
fn a_nested_function_may_return_and_read_its_own_arguments() {
    let source = "(function (a) {\n  exports.f = function () {\n    return arguments.length + this.n + a;\n  };\n}).call(exports, 1);\n";
    let spliced = proven(source);
    assert!(
        spliced.starts_with("var a = 1;\nexports.f = function"),
        "{spliced}"
    );
}

#[test]
fn an_arrow_in_the_body_may_return() {
    let source =
        "(function (a) {\n  exports.n = (() => {\n    return a;\n  })();\n}).call(this, 1);\n";
    let spliced = proven(source);
    assert!(spliced.starts_with("var a = 1;\nexports.n = "), "{spliced}");
}

#[test]
fn a_wrapper_beside_other_statements_stays_a_call() {
    for source in [
        "var x = 1;\n(function (a) {\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  exports.a = a;\n}).call(this, 1);\nexports.b = 2;\n",
        "require.d(exports, \"a\", function () {});\n(function (a) {\n  exports.a = a;\n}).call(this, 1);\n",
    ] {
        assert_eq!(proven(source), printed(source), "{source}");
    }
}

#[test]
fn a_body_that_needs_its_own_function_stays_a_call() {
    for source in [
        "(function (a) {\n  if (a) return;\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  exports.n = arguments.length;\n}).call(this, 1);\n",
        "(function (a) {\n  exports.n = (() => arguments.length)();\n}).call(this, 1);\n",
        "(function (a) {\n  \"use strict\";\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  exports.a = eval(\"a\");\n}).call(this, 1);\n",
        "(function (a) {\n  exports.a = new.target;\n}).call(this, 1);\n",
        "(function named(a) {\n  exports.a = named;\n}).call(this, 1);\n",
        "(function (a) {\n  this.a = a;\n}).call(exports, 1);\n",
        "(function (a) {\n  (() => this.a = a)();\n}).call(exports, 1);\n",
    ] {
        assert_eq!(proven(source), printed(source), "{source}");
    }
}

#[test]
fn arguments_that_do_not_match_the_parameters_stay_a_call() {
    for source in [
        "(function (a) {\n  exports.a = a;\n}).call(this, 1, effect());\n",
        "(function (a, b) {\n  exports.a = b;\n}).call(this, 1);\n",
        "(function (a = 2) {\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a, a) {\n  exports.a = a;\n}).call(this, 1, 2);\n",
        "(function (exports) {\n  exports.a = 1;\n}).call(this, {});\n",
        "(function (require) {\n  require(1);\n}).call(this, f);\n",
        "(function (module) {\n  module.exports = 1;\n}).call(this, require(3)(module));\n",
    ] {
        assert_eq!(proven(source), printed(source), "{source}");
    }
}

#[test]
fn a_name_the_splice_would_capture_stays_a_call() {
    for source in [
        "(function (a) {\n  function a() {}\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  {\n    function a() {}\n  }\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  var exports = {};\n  exports.a = a;\n}).call(this, 1);\n",
        "(function (a) {\n  var module = a;\n}).call(this, 1);\n",
        "(function (a) {\n  function require() {}\n}).call(this, 1);\n",
        "(function (a) {\n  var late = 1;\n  exports.a = a;\n}).call(this, typeof late);\n",
        "(function (a, b) {\n  exports.a = a + b;\n}).call(this, 1, typeof a);\n",
        "(function (global) {\n  exports.g = global;\n}).call(this, typeof global);\n",
    ] {
        assert_eq!(proven(source), printed(source), "{source}");
    }
}
