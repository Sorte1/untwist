mod common;

use common::bundle_of;
use common::PrintedModule;
use oxc_allocator::Allocator;
use untwist_unpack::path::PosixPath;
use untwist_unpack::webpack::require_paths::replace_require_paths;

#[test]
fn numeric_and_string_ids_become_relative_specifiers() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            (
                "2",
                "const a = require(3);\nconst b = require('E8gZ');\n",
                false,
            ),
            ("3", "module.exports = 1;\n", false),
            ("E8gZ", "module.exports = 2;\n", false),
        ],
        Some("2"),
    );
    let (missing, _) = replace_require_paths(&alloc, &mut bundle);
    assert!(missing.is_empty());
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const a = require(\"./3.js\");\nconst b = require(\"./E8gZ.js\");"
    );
}

#[test]
fn a_missing_target_is_reported_and_gets_an_id_derived_specifier() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("2wwy", "const b = require('w6GO');\n", false)],
        None,
    );
    let (missing, _) = replace_require_paths(&alloc, &mut bundle);
    assert_eq!(missing, vec![("2wwy".to_string(), "w6GO".to_string())]);
    assert_eq!(
        bundle.modules["2wwy"].code().trim_end(),
        "const b = require(\"./w6GO.js\");"
    );
}

#[test]
fn a_missing_dotted_id_from_a_subdirectory_is_normalised() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("2", "const a = require('./missing');\n", false)],
        Some("2"),
    );
    bundle.modules["2"].path = PosixPath::parse("src/a.js").expect("safe");
    let (missing, _) = replace_require_paths(&alloc, &mut bundle);
    assert_eq!(missing, vec![("2".to_string(), "./missing".to_string())]);
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const a = require(\"../missing.js\");"
    );
}

#[test]
fn specifiers_are_relative_to_the_requiring_module() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            ("2", "const a = require(1);\nconst b = require(4);\n", false),
            ("1", "module.exports = 1;\n", false),
            ("4", "module.exports = 4;\n", false),
        ],
        Some("2"),
    );
    bundle.modules["2"].path = PosixPath::parse("utils/color.js").expect("safe");
    bundle.modules["4"].path = PosixPath::parse("node_modules/package").expect("safe");
    replace_require_paths(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const a = require(\"../1.js\");\nconst b = require(\"package\");"
    );
}

#[test]
fn import_declarations_are_rewritten_too() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[
            ("0", "import * as m from \"1\";\nconsole.log(m);\n", true),
            ("1", "module.exports = 1;\n", false),
        ],
        Some("0"),
    );
    replace_require_paths(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["0"].code().trim_end(),
        "import * as m from \"./1.js\";\nconsole.log(m);"
    );
}

#[test]
fn non_literal_and_multi_argument_requires_are_left_alone() {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[(
            "2",
            "const a = require(require.ab + \"build/Release/spdlog.node\");\nconst b = require(1, 2);\nconst c = other(1);\n",
            false,
        )],
        Some("2"),
    );
    replace_require_paths(&alloc, &mut bundle);
    assert_eq!(
        bundle.modules["2"].code().trim_end(),
        "const a = require(require.ab + \"build/Release/spdlog.node\");\nconst b = require(1, 2);\nconst c = other(1);"
    );
}

fn rewritten_alone(source: &'static str) -> String {
    let alloc = Allocator::default();
    let mut bundle = bundle_of(
        &alloc,
        &[("0", source, false), ("1", "module.exports = 1;\n", false)],
        Some("0"),
    );
    replace_require_paths(&alloc, &mut bundle);
    bundle.modules["0"].code().trim_end().to_string()
}

#[test]
fn a_local_named_require_keeps_its_arguments() {
    for source in [
        "function load(require) {\n  return require(1);\n}\n",
        "function load() {\n  var require = other;\n  return require(1);\n}\n",
        "function load() {\n  function require() {}\n  return require(1);\n}\n",
        "var load = function require() {\n  return require(1);\n};\n",
        "try {} catch (require) {\n  require(1);\n}\n",
        "var load = (require) => require(1);\n",
        "function load(require) {\n  return require.bind(null, 1);\n}\n",
        "function load(require) {\n  return require.t(1, 9);\n}\n",
        "class Loader {\n  load(require) {\n    return require(1);\n  }\n}\n",
    ] {
        assert_eq!(rewritten_alone(source), source.trim_end());
    }
}

#[test]
fn the_runtime_beside_a_local_named_require_is_still_rewritten() {
    assert_eq!(
        rewritten_alone("function load(require) {\n  return require(1);\n}\nvar a = require(1);\n"),
        "function load(require) {\n  return require(1);\n}\nvar a = require(\"./1.js\");"
    );
}
