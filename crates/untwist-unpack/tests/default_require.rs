mod common;

use common::bundle_of;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::webpack::default_require::convert_default_require;

const ENTRY: &str = "const b = require(1);\nconst c = require(2);\nconst bDefault = require.n(b);\nconst cDefault = require.n(c);\nconst dDefault = require.n(b).a;\nconst eDefault = require.n(c)();\nconsole.log(bDefault().VERSION);\nconsole.log(cDefault.a.VERSION);\n";

#[test]
fn interop_getters_become_the_module_or_its_default_export() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            ("0", ENTRY, false),
            ("1", "export default 1;\n", true),
            ("2", "module.exports = 1;\n", false),
        ],
        Some("0"),
    );
    convert_default_require(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "const b = require(1);\nconst c = require(2);\nconst bDefault = b.default;\nconst cDefault = c;\nconst dDefault = b.default;\nconst eDefault = c;\nconsole.log(bDefault.VERSION);\nconsole.log(cDefault.VERSION);"
    );
}

#[test]
fn an_unresolvable_argument_still_loses_the_helper() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("0", "console.log(require.n(x).a);\n", false)],
        Some("0"),
    );
    convert_default_require(&alloc, &mut bundle);
    assert_eq!(bundle.modules["0"].code().trim_end(), "console.log(x);");
}

#[test]
fn string_module_ids_never_resolve() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "0",
                "const b = require('1');\nconst g = require.n(b);\n",
                false,
            ),
            ("1", "export default 1;\n", true),
        ],
        Some("0"),
    );
    convert_default_require(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "const b = require(\"1\");\nconst g = b;"
    );
}

#[test]
fn other_helper_shapes_are_left_alone() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[(
            "0",
            "const b = require(1);\nconsole.log(require.n(b, 2));\nconsole.log(require.n(b)(1));\nconsole.log(require.t(b).a);\n",
            false,
        )],
        Some("0"),
    );
    convert_default_require(&alloc, &mut bundle);
    let code = bundle.modules["0"].code().to_string();
    assert!(code.contains("require.n(b, 2)"), "{code}");
    assert!(code.contains("require.n(b)(1)"), "{code}");
    assert!(code.contains("require.t(b).a"), "{code}");
}

#[test]
fn a_getter_with_no_declarator_does_not_collapse_member_access() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("0", "const b = require(1);\nconsole.log(b.prop);\n", false)],
        Some("0"),
    );
    convert_default_require(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "const b = require(1);\nconsole.log(b.prop);"
    );
}
