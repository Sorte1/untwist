mod common;

use common::scaling::assert_scales_linearly;
use untwist::{Options, Pipeline};

const FILLER: &str = "v=1;";

#[test]
fn a_constant_else_if_ladder_folds_in_one_pass() {
    assert_scales_linearly(
        "else-if ladder beside filler",
        100,
        |count| {
            format!(
                "{}{{b()}}{}",
                "if('a'!=='a'){a()}else ".repeat(count),
                FILLER.repeat(count * 10)
            )
        },
        Options::default,
    );
}

#[test]
fn a_folded_ladder_keeps_only_the_live_branch() {
    let out = Pipeline::new(Options::default())
        .run(&format!("{}{{b()}}", "if('a'!=='a'){a()}else ".repeat(50)));
    assert_eq!(out.code().trim(), "b();");
}

#[test]
fn a_method_chain_on_a_free_name_is_walked_once() {
    assert_scales_linearly(
        "a.b().b()...",
        1_000,
        |count| format!("x=a{};", ".b()".repeat(count)),
        Options::default,
    );
}

#[test]
fn a_nest_of_pure_calls_folds_in_one_pass() {
    assert_scales_linearly(
        "w(w(w(1,1),1),1) beside filler",
        50,
        |count| {
            format!(
                "function w(a,b){{return a+b}}x={}1{};{}",
                "w(".repeat(count),
                ",1)".repeat(count),
                FILLER.repeat(count * 20)
            )
        },
        Options::default,
    );
}

#[test]
fn a_folded_nest_of_pure_calls_is_one_literal() {
    let out = Pipeline::new(Options::default()).run(&format!(
        "function w(a,b){{return a+b}}x={}1{};",
        "w(".repeat(40),
        ",1)".repeat(40)
    ));
    assert_eq!(out.code().trim(), "x = 41;");
}
