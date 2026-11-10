mod common;

use common::scaling::assert_scales_linearly;
use untwist::{Options, Pipeline};

const HEAD: &str = "function sa() { var h = ['log', 'hello', 'world']; sa = function () { return h; }; return sa(); }\n\
function dec(c, d) { var e = sa(); return (dec = function (f, g) { f = f - 0x0; var i = e[f];\n\
  return String.fromCharCode(i.charCodeAt(0)) + i.slice(1); }), dec(c, d); }\n";

fn reversed(count: usize, end: &str) -> String {
    let links: String = (0..count)
        .map(|index| format!("var a{index}=a{};", index + 1))
        .collect();
    format!("{HEAD}{links}var a{count}={end};console.log(dec(0x1));")
}

fn reversed_functions(count: usize) -> String {
    let links: String = (0..count)
        .map(|index| format!("function f{index}(){{return f{}()}}", index + 1))
        .collect();
    format!(
        "{HEAD}{links}function f{count}(){{return Object.defineProperty}}console.log(dec(0x1));"
    )
}

#[test]
fn an_alias_chain_written_before_its_source_is_solved_without_a_sweep_per_link() {
    assert_scales_linearly(
        "var a0=a1; ... var aN=Object;",
        1_000,
        |count| reversed(count, "Object"),
        Options::default,
    );
}

#[test]
fn a_string_chain_written_before_its_source_is_solved_without_a_sweep_per_link() {
    assert_scales_linearly(
        "var a0=a1; ... var aN='x';",
        1_000,
        |count| reversed(count, "'x'"),
        Options::default,
    );
}

#[test]
fn a_function_chain_returning_a_builtin_is_solved_without_a_sweep_per_link() {
    assert_scales_linearly(
        "function f0(){return f1()} ...",
        500,
        reversed_functions,
        Options::default,
    );
}

#[test]
fn a_decoder_beside_a_reversed_alias_chain_still_decodes() {
    let out = Pipeline::new(Options::default()).run(&reversed(50, "'x'"));
    assert!(out.code().contains("\"hello\""), "{}", out.code());
}
