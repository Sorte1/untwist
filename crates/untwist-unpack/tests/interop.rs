mod common;

use common::bundle_of;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::webpack::interop::convert_interop;

const HARMONY: &str = "require.r(exports);\nrequire.d(exports, \"default\", function () {\n  return f;\n});\nfunction f() {}\n";
const PAIR: &str = "var lib = require(1);\nvar lib_default = require.n(lib);\n";

fn entry_after(entry: &'static str, target: &'static str) -> (usize, String) {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("0", entry, false), ("1", target, false)],
        Some("0"),
    );
    let changes = convert_interop(&alloc, &mut bundle);
    (changes, bundle.modules["0"].code().trim_end().to_string())
}

fn unchanged(entry: &'static str, target: &'static str) {
    let (changes, code) = entry_after(entry, target);
    assert_eq!(changes, 0, "{code}");
    assert_eq!(code, entry.trim_end());
}

#[test]
fn each_use_of_the_getter_reads_the_default_export_of_a_harmony_target() {
    let (changes, code) = entry_after(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nconsole.log(lib_default()(\"x\"), lib_default.a, typeof lib_default());\n",
        HARMONY,
    );
    assert_eq!(changes, 3);
    assert_eq!(
        code,
        "var lib = require(1);\nconsole.log((0, lib.default)(\"x\"), lib.default, typeof lib.default);"
    );
}

#[test]
fn a_use_inside_a_function_reads_the_export_when_the_function_runs() {
    let (_, code) = entry_after(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nexports.late = function () {\n  return lib_default.a;\n};\n",
        HARMONY,
    );
    assert_eq!(
        code,
        "var lib = require(1);\nexports.late = function () {\n  return lib.default;\n};"
    );
}

#[test]
fn a_tagged_getter_call_keeps_its_undefined_receiver() {
    let (_, code) = entry_after(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nlib_default()`x`;\n",
        HARMONY,
    );
    assert_eq!(code, "var lib = require(1);\n(0, lib.default)`x`;");
}

#[test]
fn string_ids_name_the_target_like_numeric_ones() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "./src/index.js",
                "var lib = require(\"./src/lib.js\");\nvar lib_default = require.n(lib);\nlib_default.a;\n",
                false,
            ),
            ("./src/lib.js", HARMONY, false),
        ],
        Some("./src/index.js"),
    );
    assert_eq!(convert_interop(&alloc, &mut bundle), 1);
    assert_eq!(
        bundle.modules["./src/index.js"].code().trim_end(),
        "var lib = require(\"./src/lib.js\");\nlib.default;"
    );
}

#[test]
fn a_target_already_converted_to_a_module_counts_as_harmony() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "0",
                "var lib = require(1);\nvar lib_default = require.n(lib);\nlib_default.a;\n",
                false,
            ),
            ("1", "export default 1;\n", true),
        ],
        Some("0"),
    );
    assert_eq!(convert_interop(&alloc, &mut bundle), 1);
}

#[test]
fn a_commonjs_target_that_sets_the_flag_itself_keeps_the_helper() {
    unchanged(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nconsole.log(lib_default()(\"x\"));\n",
        "Object.defineProperty(exports, \"__esModule\", { value: true });\nexports.default = function () {};\n",
    );
}

#[test]
fn a_target_that_marks_its_exports_late_keeps_the_helper() {
    unchanged(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nlib_default.a;\n",
        "var other = require(0);\nrequire.r(exports);\n",
    );
}

#[test]
fn a_target_that_can_replace_its_exports_keeps_the_helper() {
    unchanged(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nlib_default.a;\n",
        "require.r(exports);\nmodule.exports = function () {};\n",
    );
}

#[test]
fn an_argument_that_is_not_the_import_declared_just_before_keeps_the_helper() {
    unchanged(
        "var lib = require(1);\nvar other = pick();\nvar lib_default = require.n(other);\nlib_default.a;\n",
        HARMONY,
    );
    unchanged(
        "var lib = require(1);\nside();\nvar lib_default = require.n(lib);\nlib_default.a;\n",
        HARMONY,
    );
    unchanged("console.log(require.n(require(1))());\n", HARMONY);
}

#[test]
fn a_getter_that_escapes_keeps_the_helper() {
    for uses in [
        "use(lib_default);\n",
        "lib_default.a();\n",
        "lib_default(1);\n",
        "lib_default.b;\n",
        "lib_default = other;\n",
        "delete lib_default.a;\n",
        "new lib_default();\n",
    ] {
        let entry: &'static str = Box::leak(format!("{PAIR}{uses}").into_boxed_str());
        unchanged(entry, HARMONY);
    }
}

#[test]
fn a_reassigned_or_shadowed_import_keeps_the_helper() {
    unchanged(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nlib = other;\nlib_default.a;\n",
        HARMONY,
    );
    unchanged(
        "var lib = require(1);\nvar lib_default = require.n(lib);\nfunction inner(lib) {\n  return lib_default.a;\n}\n",
        HARMONY,
    );
}

#[test]
fn a_local_named_require_is_not_the_runtime() {
    unchanged(
        "function load(require) {\n  var lib = require(1);\n  var lib_default = require.n(lib);\n  return lib_default.a;\n}\n",
        HARMONY,
    );
}

#[test]
fn one_module_that_hands_the_runtime_away_stops_the_whole_bundle() {
    for leak in [
        "hand(require);\n",
        "require.c[1] = {};\n",
        "eval(\"1\");\n",
        "arguments[2].r(lib);\n",
    ] {
        let entry: &'static str =
            Box::leak(format!("{PAIR}lib_default.a;\n{leak}").into_boxed_str());
        unchanged(entry, HARMONY);
    }
}
