mod common;

use common::PrintedModule;
use common::{bundle_from, fixture, module_summary};
use oxc_allocator::Allocator;
use untwist_unpack::model::BundleKind;

fn detect(name: &str) -> (BundleKind, Option<String>, Vec<(String, String, bool)>) {
    let source = fixture(name);
    let alloc = Allocator::default();
    let bundle = bundle_from(&alloc, &source).unwrap_or_else(|| panic!("no bundle in {name}"));
    (
        bundle.kind,
        bundle.entry_id.clone(),
        module_summary(&bundle),
    )
}

#[test]
fn every_webpack_shape_is_found_through_its_wrapper() {
    for (name, entry) in [
        ("webpack-4.js", Some("2")),
        ("webpack-object.js", Some("386")),
        ("webpack-0.11.x.js", Some("0")),
        ("webpack-esm.js", Some("0")),
        ("webpack-var-injection.js", Some("0")),
        ("webpack-5.js", Some("2")),
        ("webpack-5-method.js", Some("2")),
        ("webpack-5-no-entry.js", None),
        ("webpack-jsonp-chunk.js", None),
        ("webpack-5-json.js", None),
        ("webpack-chunk-no-entries.js", None),
    ] {
        let (kind, entry_id, modules) = detect(name);
        assert_eq!(kind, BundleKind::Webpack, "{name}");
        assert_eq!(entry_id.as_deref(), entry, "{name}");
        assert!(!modules.is_empty(), "{name}");
    }
}

#[test]
fn a_guarded_bundle_directory_helper_moves_to_the_runtime_module() {
    let source = fixture("webpack-5-object.js");
    let alloc = Allocator::default();
    let bundle = bundle_from(&alloc, &source).expect("a bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
    let program = common::prepared(&alloc, &source);
    let conservative = untwist_unpack::UnpackOptions {
        conservative: true,
        ..untwist_unpack::UnpackOptions::default()
    };
    let bundle =
        untwist_unpack::unpack_program_with(&alloc, &program, conservative).expect("a bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
}

#[test]
fn browserify_bundles_are_found() {
    for name in ["browserify.js", "browserify-2.js", "browserify-cocos2d.js"] {
        let (kind, entry_id, modules) = detect(name);
        assert_eq!(kind, BundleKind::Browserify, "{name}");
        assert!(entry_id.is_some(), "{name}");
        assert!(!modules.is_empty(), "{name}");
    }
}

#[test]
fn the_outermost_bundle_wins() {
    let (kind, entry_id, modules) = detect("browserify-webpack-nested.js");
    assert_eq!(kind, BundleKind::Browserify);
    assert_eq!(entry_id.as_deref(), Some("2"));
    assert_eq!(
        modules
            .iter()
            .map(|(id, _, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "2"]
    );
    let source = fixture("browserify-webpack-nested.js");
    let alloc = Allocator::default();
    let bundle = bundle_from(&alloc, &source).expect("bundle");
    assert!(
        bundle.modules["1"].code().contains("__webpack")
            || bundle.modules["1"].code().contains("function n(t)"),
        "the nested webpack bundle stays inline: {}",
        bundle.modules["1"].code()
    );
}

#[test]
fn an_unwrapped_top_level_webpack_5_bundle_is_found() {
    let source = "var __webpack_modules__ = { 2: (module, exports, require) => {\n  x();\n} };\nfunction __webpack_require__(id) {\n  __webpack_modules__[id]();\n}\n__webpack_require__(__webpack_require__.s = 2);\n".to_string();
    let alloc = Allocator::default();
    let bundle = bundle_from(&alloc, &source).expect("top-level webpack 5 bundle");
    assert_eq!(bundle.entry_id.as_deref(), Some("2"));
}

#[test]
fn a_program_with_no_bundle_yields_none() {
    let source = "console.log(1);\nfunction f() {\n  return 2;\n}\n".to_string();
    let alloc = Allocator::default();
    assert!(bundle_from(&alloc, &source).is_none());
}
