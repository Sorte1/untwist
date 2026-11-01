mod common;

use common::{assert_scope_safe, rename_full};
use untwist_rename::{RenameConfig, RenameMode};

fn shorthand(source: &str, mode: RenameMode) -> String {
    let config = RenameConfig {
        mode,
        rename_all: false,
        extended: true,
    };
    let output = rename_full(source, config);
    assert_scope_safe(source, &output);
    output
}

fn stable_shorthand(source: &str) -> String {
    let once = shorthand(source, RenameMode::None);
    assert_eq!(shorthand(&once, RenameMode::None), once);
    once
}

#[test]
fn a_renamed_destructuring_alias_collapses_to_shorthand() {
    assert_eq!(
        shorthand(
            "let { sin: k, cos: v, PI: A } = Math; k(A); v(A);",
            RenameMode::None
        ),
        "let{sin,cos,PI}=Math;sin(PI);cos(PI);"
    );
}

#[test]
fn an_aliased_import_specifier_collapses_to_shorthand() {
    let output = shorthand(r#"import { sin as k } from "m"; k(1);"#, RenameMode::None);
    assert!(output.contains("{sin}"), "{output}");
    assert!(output.ends_with("sin(1);"), "{output}");
    assert!(!output.contains(" as "), "{output}");
}

#[test]
fn an_object_literal_value_takes_its_key() {
    assert_eq!(
        shorthand("var t = 1; send({ videoRef: t });", RenameMode::None),
        "var videoRef=1;send({videoRef});"
    );
}

#[test]
fn a_key_that_is_not_free_leaves_the_alias_alone() {
    assert_eq!(
        shorthand(
            "var sin = 0; let { sin: k } = Math; k(sin);",
            RenameMode::None
        ),
        "var sin=0;let{sin:k}=Math;k(sin);"
    );
}

#[test]
fn a_key_that_is_not_an_identifier_is_never_used() {
    let output = shorthand(r#"import { "a-b" as k } from "m"; k();"#, RenameMode::None);
    assert!(output.contains(r#""a-b"as k"#), "{output}");
    assert!(output.ends_with("k();"), "{output}");
}

#[test]
fn exported_bindings_are_never_moved_toward_a_key() {
    assert_eq!(
        shorthand("let { sin: k } = Math; export { k };", RenameMode::None),
        "let{sin:k}=Math;export{k};"
    );
}

#[test]
fn it_runs_after_every_mode() {
    for mode in [RenameMode::Unique, RenameMode::Simple, RenameMode::Mangle] {
        assert_eq!(
            shorthand("let { sin: _0xk } = Math; _0xk(1);", mode),
            "let{sin}=Math;sin(1);",
            "{mode:?}"
        );
    }
}

#[test]
fn mangle_then_shorthand_restores_every_legal_form() {
    let output = shorthand(
        r#"import { sin as _0xk } from "m"; const { cos: _0xc } = Math; const _0xt = 1; send({ videoRef: _0xt }); _0xk(_0xc(_0xt));"#,
        RenameMode::Mangle,
    );
    assert!(output.contains(r#"import{sin}from"m""#), "{output}");
    assert!(output.contains("{cos}=Math"), "{output}");
    assert!(output.contains("send({videoRef})"), "{output}");
    assert!(!output.contains(" as "), "{output}");
}

#[test]
fn a_key_another_binding_still_holds_is_left_alone() {
    assert_eq!(
        shorthand(
            "let cos = 1; let { sin: k } = Math; let { cos: sin } = Math; k(); sin(); cos();",
            RenameMode::None
        ),
        "let cos=1;let{sin:k}=Math;let{cos:sin}=Math;k();sin();cos();"
    );
    assert_eq!(
        shorthand(
            "let { sin } = Math; let { sin: k } = obj; sin(); k();",
            RenameMode::None
        ),
        "let{sin}=Math;let{sin:k}=obj;sin();k();"
    );
}

#[test]
fn running_the_shorthand_stage_twice_changes_nothing() {
    assert_eq!(
        stable_shorthand("let { sin: k, cos: v } = Math; k(); v();"),
        "let{sin,cos}=Math;sin();cos();"
    );
}

#[test]
fn a_symbol_already_on_its_key_is_not_moved_off_it() {
    assert_eq!(
        stable_shorthand("let { a } = o; use({ a }); send({ b: a });"),
        "let{a}=o;use({a});send({b:a});"
    );
}

#[test]
fn a_destructuring_cascade_collapses_fully_in_one_stage_run() {
    assert_eq!(
        stable_shorthand("let { sin: k } = Math; let { cos: sin } = Math; k(); sin();"),
        "let{sin}=Math;let{cos}=Math;sin();cos();"
    );
}

#[test]
fn an_import_cascade_collapses_fully_in_one_stage_run() {
    assert_eq!(
        stable_shorthand(r#"import { b as c, a as b } from "m"; c(); b();"#),
        r#"import{b,a}from"m";b();a();"#
    );
}
