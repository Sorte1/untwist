mod common;

use common::PrintedModule;
use common::{fixture, module_summary, parse, prepared};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_unpack::model::{Bundle, BundleKind};
use untwist_unpack::webpack::detect5::{detect_webpack5, detect_webpack5_with};

fn bundle_from_wrapper<'a>(alloc: &'a Allocator, program: &Program<'a>) -> Option<Bundle<'a>> {
    let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
        return None;
    };
    let expr = untwist_unpack::matcher::unwrap_statement_expression(&stmt.expression);
    let factory = untwist_unpack::matcher::call_parts(expr)
        .and_then(|(callee, _)| untwist_unpack::matcher::anonymous_factory(callee))?;
    detect_webpack5(alloc, program, &factory.body.statements)
}

fn conservative_bundle_from_wrapper<'a>(
    alloc: &'a Allocator,
    program: &Program<'a>,
) -> Option<Bundle<'a>> {
    let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
        return None;
    };
    let expr = untwist_unpack::matcher::unwrap_statement_expression(&stmt.expression);
    let factory = untwist_unpack::matcher::call_parts(expr)
        .and_then(|(callee, _)| untwist_unpack::matcher::anonymous_factory(callee))?;
    let conservative = untwist_unpack::UnpackOptions {
        conservative: true,
        ..untwist_unpack::UnpackOptions::default()
    };
    detect_webpack5_with(
        alloc,
        program,
        &factory.body.statements,
        &mut untwist_unpack::model::Bodies::Cloned,
        conservative,
    )
}

#[test]
fn unpacks_the_canonical_webpack_5_bundle() {
    let source = fixture("webpack-5.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = bundle_from_wrapper(&alloc, &program).expect("webpack 5 bundle");
    assert_eq!(bundle.kind, BundleKind::Webpack);
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        module_summary(&bundle),
        vec![
            ("2".to_string(), "index.js".to_string(), true),
            ("3".to_string(), "3.js".to_string(), false),
            ("4".to_string(), "4.js".to_string(), false),
        ]
    );
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "\"use strict\";\nconst lib = require(3);\nconsole.log(lib);"
    );
}

#[test]
fn unpacks_a_one_parameter_arrow_factory_and_leaves_computed_requires_alone() {
    let source = fixture("webpack-5-object.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = conservative_bundle_from_wrapper(&alloc, &program).expect("webpack 5 bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        bundle.modules["3"].code().trim_end(),
        "module.exports = \"foo\";"
    );
    assert!(bundle.modules["2"]
        .code()
        .contains("require(require.ab + \"build/Release/spdlog.node\")"));
}

#[test]
fn unpacks_an_object_method_factory() {
    let source = fixture("webpack-5-method.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = bundle_from_wrapper(&alloc, &program).expect("webpack 5 bundle");
    assert_eq!(
        module_summary(&bundle),
        vec![("2".to_string(), "index.js".to_string(), true)]
    );
}

#[test]
fn a_runtime_with_no_assigned_entry_has_no_entry() {
    let source = fixture("webpack-5-no-entry.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = bundle_from_wrapper(&alloc, &program).expect("webpack 5 bundle");
    assert_eq!(bundle.entry_id, None);
    assert_eq!(
        module_summary(&bundle),
        vec![("97418".to_string(), "97418.js".to_string(), false)]
    );
}

#[test]
fn a_decoy_container_before_the_real_one_does_not_defeat_detection() {
    let source = "var installedChunks = { 0: function () {} };\nvar __webpack_modules__ = { 2: (module, exports, require) => { x(); } };\nfunction __webpack_require__(id) {\n  __webpack_modules__[id]();\n}\n__webpack_require__(__webpack_require__.s = 2);\n";
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    let bundle = detect_webpack5(&alloc, &program, &program.body).expect("webpack 5 bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        module_summary(&bundle),
        vec![("2".to_string(), "index.js".to_string(), true)]
    );
}

#[test]
fn an_empty_container_is_never_the_modules_container() {
    let source = "var cache = {};\nfunction r(id) {\n  cache[id]();\n}\n";
    let alloc = Allocator::default();
    let program = parse(&alloc, source);
    assert!(detect_webpack5(&alloc, &program, &program.body).is_none());
}
