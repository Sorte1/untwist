mod common;

use common::rename_full;
use untwist_rename::{RenameConfig, RenameMode};

const MODES: [RenameMode; 4] = [
    RenameMode::Mangle,
    RenameMode::Simple,
    RenameMode::Unique,
    RenameMode::Smart,
];

fn tokens(text: &str, token: &str) -> usize {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
        .filter(|word| *word == token)
        .count()
}

fn renamed(source: &str, mode: RenameMode) -> String {
    rename_full(
        source,
        RenameConfig {
            mode,
            rename_all: true,
            extended: true,
        },
    )
}

#[test]
fn the_var_a_sloppy_block_function_writes_keeps_the_function_name() {
    let source = "function g() { var f = 1; { function f() { return 2; } } return typeof f; }";
    for mode in MODES {
        let output = renamed(source, mode);
        assert!(output.contains("var f=1"), "{mode:?}: {output}");
        assert!(output.contains("return typeof f}"), "{mode:?}: {output}");
    }
}

#[test]
fn a_machinery_named_parameter_keeps_its_name_and_no_other_binding_takes_it() {
    let source =
        "function f(__DECODE_0__, s) { return [{ foo: __DECODE_0__ }, { __DECODE_0__: s }]; }";
    for mode in MODES {
        let output = renamed(source, mode);
        assert_eq!(tokens(&output, "__DECODE_0__"), 3, "{mode:?}: {output}");
    }
}

#[test]
fn a_catch_parameter_never_takes_the_name_of_a_var_in_its_body() {
    let source = "function f(o) { try { throw o; } catch (e) { var k = 5; var r = { c: e }; } return [r, k]; }";
    for mode in MODES {
        let output = renamed(source, mode);
        let catch_name = output
            .split("catch (")
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .unwrap_or_default()
            .to_string();
        assert!(
            !output.contains(&format!("var {catch_name}=")),
            "{mode:?}: {output}"
        );
    }
}

#[test]
fn a_block_let_never_shares_a_name_with_a_var_written_beside_it() {
    let source = "function f() { { let x = 2; var a = 7; console.log(x); } return a; }";
    for mode in MODES {
        let output = renamed(source, mode);
        let let_name = output
            .split("let ")
            .nth(1)
            .and_then(|rest| rest.split('=').next())
            .unwrap_or_default()
            .to_string();
        assert!(
            !output.contains(&format!("var {let_name}=")),
            "{mode:?}: {output}"
        );
    }
}

#[test]
fn a_name_read_pins_every_binding_that_names_a_function_or_class() {
    let source =
        "function foo() {} var bar = function () {}; class Baz {} var later; later = () => 1; \
                  console.log(foo.name, bar.name, Baz.name, later.name);";
    for mode in MODES {
        let output = renamed(source, mode);
        for kept in ["function foo()", "var bar=", "class Baz", "later=()=>"] {
            assert!(output.contains(kept), "{mode:?}: {kept}: {output}");
        }
    }
}

#[test]
fn a_name_read_on_an_object_literal_pins_nothing() {
    let source = "function foo() {} console.log(({ name: 1 }).name, foo());";
    for mode in MODES {
        let output = renamed(source, mode);
        assert!(!output.contains("function foo()"), "{mode:?}: {output}");
    }
}

#[test]
fn writing_name_pins_nothing() {
    let source = "function foo() {} var o = {}; o.name = 1; console.log(o, foo());";
    for mode in MODES {
        let output = renamed(source, mode);
        assert!(!output.contains("function foo()"), "{mode:?}: {output}");
    }
}
