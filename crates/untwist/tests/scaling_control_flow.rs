mod common;

use common::scaling::assert_scales_linearly;
use untwist::{Options, Pipeline};

fn declared_nest(depth: usize) -> String {
    format!(
        "var o={{abcde:function(a,b){{return a+b}}}};x={}1{};",
        "o.abcde(".repeat(depth),
        ",1)".repeat(depth)
    )
}

fn literal_nest(depth: usize) -> String {
    format!(
        "x={}1{};",
        "({abcde:function(a,b){return a+b}}).abcde(".repeat(depth),
        ",1)".repeat(depth)
    )
}

#[test]
fn a_deep_nest_of_declared_control_flow_calls_is_not_copied_per_level() {
    assert_scales_linearly(
        "o.abcde(o.abcde(...))",
        300,
        declared_nest,
        Options::default,
    );
}

#[test]
fn a_deep_nest_of_literal_control_flow_calls_is_not_copied_per_level() {
    assert_scales_linearly(
        "({..}).abcde(({..}).abcde(...))",
        300,
        literal_nest,
        Options::default,
    );
}

#[test]
fn a_shallow_nest_of_control_flow_calls_is_still_inlined() {
    let out = Pipeline::new(Options::default()).run(&declared_nest(20));
    assert!(!out.code().contains("abcde"), "{}", out.code());
}
