mod common;

use common::PrintedModule;
use common::{first_expression_statement, fixture, parse, prepared};
use oxc_allocator::Allocator;
use untwist_unpack::browserify::detect::detect_browserify;
use untwist_unpack::model::BundleKind;

#[test]
fn unpacks_the_classic_prelude() {
    let source = fixture("browserify.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_browserify(&alloc, &program, first_expression_statement(&program))
        .expect("browserify bundle");
    assert_eq!(bundle.kind, BundleKind::Browserify);
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        bundle.modules.keys().cloned().collect::<Vec<_>>(),
        vec!["1", "2", "3", "4"]
    );
    assert!(bundle.modules["2"].is_entry);
    assert_eq!(
        bundle.modules["2"].dependencies,
        vec![("4".to_string(), "./sum".to_string())]
    );
    assert_eq!(
        bundle.modules["4"].dependencies,
        vec![
            ("1".to_string(), "./add".to_string()),
            ("3".to_string(), "./reduce".to_string())
        ]
    );
    assert_eq!(
        bundle.modules["1"].code().trim_end(),
        "function add(a, b) {\n  return a + b;\n}\nmodule.exports = add;"
    );
}

#[test]
fn unpacks_the_curried_prelude_and_skips_externals() {
    let source = fixture("browserify-2.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_browserify(&alloc, &program, first_expression_statement(&program))
        .expect("browserify bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        bundle.modules["2"].dependencies,
        vec![("1".to_string(), "./lib".to_string())]
    );
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const vscode = require(\"vscode\");\nconst lib = require(\"./lib\");\nconsole.log(lib);"
    );
}

#[test]
fn only_the_first_entry_id_is_used() {
    let source = fixture("browserify-cocos2d.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let expr = first_expression_statement(&program);
    let Some(bundle) = browserify_from_assignment(&alloc, &program, expr) else {
        panic!("browserify bundle");
    };
    assert_eq!(bundle.entry_id.as_deref(), Some("BattleHint"));
    assert!(bundle.modules["BattleHint"].is_entry);
    assert!(!bundle.modules["GChangeCount"].is_entry);
}

fn browserify_from_assignment<'a>(
    alloc: &'a Allocator,
    program: &oxc_ast::ast::Program<'a>,
    expr: &oxc_ast::ast::Expression<'a>,
) -> Option<untwist_unpack::model::Bundle<'a>> {
    let oxc_ast::ast::Expression::AssignmentExpression(assignment) = expr else {
        return None;
    };
    detect_browserify(alloc, program, &assignment.right)
}

#[test]
fn non_browserify_calls_are_rejected() {
    let alloc = Allocator::default();
    for source in [
        "(function (t, n) { x(); })({ 1: [function () {}, {}] }, {}, [1]);",
        "(function (t, n, r) { x(); })({ 1: [function () {}] }, {}, [1]);",
        "(function (t, n, r) { x(); })({ 1: [function () {}, {}] }, [], [1]);",
        "(function (t, n, r) { x(); })({ 1: [function () {}, {}] }, {}, []);",
        "(function (t, n, r) { x(); })({}, {}, [1]);",
        "(function (t, n, r) { x(); })({ 1: [function () {}, { './a': {} }] }, {}, [1]);",
    ] {
        let program = parse(&alloc, source);
        let expr = first_expression_statement(&program);
        assert!(
            detect_browserify(&alloc, &program, expr).is_none(),
            "{source}"
        );
    }
}

fn paths(bundle: &untwist_unpack::model::Bundle<'_>) -> Vec<(String, String)> {
    bundle
        .modules
        .values()
        .map(|module| (module.id.clone(), module.path.as_str().to_string()))
        .collect()
}

#[test]
fn dependency_requests_decide_the_file_names() {
    let source = fixture("browserify.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_browserify(&alloc, &program, first_expression_statement(&program))
        .expect("browserify bundle");
    assert_eq!(
        paths(&bundle),
        vec![
            ("1".to_string(), "add.js".to_string()),
            ("2".to_string(), "index.js".to_string()),
            ("3".to_string(), "reduce.js".to_string()),
            ("4".to_string(), "sum.js".to_string()),
        ]
    );
}

#[test]
fn an_external_only_module_keeps_its_request_name() {
    let source = fixture("browserify-2.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_browserify(&alloc, &program, first_expression_statement(&program))
        .expect("browserify bundle");
    assert_eq!(
        paths(&bundle),
        vec![
            ("1".to_string(), "lib.js".to_string()),
            ("2".to_string(), "index.js".to_string()),
        ]
    );
}

const UNSAFE_ID_SOURCE: &str = r#"(function e(t, n, r) {
  function s(o, u) {
    if (!n[o]) {
      if (!t[o]) {
        var a = typeof require == 'function' && require;
        if (!u && a) return a(o, !0);
        if (i) return i(o, !0);
        var f = new Error("Cannot find module '" + o + "'");
        throw ((f.code = 'MODULE_NOT_FOUND'), f);
      }
      var l = (n[o] = { exports: {} });
      t[o][0].call(
        l.exports,
        function (e) {
          var n = t[o][1][e];
          return s(n ? n : e);
        },
        l,
        l.exports,
        e,
        t,
        n,
        r
      );
    }
    return n[o].exports;
  }
  var i = typeof require == 'function' && require;
  for (var o = 0; o < r.length; o++) s(r[o]);
  return s;
})(
  {
    "../evil": [
      function (require, module, exports) {
        module.exports = 1;
      },
      {},
    ],
    1: [
      function (require, module, exports) {
        var helper = require('./safe');
        console.log(helper);
      },
      { './safe': '../evil' },
    ],
  },
  {},
  [1]
);
"#;

#[test]
fn a_resolved_path_clears_a_stale_unsafe_path_source() {
    let alloc = Allocator::default();
    let program = parse(&alloc, UNSAFE_ID_SOURCE);
    let bundle = detect_browserify(&alloc, &program, first_expression_statement(&program))
        .expect("browserify bundle");
    assert_eq!(bundle.modules["../evil"].path.as_str(), "safe.js");
    assert_eq!(bundle.modules["../evil"].unsafe_path_source, None);
}

#[test]
fn modules_unreachable_from_the_entry_keep_id_derived_paths() {
    let source = fixture("browserify-cocos2d.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let expr = first_expression_statement(&program);
    let bundle = browserify_from_assignment(&alloc, &program, expr).expect("browserify bundle");
    assert_eq!(
        paths(&bundle),
        vec![
            ("BattleHint".to_string(), "index.js".to_string()),
            ("GChangeCount".to_string(), "GChangeCount.js".to_string()),
            (
                "pako_deflate.min".to_string(),
                "pako_deflate.min.js".to_string()
            ),
        ]
    );
}
