use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_core::print::pretty_codegen;
use untwist_testing::js_files_in;
use untwist_unpack::{holds_a_bundle, unpack, unpack_taking, BundleKind, UnpackResult};

fn code(result: Option<UnpackResult<'_>>, kind: BundleKind) -> Option<String> {
    let result = result?;
    assert_eq!(result.bundle.kind, kind);
    assert_eq!(result.bundle.modules.len(), 1);
    result
        .bundle
        .modules
        .values()
        .next()
        .map(|module| pretty_codegen(module.program()))
}

fn split(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    code(unpack(&alloc, &parsed.program, &[]), BundleKind::Umd)
}

fn split_taken(source: &str) -> (Option<String>, String) {
    let alloc = Allocator::default();
    let mut parsed = parse_program(&alloc, source);
    let found = code(
        unpack_taking(&alloc, &mut parsed.program, &[]),
        BundleKind::Umd,
    );
    (found, pretty_codegen(&parsed.program))
}

fn refused(source: &str) {
    assert_eq!(split(source), None, "split:\n{source}");
    let alloc = Allocator::default();
    let before = pretty_codegen(&parse_program(&alloc, source).program);
    let (taken, after) = split_taken(source);
    assert_eq!(taken, None);
    assert_eq!(after, before, "the refused input is left intact");
}

fn wrapper(body: &str, args: &str) -> String {
    format!("(function (root, factory) {{\n{body}\n}})({args});")
}

const NODE_ONLY: &str = "if (typeof exports === \"object\" && typeof module !== \"undefined\") factory(exports); else factory(root.lib = {});";

#[test]
fn the_commonjs_branch_decides_the_arguments() {
    let source = wrapper(NODE_ONLY, "this, function (exports) { exports.v = 1; }");
    assert_eq!(split(&source).as_deref(), Some("exports.v = 1;"));
}

#[test]
fn a_module_exports_assignment_keeps_the_return_value() {
    let source = wrapper(
        "if (typeof define === \"function\" && define.amd) define([\"path\"], factory);\n\
         else if (typeof module === \"object\" && module.exports) module.exports = factory(require(\"path\"));\n\
         else root.lib = factory(root.path);",
        "this, function (p) { return { sep: p.sep }; }",
    );
    assert_eq!(
        split(&source).as_deref(),
        Some("var p = require(\"path\");\nmodule.exports = { sep: p.sep };")
    );
}

#[test]
fn a_factory_that_returns_nothing_leaves_a_bare_assignment_undefined() {
    let source = wrapper(
        "typeof module === \"object\" ? module.exports = factory() : factory();",
        "this, function () { console.log(1); }",
    );
    assert_eq!(
        split(&source).as_deref(),
        Some("console.log(1);\nmodule.exports = void 0;")
    );
}

#[test]
fn a_value_returned_where_the_wrapper_drops_it_is_evaluated_not_exported() {
    let source = wrapper(
        NODE_ONLY,
        "this, function (exports) { return exports.v = 1; }",
    );
    assert_eq!(split(&source).as_deref(), Some("exports.v = 1;"));
}

#[test]
fn the_factory_may_come_first_and_the_wrapper_may_be_rollups_one_parameter_form() {
    let first = "(function (factory, root) {\n if (typeof exports === \"object\") factory(exports); else factory(root.x = {});\n})(function (exports) { exports.v = 2; }, this);";
    assert_eq!(split(first).as_deref(), Some("exports.v = 2;"));
    let rollup = "(function (factory) {\n typeof define === 'function' && define.amd ? define(factory) : factory();\n})((function () { console.log(3); }));";
    assert_eq!(split(rollup).as_deref(), Some("console.log(3);"));
}

#[test]
fn the_probe_follows_the_same_proof_as_the_detector() {
    let alloc = Allocator::default();
    let text = wrapper(NODE_ONLY, "this, function (exports) {}");
    let yes = parse_program(&alloc, &text);
    assert!(holds_a_bundle(&yes.program));
    let no = parse_program(
        &alloc,
        "(function (a, b) { return b; })(this, function () { return 1; });",
    );
    assert!(!holds_a_bundle(&no.program));
}

#[test]
fn a_mapped_parameter_keeps_its_name_and_extra_parameters_stay_declared() {
    let source = wrapper(
        "if (typeof exports === \"object\") factory(exports, module, require(\"os\"));",
        "this, function (e, m, os, extra) { e.v = m.id + os.EOL; }",
    );
    assert_eq!(
        split(&source).as_deref(),
        Some("var e = exports;\nvar m = module;\nvar os = require(\"os\");\nvar extra;\ne.v = m.id + os.EOL;")
    );
}

#[test]
fn a_wrapper_the_proof_does_not_cover_is_refused() {
    let factory = "this, function (exports) { exports.v = 1; }";
    for source in [
        wrapper("var seen = 1;\nif (typeof exports === \"object\") factory(exports);", factory),
        wrapper("if (typeof window === \"undefined\") factory(exports);", factory),
        wrapper("if (typeof define === \"function\") factory(exports);", factory),
        wrapper("if (typeof exports === \"object\") factory(exports, this);", factory),
        wrapper("if (typeof exports === \"object\") factory(exports, require(\"./a\"));", factory),
        wrapper("if (typeof exports === \"object\") factory(exports);\nvar after = 1;", factory),
        format!("{}\nafter();", wrapper(NODE_ONLY, factory)),
        wrapper("if (typeof exports === \"object\") factory(exports, require(\"/a\"));", factory),
        wrapper("if (typeof exports === \"object\") factory(exports, require(a));", factory),
        wrapper("if (typeof exports === \"object\") factory(...exports);", factory),
        wrapper("if (typeof exports === \"object\") root.factory(exports);", factory),
        wrapper("if (typeof exports === \"object\") other(exports);", factory),
        wrapper("if (typeof exports === \"object\") exports.v = factory(exports);", factory),
        wrapper("if (typeof exports === \"object\") factory?.(exports);", factory),
        wrapper("if (typeof exports !== \"object\") factory(exports);", factory),
        wrapper("if (typeof exports === \"object\") { factory(exports); factory(exports); }", factory),
        "(function (exports, factory) { factory(exports); })(this, function (exports) {});".to_string(),
        "(function (root, factory) { if (typeof exports === \"object\") factory(exports); })(this, function (exports) {}, function () {});".to_string(),
        "var lib = (function (root, factory) { if (typeof exports === \"object\") factory(exports); })(this, function (exports) {});".to_string(),
        "(function (root, factory) { if (typeof exports === \"object\") factory(exports); })(this, () => 1);".to_string(),
    ] {
        refused(&source);
    }
}

fn with_factory(factory: &str) -> String {
    wrapper(
        "if (typeof exports === \"object\") factory(exports, require(\"os\"));",
        &format!("this, {factory}"),
    )
}

#[test]
fn a_factory_that_is_not_closed_is_refused() {
    for factory in [
        "function (exports) { return this; }",
        "function (exports) { return arguments; }",
        "function (exports) { if (a) { return 1; } return 2; }",
        "function (exports) { var module = 1; return module; }",
        "function (exports) { var require = 1; }",
        "async function (exports) { }",
    ] {
        refused(&with_factory(factory));
    }
}

#[test]
fn a_function_declaration_that_takes_a_parameters_name_is_refused() {
    for factory in [
        "function (e, os) { function os() {} e.v = os(); }",
        "function (e) { function e() {} e(); }",
        "function (e, os) { if (e) { function os() {} } e.v = os; }",
    ] {
        refused(&with_factory(factory));
    }
    assert!(split(&with_factory(
        "function (e, os) { e.v = function () { function os() {} return os; }; }"
    ))
    .is_some());
}

#[test]
fn a_factory_that_reads_a_host_name_free_is_refused() {
    for read in [
        "__filename",
        "__dirname",
        "module.filename",
        "require.main",
        "exports",
    ] {
        refused(&with_factory(&format!("function (e) {{ e.v = {read}; }}")));
    }
    assert!(split(&with_factory("function (e) { e.v = 1; }")).is_some());
}

#[test]
fn fewer_parameters_than_arguments_still_evaluate_the_extra_requires() {
    assert_eq!(
        split(&with_factory("function (a) { return a; }")).as_deref(),
        Some("var a = exports;\nrequire(\"os\");\na;")
    );
}

#[test]
fn two_wrappers_or_wrapper_code_around_the_call_are_refused() {
    refused(&format!(
        "{}\n{}",
        wrapper(NODE_ONLY, "this, function (exports) {}"),
        wrapper(NODE_ONLY, "this, function (exports) {}")
    ));
    refused(&format!(
        "a();\n{}",
        wrapper(NODE_ONLY, "this, function (exports) {}")
    ));
}

#[test]
fn a_webpack_bundle_inside_a_umd_wrapper_is_found_by_the_first_detectors() {
    let inner = "(function (modules) {\n var cache = {};\n function __webpack_require__(id) {\n  if (cache[id]) return cache[id].exports;\n  var module = cache[id] = { exports: {} };\n  modules[id].call(module.exports, module, module.exports, __webpack_require__);\n  return module.exports;\n }\n return __webpack_require__(0);\n})([function (module, exports) { exports.v = 1; }, function (module, exports) { exports.w = 2; }])";
    let source = wrapper(
        "if (typeof exports === \"object\") module.exports = factory(); else root.lib = factory();",
        &format!("this, function () {{ return {inner}; }}"),
    );
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, &source);
    let result = unpack(&alloc, &parsed.program, &[]).expect("a bundle");
    assert_eq!(result.bundle.kind, BundleKind::Webpack);
}

#[test]
fn the_fixtures_extract_alike_taken_and_cloned() {
    let mut found = 0;
    for (label, source) in js_files_in("fixtures/unpack-bundlers/umd") {
        let cloned = split(&source);
        let (taken, _) = split_taken(&source);
        assert_eq!(taken, cloned, "{label}");
        found += usize::from(cloned.is_some());
    }
    assert_eq!(found, 4);
}

#[test]
fn a_sequence_before_the_wrapper_or_a_wrapper_argument_with_effects_is_refused() {
    let args = "this, function (exports) {}";
    assert!(split(&wrapper(NODE_ONLY, args)).is_some());
    let call = wrapper(NODE_ONLY, args);
    let call = call.trim_end_matches(';');
    refused(&format!("f(), {call};"));
    refused(&format!("!(f(), {call});"));
    refused(&wrapper(NODE_ONLY, "(f(), this), function (exports) {}"));
    refused(&wrapper(NODE_ONLY, "f(), function (exports) {}"));
    refused(&wrapper(NODE_ONLY, "self, function (exports) {}"));
    refused(&wrapper(
        NODE_ONLY,
        "typeof self !== \"undefined\" ? f() : this, function (exports) {}",
    ));
    refused(&wrapper(
        NODE_ONLY,
        "typeof self !== \"undefined\" ? window : this, function (exports) {}",
    ));
    refused(&wrapper(
        NODE_ONLY,
        "typeof self === \"undefined\" ? self : this, function (exports) {}",
    ));
    refused(&wrapper(
        NODE_ONLY,
        "typeof self !== \"undefined\" ? self : f(), function (exports) {}",
    ));
    for root in [
        "typeof self !== \"undefined\" ? self : this",
        "\"undefined\" != typeof globalThis ? globalThis : typeof self !== \"undefined\" ? self : this",
        "null",
    ] {
        assert!(
            split(&wrapper(NODE_ONLY, &format!("{root}, function (exports) {{}}"))).is_some(),
            "{root}"
        );
    }
}

#[test]
fn a_bare_define_read_is_not_folded() {
    let factory = "this, function () { return 1; }";
    for body in [
        "if (define && define.amd) define(factory); else module.exports = factory();",
        "if (!define) module.exports = factory(); else define(factory);",
        "define ? define(factory) : (module.exports = factory());",
    ] {
        refused(&wrapper(body, factory));
    }
    let probed = wrapper(
        "if (typeof define === \"function\" && define.amd) define(factory); else module.exports = factory();",
        factory,
    );
    assert!(split(&probed).is_some());
}
