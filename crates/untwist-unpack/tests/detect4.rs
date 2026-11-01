mod common;

use common::PrintedModule;
use common::{first_expression_statement, fixture, module_summary, prepared};
use oxc_allocator::Allocator;
use untwist_unpack::model::BundleKind;
use untwist_unpack::webpack::detect4::detect_webpack4;

#[test]
fn unpacks_the_array_container_fixture() {
    let source = fixture("webpack-4.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack4(&alloc, &program, first_expression_statement(&program))
        .expect("webpack 4 bundle");
    assert_eq!(bundle.kind, BundleKind::Webpack);
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    assert_eq!(
        module_summary(&bundle),
        vec![
            ("1".to_string(), "1.js".to_string(), false),
            ("2".to_string(), "index.js".to_string(), true),
            ("3".to_string(), "3.js".to_string(), false),
        ]
    );
    assert_eq!(
        bundle.modules["1"].code().trim_end(),
        "const a = require(3);"
    );
    assert_eq!(bundle.modules["3"].code().trim_end(), "module.exports = 4;");
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const a = require(1);\nconst _module = 1;\nmodule.exports.color = \"#FBC02D\";\n{\n  const _module2 = 2;\n  console.log(_module2);\n  console.log(module);\n}\nexports.a = 3;"
    );
}

#[test]
fn unpacks_the_object_container_fixture() {
    let source = fixture("webpack-object.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack4(&alloc, &program, first_expression_statement(&program))
        .expect("webpack 4 bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("386"));
    assert_eq!(
        module_summary(&bundle),
        vec![
            ("386".to_string(), "index.js".to_string(), true),
            ("387".to_string(), "387.js".to_string(), false),
        ]
    );
    assert_eq!(
        bundle.modules["386"].code().trim_end(),
        "const r = require(387).default;"
    );
}

#[test]
fn finds_the_returned_entry_id_of_a_webpack_0_11_bundle() {
    let source = fixture("webpack-0.11.x.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack4(&alloc, &program, first_expression_statement(&program))
        .expect("webpack 0.11.x bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("0"));
    assert_eq!(
        bundle.modules.keys().cloned().collect::<Vec<_>>(),
        vec!["0", "1", "2", "3", "4", "5", "6"]
    );
    assert!(bundle.modules["0"].code().starts_with("require(1);"));
    assert!(
        !bundle.modules["0"].code().contains("/***/"),
        "{}",
        bundle.modules["0"].code()
    );
    assert!(bundle.modules["2"].code().contains("MIT License"));
}

#[test]
fn a_bundle_with_a_traversing_module_id_still_unpacks() {
    let source = fixture("webpack-path-traversal.js");
    let alloc = Allocator::default();
    let program = prepared(&alloc, &source);
    let bundle = detect_webpack4(&alloc, &program, first_expression_statement(&program))
        .expect("webpack 4 bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("386"));
    assert_eq!(
        module_summary(&bundle),
        vec![(
            "../tmp.js".to_string(),
            "_unsafe____tmp_js.js".to_string(),
            false
        )]
    );
}

#[test]
fn non_bundles_are_rejected() {
    let alloc = Allocator::default();
    for source in [
        "(function (m) { m[0](); })([]);",
        "(function (m) { function r(i) { m[i](); } })();",
        "((m) => { function r(i) { m[i](); } })([function () {}]);",
        "(function (m, n) { function r(i) { m[i](); } })([function () {}]);",
    ] {
        let program = common::parse(&alloc, source);
        let expr = first_expression_statement(&program);
        assert!(
            detect_webpack4(&alloc, &program, expr).is_none(),
            "{source}"
        );
    }
}
