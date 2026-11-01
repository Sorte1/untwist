mod common;

use common::parse;
use oxc_allocator::Allocator;
use untwist_core::print::{print, PrintStyle};
use untwist_unpack::rename::rename_binding;
use untwist_unpack::rename::{rename_parameters, rename_parameters_uncaptured};

const WEBPACK: [&str; 3] = ["module", "exports", "require"];
const BROWSERIFY: [&str; 3] = ["require", "module", "exports"];

fn rename(source: &str, original: &[&str], renamed: &[&str]) -> String {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    rename_parameters(&alloc, &mut program, original, renamed);
    print(&program, PrintStyle::Pretty).trim_end().to_string()
}

#[test]
fn colliding_bindings_move_out_of_the_way_like_babel() {
    let source = "const a = i(1);\nconst module = 1;\ne.exports.color = '#FBC02D';\n{\n  const module = 2;\n  console.log(module);\n  console.log(e);\n}\nt.a = 3;\n";
    assert_eq!(
        rename(source, &["e", "t", "i"], &WEBPACK),
        "const a = require(1);\nconst _module = 1;\nmodule.exports.color = \"#FBC02D\";\n{\n  const _module2 = 2;\n  console.log(_module2);\n  console.log(module);\n}\nexports.a = 3;"
    );
}

#[test]
fn identical_names_leave_nested_bindings_alone() {
    let source = "module.exports = 1;\n(function (e) {\n  const module = 1;\n  console.log(module);\n})();\n";
    assert_eq!(
        rename(source, &["require", "module", "exports"], &BROWSERIFY),
        "module.exports = 1;\n(function (e) {\n  const module = 1;\n  console.log(module);\n})();"
    );
}

#[test]
fn shadowed_references_are_not_renamed() {
    let source = "function f(e) {\n  return e;\n}\ne.x = 1;\n";
    assert_eq!(
        rename(source, &["e", "t", "i"], &WEBPACK),
        "function f(e) {\n  return e;\n}\nmodule.x = 1;"
    );
}

#[test]
fn assignment_targets_are_renamed_too() {
    let source = "e = i.hmd(e);\ni.r(t);\n";
    assert_eq!(
        rename(source, &["e", "t", "i"], &WEBPACK),
        "module = require.hmd(module);\nrequire.r(exports);"
    );
}

#[test]
fn a_short_parameter_list_only_renames_what_it_has() {
    let source = "module.exports = 'foo';\n";
    assert_eq!(
        rename(source, &["module"], &WEBPACK),
        "module.exports = \"foo\";"
    );
}

fn rename_one(source: &str, from: &str, to: &str) -> String {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    rename_binding(&alloc, &mut program, from, to);
    print(&program, PrintStyle::Pretty).trim_end().to_string()
}

#[test]
fn renaming_a_binding_moves_the_conflicting_one_aside() {
    assert_eq!(
        rename_one("class f {}\nlet counter = 1;\nf = 2;\n", "f", "counter"),
        "class counter {}\nlet _counter = 1;\ncounter = 2;"
    );
}

#[test]
fn renaming_without_a_conflict_touches_one_binding() {
    assert_eq!(
        rename_one("const b = 2;\nconsole.log(b);\n", "b", "foo"),
        "const foo = 2;\nconsole.log(foo);"
    );
}

#[test]
fn renaming_an_unknown_binding_changes_nothing() {
    assert_eq!(rename_one("const b = 2;\n", "q", "foo"), "const b = 2;");
    assert_eq!(rename_one("const b = 2;\n", "b", "b"), "const b = 2;");
}

#[test]
fn nested_bindings_of_the_same_name_are_not_renamed() {
    assert_eq!(
        rename_one(
            "const b = 2;\nfunction f() {\n  const b = 3;\n  return b;\n}\n",
            "b",
            "foo"
        ),
        "const foo = 2;\nfunction f() {\n  const b = 3;\n  return b;\n}"
    );
}

fn rename_uncaptured(source: &str, original: &[&str], renamed: &[&str]) -> String {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    rename_parameters_uncaptured(&alloc, &mut program, original, renamed);
    print(&program, PrintStyle::Pretty).trim_end().to_string()
}

#[test]
fn the_reference_rename_still_folds_a_parameter_onto_a_free_name() {
    let source = "var path = require(\"path\");\nvar lib = n(1);\n";
    assert_eq!(
        rename(source, &["e", "t", "n"], &WEBPACK),
        "var path = require(\"path\");\nvar lib = require(1);"
    );
}

#[test]
fn a_parameter_is_not_renamed_onto_the_hosts_require() {
    let source = "var path = require(\"path\");\nvar lib = n(1);\ne.exports = path.join(lib);\n";
    assert_eq!(
        rename_uncaptured(source, &["e", "t", "n"], &WEBPACK),
        "var path = require(\"path\");\nvar lib = _require(1);\nmodule.exports = path.join(lib);"
    );
}

#[test]
fn a_parameter_is_not_renamed_onto_a_free_module_or_exports() {
    let source = "if (typeof module === \"object\") e.exports = t;\n";
    assert_eq!(
        rename_uncaptured(source, &["e", "t", "n"], &WEBPACK),
        "if (typeof module === \"object\") _module.exports = exports;"
    );
}

#[test]
fn a_swap_between_parameters_is_not_a_host_name() {
    let source = "exports.a = module;\n";
    assert_eq!(
        rename_uncaptured(source, &["exports", "module"], &["module", "exports"]),
        "module.a = exports;"
    );
}
