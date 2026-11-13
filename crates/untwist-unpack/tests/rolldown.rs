use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_unpack::{unpack, unpack_taking, BundleKind};

const HELPER_1_3: &str = "var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = null), mod.exports);\n";
const HELPER_1_0: &str = "var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod), cb = null), mod.exports);\n";
const HELPER_UNMINIFIED: &str = "const __commonJSMin = (cb, mod) => () => {
  if (!mod) {
    cb((mod = { exports: {} }).exports, mod, mod.exports);
    cb = null;
  }
  return mod.exports;
};
";
const NOT_A_HELPER: &str = "var __commonJSMin = (cb, mod) => () => (mod || (cb((mod = { exports: {} }).exports, mod, mod.exports), cb = 1), mod.exports);\n";

type Files = Vec<(String, String)>;

fn modules(source: &str) -> Option<(BundleKind, Files)> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    let result = unpack(&alloc, &parsed.program, &[])?;
    let files = result
        .bundle
        .modules
        .values()
        .map(|module| {
            (
                module.path.as_str().to_string(),
                pretty_codegen(module.program()),
            )
        })
        .collect();
    Some((result.bundle.kind, files))
}

fn taken_count(source: &str) -> Option<usize> {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    unpack_taking(&alloc, &mut parsed.program, &[]).map(|result| result.bundle.modules.len())
}

fn two_factories(helper: &str) -> String {
    format!(
        "{helper}var require_a = /* @__PURE__ */ __commonJSMin(((exports) => {{\n  exports.x = 1;\n}}));\nvar require_b = __commonJSMin((exports, module) => {{\n  module.exports = require_a().x + 1;\n}});\nconsole.log(require_b());\n"
    )
}

#[test]
fn every_rolldown_commonjs_helper_form_splits_its_factories() {
    for helper in [HELPER_1_3, HELPER_1_0, HELPER_UNMINIFIED] {
        let source = two_factories(helper);
        let (kind, files) = modules(&source).expect("a bundle");
        assert_eq!(kind, BundleKind::Rolldown, "{helper}");
        assert_eq!(files.len(), 3, "{helper}: {files:?}");
        assert_eq!(taken_count(&source), Some(3), "{helper}");
    }
}

#[test]
fn a_helper_that_does_not_reset_to_null_is_not_rolldowns() {
    let source = two_factories(NOT_A_HELPER);
    assert!(modules(&source).is_none_or(|(_, files)| files.len() == 1));
}

#[test]
fn a_factory_reached_before_its_declaration_runs_is_refused() {
    let source = format!(
        "{HELPER_1_3}console.log(early());\nfunction early() {{ return require_a().x; }}\nvar require_a = __commonJSMin((exports) => {{\n  exports.x = 1;\n}});\n"
    );
    assert!(modules(&source).is_none());
}

#[test]
fn a_factory_declared_after_code_that_runs_is_split_when_every_use_comes_later() {
    let source = format!(
        "{HELPER_1_3}console.log(\"first\");\nvar require_a = __commonJSMin((exports) => {{\n  exports.x = 1;\n}});\nconsole.log(require_a().x);\n"
    );
    let (_, files) = modules(&source).expect("a bundle");
    assert_eq!(files.len(), 2, "{files:?}");
}

#[test]
fn a_factory_that_reads_hoisted_code_stays_in_the_entry_while_the_others_split() {
    let source = format!(
        "{HELPER_1_3}var require_a = __commonJSMin((exports) => {{\n  exports.x = 1;\n}});\nvar shared = 2;\nvar require_b = __commonJSMin((exports) => {{\n  exports.y = shared + require_a().x;\n}});\nconsole.log(require_b().y);\n"
    );
    let (_, files) = modules(&source).expect("a bundle");
    assert_eq!(files.len(), 2, "{files:?}");
    let entry = &files
        .iter()
        .find(|(path, _)| path == "index.js")
        .expect("entry")
        .1;
    assert!(entry.contains("shared + require"), "{entry}");
    assert_eq!(taken_count(&source), Some(2));
}

#[test]
fn an_inline_factory_call_becomes_a_require_of_its_own_module() {
    let source = format!(
        "{HELPER_1_3}var __toESM = (mod) => mod;\nvar import_greet = __toESM(__commonJSMin((exports, module) => {{\n  module.exports = \"hi\";\n}})());\nconsole.log(import_greet);\n"
    );
    let (_, files) = modules(&source).expect("a bundle");
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(
        files.iter().any(|(path, _)| path == "greet.js"),
        "{files:?}"
    );
}

#[test]
fn a_function_iife_reading_this_at_its_top_is_refused() {
    let source = format!(
        "(function () {{\n{}console.log(this === undefined, require_b());\n}})();\n",
        two_factories(HELPER_1_3).replace("console.log(require_b());\n", "")
    );
    assert!(modules(&source).is_none());
    let plain = format!(
        "(function () {{\n{}console.log(require_b());\n}})();\n",
        two_factories(HELPER_1_3).replace("console.log(require_b());\n", "")
    );
    assert_eq!(modules(&plain).map(|(_, files)| files.len()), Some(3));
}
