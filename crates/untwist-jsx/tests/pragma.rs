use untwist_core::pass::PassOptions;
use untwist_testing::{transform_with, Allocator};

fn opts(extended: bool) -> PassOptions {
    PassOptions {
        unsafe_transforms: true,
        extended,
        max_iterations: None,
        fuel: None,
        ..PassOptions::default()
    }
}

fn run(source: &str, extended: bool) -> String {
    let alloc = Allocator::default();
    transform_with(
        &alloc,
        source,
        &[untwist_jsx::classic::boxed],
        opts(extended),
    )
}

const REQUIRED: &str = "var r = require(\"react\");\nr.createElement(\"div\", null);";
const IMPORTED: &str = "import R from \"react\";\nR.createElement(\"div\", null);";
const NAMESPACED: &str = "import * as n from \"react\";\nn.createElement(\"div\", null);";
const UMD: &str = "var q = window.React;\nq.createElement(\"div\", null);";
const UNRELATED: &str = "var r = require(\"preact\");\nr.createElement(\"div\", null);";

#[test]
fn a_required_react_binding_is_the_pragma_by_default() {
    assert_eq!(run(REQUIRED, true), "var r = require(\"react\");\n<div />;");
}

#[test]
fn an_imported_react_default_is_the_pragma_by_default() {
    assert_eq!(run(IMPORTED, true), "import R from \"react\";\n<div />;");
}

#[test]
fn an_imported_react_namespace_is_the_pragma_by_default() {
    assert_eq!(
        run(NAMESPACED, true),
        "import * as n from \"react\";\n<div />;"
    );
}

#[test]
fn a_umd_global_read_is_the_pragma_by_default() {
    assert_eq!(run(UMD, true), "var q = window.React;\n<div />;");
}

#[test]
fn an_unrelated_module_is_not_the_pragma() {
    assert_eq!(run(UNRELATED, true), UNRELATED);
}

#[test]
fn conservative_still_matches_only_the_literal_name() {
    assert_eq!(run(REQUIRED, false), REQUIRED);
    assert_eq!(
        run("React.createElement(\"div\", null);", false),
        "<div />;"
    );
}

#[test]
fn the_literal_name_still_converts_by_default() {
    assert_eq!(run("React.createElement(\"div\", null);", true), "<div />;");
}

const FRAGMENT_VIA_RENAMED_IMPORT: &str =
    "import R from \"react\";\nR.createElement(R.Fragment, null, \"x\");";
const FRAGMENT_VIA_RENAMED_IMPORT_COMPUTED: &str =
    "import R from \"react\";\nR.createElement(R[\"Fragment\"], null, \"x\");";

#[test]
fn a_renamed_react_import_still_takes_the_fragment_shortcut() {
    assert_eq!(
        run(FRAGMENT_VIA_RENAMED_IMPORT, true),
        "import R from \"react\";\n<>x</>;"
    );
}

#[test]
fn a_computed_fragment_member_on_a_renamed_import_still_takes_the_shortcut() {
    assert_eq!(
        run(FRAGMENT_VIA_RENAMED_IMPORT_COMPUTED, true),
        "import R from \"react\";\n<>x</>;"
    );
}

#[test]
fn under_conservative_a_renamed_react_import_does_not_take_the_fragment_shortcut() {
    assert_eq!(
        run(FRAGMENT_VIA_RENAMED_IMPORT, false),
        FRAGMENT_VIA_RENAMED_IMPORT
    );
}

const DEFAULT_NAMED_IMPORT: &str =
    "import { default as R } from \"react\";\nR.createElement(\"div\", null);";

#[test]
fn a_default_named_import_is_the_pragma_by_default() {
    assert_eq!(
        run(DEFAULT_NAMED_IMPORT, true),
        "import { default as R } from \"react\";\n<div />;"
    );
}

const MULTI_ARGUMENT_WRAPPER: &str =
    "var r = log(1, require(\"react\"), 2);\nr.createElement(\"div\", null);";

#[test]
fn a_multi_argument_wrapper_around_require_is_not_the_pragma() {
    assert_eq!(run(MULTI_ARGUMENT_WRAPPER, true), MULTI_ARGUMENT_WRAPPER);
}

const SHADOWED_PARAMETER: &str =
    "var r = require(\"react\");\nfunction f(r) {\n  return r.createElement(\"div\", null);\n}";

#[test]
fn a_parameter_shadowing_a_react_binding_is_not_the_pragma() {
    assert_eq!(run(SHADOWED_PARAMETER, true), SHADOWED_PARAMETER);
}

const UNRELATED_TOP_LEVEL_BINDING: &str = "function m() {\n  var r = require(\"react\");\n}\nvar r = document;\nr.createElement(\"div\", null);";

#[test]
fn a_require_local_to_one_function_does_not_convert_an_unrelated_top_level_binding() {
    assert_eq!(
        run(UNRELATED_TOP_LEVEL_BINDING, true),
        UNRELATED_TOP_LEVEL_BINDING
    );
}

#[test]
fn a_literal_react_bound_to_a_non_react_declarator_is_refused_by_default_and_conservative_inherited(
) {
    let source = "function g() {\n  var React = { createElement: h };\n  return React.createElement(\"p\", null);\n}";
    assert_eq!(run(source, true), source);
    assert_eq!(
        run(source, false),
        "function g() {\n  var React = { createElement: h };\n  return <p />;\n}"
    );
}
