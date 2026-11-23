mod common;

use common::parse;
use oxc_allocator::Allocator;
use untwist_core::print::{print, PrintStyle};
use untwist_unpack::rename::rename_binding;
use untwist_unpack::rename::{
    clear_rename_hazard, rename_parameters, rename_parameters_uncaptured, took_rename_hazard,
};

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
        "let counter = class f {};\nlet _counter = 1;\ncounter = 2;"
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

fn rename_refusal(source: &str, original: &[&str], renamed: &[&str]) -> Option<&'static str> {
    let alloc = Allocator::default();
    let mut program = parse(&alloc, source);
    let before = print(&program, PrintStyle::Pretty);
    clear_rename_hazard();
    let changes = rename_parameters(&alloc, &mut program, original, renamed);
    let refused = took_rename_hazard();
    if refused.is_some() {
        assert_eq!(changes, 0, "{source}");
        assert_eq!(print(&program, PrintStyle::Pretty), before, "{source}");
    }
    refused
}

#[test]
fn every_binding_named_like_the_target_moves_aside_not_only_the_nearest() {
    let source = "function f(require) {\n  return function (require) {\n    return n(0).v + require;\n  }(2) + require;\n}\n";
    assert_eq!(
        rename(source, &["e", "t", "n"], &WEBPACK),
        "function f(_require) {\n  return function (_require2) {\n    return require(0).v + _require2;\n  }(2) + _require;\n}"
    );
}

#[test]
fn a_binding_off_every_reference_path_moves_aside_too() {
    let source = "function g(module) {\n  return module;\n}\nn(0);\nfunction h() {\n  var require = 1;\n  return require;\n}\n";
    assert_eq!(
        rename(source, &["e", "t", "n"], &WEBPACK),
        "function g(module) {\n  return module;\n}\nrequire(0);\nfunction h() {\n  var _require = 1;\n  return _require;\n}"
    );
}

#[test]
fn a_catch_parameter_in_a_var_scope_binding_moves_aside_with_it() {
    let source = "function f() {\n  var require = 1;\n  try {\n    throw 2;\n  } catch (require) {\n    return n(0).v + require;\n  }\n}\n";
    assert_eq!(
        rename(source, &["e", "t", "n"], &WEBPACK),
        "function f() {\n  var _require = 1;\n  try {\n    throw 2;\n  } catch (_require2) {\n    return require(0).v + _require2;\n  }\n}"
    );
}

#[test]
fn a_catch_parameter_beside_a_same_named_block_function_refuses_the_rename() {
    for (source, original, renamed) in [
        (
            "function f() { try {} catch (require) { { function require() {} n(0); } } }",
            &["e", "t", "n"][..],
            &WEBPACK[..],
        ),
        (
            "function f() { try {} catch (require) { { function require() {} } n(0); } }",
            &["e", "t", "n"][..],
            &WEBPACK[..],
        ),
        (
            "function f() { try {} catch (exports) { { function exports() {} } c.k = 1; } }",
            &["a", "b", "c"][..],
            &BROWSERIFY[..],
        ),
    ] {
        assert!(
            rename_refusal(source, original, renamed).is_some(),
            "{source}"
        );
    }
}

#[test]
fn a_direct_eval_or_with_statement_near_the_target_refuses_the_rename() {
    for source in [
        "function f(require) { eval(s); return require; } n(0);",
        "with (o) { n(0); }",
        "function f() { var require = 1; with (o) { return require; } } n(0);",
        "function f() { eval(s); return n(0); }",
    ] {
        assert!(
            rename_refusal(source, &["e", "t", "n"], &WEBPACK).is_some(),
            "{source}"
        );
    }
}

#[test]
fn a_direct_eval_through_a_local_binding_named_eval_refuses_the_rename() {
    for source in [
        "function g(s, require) { var eval = globalThis.eval; return eval(s); } n(0);",
        "function g(s, require) { var { eval } = globalThis; return eval(s); } n(0);",
        "function g(s, require) { let eval = globalThis.eval; return eval(s); } n(0);",
        "var eval = globalThis.eval; function g(s, require) { return eval(s); } n(0);",
        "function g(s, require) { try { throw globalThis.eval; } catch (eval) { return eval(s); } } n(0);",
        "function g(eval, s, require) { return eval(s); } n(0);",
        "function g(s, require) { return (function (eval) { return eval(s); })(globalThis.eval); } n(0);",
        "function g(s) { var eval = globalThis.eval; eval(s); return n(0); }",
    ] {
        assert!(
            rename_refusal(source, &["e", "t", "n"], &WEBPACK).is_some(),
            "{source}"
        );
    }
}

#[test]
fn a_rename_without_a_collision_or_hazard_is_not_refused() {
    for source in [
        "n(0);",
        "function f(require) { return require; } n(0);",
        "function f() { try {} catch (require) { { function g() {} } n(0); } }",
        "function f(require) { eval(s); return require; }",
        "function f() { eval(s); } n(0);",
        "function f() { eval(\"Math;\"); return n(0); }",
    ] {
        assert_eq!(
            rename_refusal(source, &["e", "t", "n"], &WEBPACK),
            None,
            "{source}"
        );
    }
}

#[test]
fn metro_parameters_move_every_colliding_binding_aside() {
    let canonical = [
        "global",
        "require",
        "_$$_IMPORT_DEFAULT",
        "_$$_IMPORT_ALL",
        "module",
        "exports",
        "_dependencyMap",
    ];
    let source = "function f(require) {\n  return function (require) {\n    return r(d[0]) + require;\n  }(2) + require;\n}\n";
    assert_eq!(
        rename(source, &["g", "r", "i", "a", "m", "e", "d"], &canonical),
        "function f(_require) {\n  return function (_require2) {\n    return require(_dependencyMap[0]) + _require2;\n  }(2) + _require;\n}"
    );
}

#[test]
fn a_renamed_parameter_in_a_shorthand_assignment_target_keeps_its_key() {
    let source = "function f(e, t) { ({e, t = 1} = o); return e + t; }";
    assert_eq!(
        rename(source, &["e", "t"], &["module", "exports"]),
        "function f(e, t) {\n  ({e, t = 1} = o);\n  return e + t;\n}"
    );
    let source = "({e, t = 1} = o);\nconsole.log(e, t);";
    assert_eq!(
        rename(source, &["e", "t"], &["module", "exports"]),
        "({e: module, t: exports = 1} = o);\nconsole.log(module, exports);"
    );
}

#[test]
fn a_renamed_parameter_in_a_shorthand_proto_property_keeps_its_own_key() {
    let source = "const o = {__proto__, t};\nconsole.log(o, __proto__);";
    assert_eq!(
        rename(source, &["__proto__", "t"], &["exports", "module"]),
        "const o = {\n  [\"__proto__\"]: exports,\n  t: module\n};\nconsole.log(o, exports);"
    );
}
