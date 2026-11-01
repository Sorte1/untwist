mod common;

use common::parse;
use common::PrintedModule;
use oxc_allocator::Allocator;
use oxc_span::SourceType;
use untwist_unpack::model::Module;
use untwist_unpack::webpack::esm::convert_esm;

fn convert(source: &str) -> (String, bool) {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    program.source_type = SourceType::cjs();
    let mut module = Module::new("0".to_string(), program, false);
    convert_esm(&alloc, &mut module);
    (module.code().trim_end().to_string(), module.is_esm())
}

#[test]
fn require_r_sets_the_module_flag_and_disappears() {
    let (code, is_esm) = convert("require.r(exports);\nconsole.log(1);\n");
    assert_eq!(code, "console.log(1);");
    assert!(is_esm);
}

#[test]
fn require_r_on_a_local_object_also_flips_the_module() {
    let (code, is_esm) = convert("var x = {};\nrequire.r(x);\n");
    assert_eq!(code, "var x = {};");
    assert!(is_esm);
}

#[test]
fn numeric_requires_become_namespace_imports_once_esm() {
    let (code, _) = convert("require.r(exports);\nconst lib = require(3);\nconsole.log(lib);\n");
    assert_eq!(code, "import * as lib from \"3\";\nconsole.log(lib);");
}

#[test]
fn requires_stay_calls_when_the_module_is_not_esm() {
    let (code, is_esm) = convert("const lib = require(3);\n");
    assert_eq!(code, "const lib = require(3);");
    assert!(!is_esm);
}

#[test]
fn string_ids_and_multi_declarators_are_left_alone() {
    let (code, _) = convert("require.r(exports);\nconst lib = require('2wwy');\n");
    assert_eq!(code, "const lib = require(\"2wwy\");");
    let (code, _) = convert("require.r(exports);\nconst a = require(1), b = 2;\n");
    assert_eq!(code, "const a = require(1), b = 2;");
}

#[test]
fn hmd_assignments_are_removed() {
    let (code, _) = convert("module = require.hmd(module);\nmodule.exports = 1;\n");
    assert_eq!(code, "module.exports = 1;");
}

#[test]
fn nested_statements_are_not_converted() {
    let (code, _) = convert("require.r(exports);\nfunction f() {\n  const lib = require(3);\n}\n");
    assert_eq!(code, "function f() {\n  const lib = require(3);\n}");
}

#[test]
fn a_named_single_define_renames_and_exports_the_declaration() {
    let (code, is_esm) = convert(
        "require.r(exports);\nrequire.d(exports, 'counter', function () {\n  return f;\n});\nclass f {}\nlet counter = 1;\nf = 2;\n",
    );
    assert_eq!(
        code,
        "export class counter {}\nlet _counter = 1;\ncounter = 2;"
    );
    assert!(is_esm);
}

#[test]
fn a_named_single_define_over_a_variable_keeps_the_declaration_kind() {
    let (code, _) =
        convert("require.d(exports, 'foo', function () {\n  return b;\n});\nconst b = 2;\n");
    assert_eq!(code, "export const foo = 2;");
}

#[test]
fn a_default_single_define_replaces_the_declaration_with_its_initialiser() {
    let (code, _) =
        convert("require.d(exports, 'default', function () {\n  return y;\n});\nlet y = 1;\n");
    assert_eq!(code, "export default 1;");
}

#[test]
fn a_default_single_define_over_a_class_keeps_the_class() {
    let (code, _) =
        convert("require.d(exports, 'default', function () {\n  return f;\n});\nclass f {}\n");
    assert_eq!(code, "export default class f {}");
}

#[test]
fn bulk_defines_insert_in_reverse_property_order() {
    let (code, _) = convert(
        "require.r(exports);\nrequire.d(exports, {\n  default: () => a.foo,\n  version: () => b.version\n});\n",
    );
    assert_eq!(
        code,
        "export let version = b.version;\nexport default a.foo;"
    );
}

#[test]
fn a_bulk_define_on_an_empty_local_object_folds_into_it() {
    let (code, _) =
        convert("var x = {};\nrequire.d(x, {\n  Console: () => bar\n});\nfunction bar() {}\n");
    assert_eq!(code, "var x = { Console: bar };\nfunction bar() {}");
}

#[test]
fn an_export_of_an_undeclared_identifier_only_removes_the_call() {
    let (code, _) = convert("require.r(exports);\nrequire.d(exports, 'foo', function () {\n  return unknownGlobal;\n});\n");
    assert_eq!(code, "");
}

#[test]
fn defines_with_the_wrong_shape_are_left_alone() {
    let (code, _) = convert("require.d(exports);\n");
    assert_eq!(code, "require.d(exports);");
    let (code, _) = convert("require.d(exports, 'foo', () => b);\n");
    assert_eq!(code, "require.d(exports, \"foo\", () => b);");
}

#[test]
fn nested_defines_are_not_converted() {
    let (code, _) = convert(
        "function f() {\n  require.d(exports, 'foo', function () {\n    return b;\n  });\n}\n",
    );
    assert!(code.contains("require.d"), "{code}");
}

#[test]
fn a_single_define_with_a_rest_parameter_is_left_alone() {
    let (code, _) = convert("require.d(exports, 'a', function (...args) {\n  return b;\n});\n");
    assert_eq!(
        code,
        "require.d(exports, \"a\", function (...args) {\n  return b;\n});"
    );
}

#[test]
fn a_plain_cjs_module_keeps_its_commonjs_source_type() {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, "const lib = require(3);\n");
    program.source_type = SourceType::cjs();
    let mut module = Module::new("0".to_string(), program, false);
    convert_esm(&alloc, &mut module);
    assert!(!module.is_esm());
    assert!(module.program().source_type.is_commonjs());
}

#[test]
fn running_convert_esm_twice_is_idempotent() {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, "require.r(exports);\nconsole.log(1);\n");
    program.source_type = SourceType::cjs();
    let mut module = Module::new("0".to_string(), program, false);
    convert_esm(&alloc, &mut module);
    assert!(module.is_esm());
    let first_code = module.code().trim_end().to_string();
    let changes = convert_esm(&alloc, &mut module);
    assert_eq!(changes, 0);
    assert!(module.is_esm());
    assert_eq!(module.code().trim_end().to_string(), first_code);
}
