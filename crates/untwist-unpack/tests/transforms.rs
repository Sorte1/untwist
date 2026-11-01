mod common;

use common::bundle_of;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::mappings::{Mapping, MappingMatcher};
use untwist_unpack::model::BundleKind;
use untwist_unpack::webpack::transforms::{apply_transforms, apply_transforms_with};

#[test]
fn the_chain_runs_in_reference_dependency_order() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "0",
                "require.r(exports);\n(function (m, n) {\n  console.log(m, n);\n}).call(this, require(1), require(2));\n",
                false,
            ),
            ("1", "module.exports = 1;\n", false),
            ("2", "module.exports = 2;\n", false),
        ],
        Some("0"),
    );
    let notes = apply_transforms_with(&alloc, &mut bundle, true);
    assert!(notes.is_empty(), "{notes:?}");
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "import * as m from \"./1.js\";\nimport * as n from \"./2.js\";\nconsole.log(m, n);"
    );
    assert!(bundle.modules["0"].is_esm());
}

#[test]
fn default_require_sees_the_esm_flag_convert_esm_set() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "0",
                "const b = require(1);\nconst g = require.n(b);\nconsole.log(g());\n",
                false,
            ),
            ("1", "require.r(exports);\nmodule.exports = 1;\n", false),
        ],
        Some("0"),
    );
    apply_transforms(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "const b = require(\"./1.js\");\nconst g = b.default;\nconsole.log(g);"
    );
}

#[test]
fn missing_targets_and_unhandled_helpers_become_notes() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[(
            "2wwy",
            "const b = require('w6GO');\nrequire.e(1);\nconst x = require.t(b, 2);\n",
            false,
        )],
        None,
    );
    let messages: Vec<String> = apply_transforms(&alloc, &mut bundle)
        .into_iter()
        .map(|note| note.message)
        .collect();
    assert_eq!(
        messages,
        vec![
            "module 2wwy: require(\"w6GO\") has no module in this bundle".to_string(),
            "module 2wwy: unhandled webpack helper require.e".to_string(),
            "module 2wwy: unhandled webpack helper require.t".to_string(),
        ]
    );
}

#[test]
fn browserify_bundles_are_left_alone() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[(
            "2",
            "var sum = require('./sum');\nconsole.log(sum);\n",
            false,
        )],
        Some("2"),
    );
    bundle.kind = BundleKind::Browserify;
    let notes = apply_transforms(&alloc, &mut bundle);
    assert!(notes.is_empty());
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "var sum = require(\"./sum\");\nconsole.log(sum);"
    );
}

fn locals_with_say_as(id: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/unpack-bundlers/webpack/min-webpack5-locals.js");
    std::fs::read_to_string(path)
        .expect("read fixture")
        .replacen("2:(e,t,r)=>", &format!("\"{id}\":(e,t,r)=>"), 1)
        .replacen("r(2)", &format!("r(\"{id}\")"), 1)
}

#[test]
fn a_module_mapped_to_the_runtime_path_is_not_overwritten_by_the_runtime() {
    let source = locals_with_say_as("2");
    let alloc = Allocator::default();
    let program = common::prepared(&alloc, &source);
    let mapped = Mapping {
        path: "./_runtime/webpack.js".to_string(),
        matcher: MappingMatcher::StringLiteral("eval say".to_string()),
    };
    let result = untwist_unpack::unpack(&alloc, &program, &[mapped]).expect("a webpack bundle");
    let module = &result.bundle.modules["2"];
    assert_eq!(module.path.as_str(), "_runtime/webpack.js");
    assert!(module.code().contains("eval say"), "{}", module.code());
    assert!(!result.bundle.modules.contains_key("_runtime/webpack"));
}

#[test]
fn a_module_id_that_claims_the_runtime_refuses_at_detection() {
    let alloc = Allocator::default();
    let control = locals_with_say_as("_runtime/webpackx");
    let program = common::prepared(&alloc, &control);
    assert!(untwist_unpack::unpack(&alloc, &program, &[]).is_some());
    for id in [
        "_runtime/webpack",
        "_runtime/webpack.js",
        "./_runtime/webpack",
    ] {
        let source = locals_with_say_as(id);
        let program = common::prepared(&alloc, &source);
        assert!(
            untwist_unpack::unpack(&alloc, &program, &[]).is_none(),
            "{id}"
        );
        let before = untwist_core::print::pretty_codegen(&program);
        let mut taken = common::prepared(&alloc, &source);
        assert!(
            untwist_unpack::unpack_taking(&alloc, &mut taken, &[]).is_none(),
            "{id}"
        );
        assert_eq!(untwist_core::print::pretty_codegen(&taken), before, "{id}");
        let conservative = untwist_unpack::UnpackOptions {
            conservative: true,
            ..Default::default()
        };
        assert!(
            untwist_unpack::detect::unpack_program_with(&alloc, &program, conservative).is_some(),
            "{id}"
        );
    }
}
