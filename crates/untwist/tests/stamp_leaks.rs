use std::path::{Path, PathBuf};

use untwist::{Options, Pipeline};
use untwist_testing::node::{node, run, skip_allowed, SKIP_MESSAGE};

const STAMPS: [&str; 2] = ["__STRING_ARRAY_", "__DECODE_"];

struct Row {
    source: &'static str,
    prints: &'static str,
    kept: &'static [&'static str],
    stamps: &'static [&'static str],
}

fn modes() -> [Options; 2] {
    [
        Options::default(),
        Options {
            extended: false,
            unsafe_transforms: true,
            ..Options::default()
        },
    ]
}

fn node_or_skip() -> Option<PathBuf> {
    let found = node();
    if found.is_none() {
        let required = std::env::var("UNTWIST_REQUIRE_NODE").ok();
        assert!(
            skip_allowed(required.as_deref()),
            "required: {SKIP_MESSAGE}"
        );
        eprintln!("{SKIP_MESSAGE}");
    }
    found
}

fn printed(node: &Path, text: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("program.js");
    std::fs::write(&path, text).unwrap();
    run(node, &path)
        .unwrap()
        .map(|out| out.stdout)
        .unwrap_or_default()
}

fn check(row: &Row) {
    let node = node_or_skip();
    if let Some(node) = &node {
        assert_eq!(printed(node, row.source), row.prints, "node on the input");
    }
    for options in modes() {
        let code = Pipeline::new(options).run(row.source).code().to_string();
        for stamp in row.stamps {
            assert!(!code.contains(stamp), "a stamp name is left: {code}");
        }
        for name in row.kept {
            assert!(
                code.contains(name),
                "the original name must be back: {code}"
            );
        }
        if let Some(node) = &node {
            assert_eq!(
                printed(node, &code),
                row.prints,
                "node on the output: {code}"
            );
        }
    }
}

fn row(source: &'static str, prints: &'static str) -> Row {
    Row {
        source,
        prints,
        kept: &["function a(", "function b("],
        stamps: &STAMPS,
    }
}

const DECODER_MEMO_ON_A_SAME_NAMED_PROPERTY: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
console.log(c(0)); console.log(c(1));"#;

#[test]
fn a_decoder_whose_memo_property_spells_its_name_decodes_without_a_stamp_left() {
    check(&Row {
        kept: &[],
        ..row(DECODER_MEMO_ON_A_SAME_NAMED_PROPERTY, "s0\ns1\n")
    });
}

const DECODER_NAME_SPELLED_BY_A_STRING: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), 'b'); console.log(b(1));"#;

#[test]
fn a_decoder_spelled_by_a_string_decodes_without_a_stamp_left() {
    check(&Row {
        kept: &[],
        ..row(DECODER_NAME_SPELLED_BY_A_STRING, "s0 b\ns1\n")
    });
}

const DECODER_SPELLED_UNDER_A_DIRECT_EVAL: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), eval('typeof b')); console.log(b(1));"#;

#[test]
fn a_decoder_spelled_under_a_direct_eval_keeps_the_arrays_name() {
    check(&row(
        DECODER_SPELLED_UNDER_A_DIRECT_EVAL,
        "s0 function\ns1\n",
    ));
}

const DECODER_NAME_SPELLED_BY_A_SHORTHAND: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
var o={b};
console.log(b(0), typeof o.b); console.log(b(1));"#;

#[test]
fn a_decoder_spelled_by_a_shorthand_property_keeps_the_arrays_name() {
    check(&row(
        DECODER_NAME_SPELLED_BY_A_SHORTHAND,
        "s0 function\ns1\n",
    ));
}

const DECODER_NAME_SPELLED_BY_AN_ARRAY_STRING: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
var k=['b'];
console.log(b(0), k[0]); console.log(b(1));"#;

#[test]
fn a_decoder_spelled_by_an_array_element_decodes_without_a_stamp_left() {
    check(&Row {
        kept: &[],
        ..row(DECODER_NAME_SPELLED_BY_AN_ARRAY_STRING, "s0 b\ns1\n")
    });
}

const DECODER_WRITTEN_OUTSIDE: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0)); console.log(b(1));
if (Math.random() > 2) { b = null; }"#;

#[test]
fn a_decoder_written_outside_its_statement_keeps_the_arrays_name() {
    check(&row(DECODER_WRITTEN_OUTSIDE, "s0\ns1\n"));
}

const DECODER_REDECLARED: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
var b;
console.log(b(0)); console.log(b(1));"#;

#[test]
fn a_redeclared_decoder_keeps_the_arrays_name() {
    check(&row(DECODER_REDECLARED, "s0\ns1\n"));
}

const DECODER_INDEXING_THE_ARRAY_CALL: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ return a()[i]; }
console.log(b(0)); console.log(b(1));"#;

#[test]
fn a_decoder_indexing_the_array_call_directly_decodes_without_a_stamp_left() {
    check(&Row {
        kept: &[],
        ..row(DECODER_INDEXING_THE_ARRAY_CALL, "s0\ns1\n")
    });
}

const SECOND_UNITS_DECODER_SPELLED: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
function x(){ var h=['t0','t1','t2']; x=function(){return h;}; return x(); }
function y(i){ var e=x(); return e[i]; }
console.log(b(0), y(1), 'y'); console.log(b(2));"#;

#[test]
fn a_second_units_spelled_decoder_decodes_beside_a_decoded_unit() {
    check(&Row {
        kept: &[],
        ..row(SECOND_UNITS_DECODER_SPELLED, "s0 t1 y\ns2\n")
    });
}

const ROTATED_UNITS_DECODER_SPELLED: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
(function(g, n){ var arr = g(); while (n--) { arr.push(arr.shift()); } })(a, 1);
function b(i){ var e=a(); return e[i]; }
var o = {g: 1, n: 2, arr: 3};
console.log(b(0), o.g); console.log(b(1), 'b');"#;

#[test]
fn a_rotated_units_spelled_decoder_keeps_the_arrays_name() {
    check(&row(ROTATED_UNITS_DECODER_SPELLED, "s1 1\ns2 b\n"));
}

const A_MACHINERY_NAMED_BINDING_IN_THE_INPUT: &str = r#"var __DECODE_5__ = 7;
function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), __DECODE_5__); console.log(b(1));"#;

#[test]
fn a_machinery_named_binding_in_the_input_leaves_no_stamp_of_untwists_own() {
    check(&Row {
        kept: &["function a(", "function b(", "var __DECODE_5__ = 7"],
        stamps: &["__STRING_ARRAY_", "__DECODE_0__"],
        ..row(A_MACHINERY_NAMED_BINDING_IN_THE_INPUT, "s0 7\ns1\n")
    });
}

const SPELLED_DECODER_BESIDE_A_LOCAL_OF_THE_ARRAYS_NAME: &str = r#"function f(){ var a = 1; return a; }
function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
console.log(b(0), 'b', f()); console.log(b(1));"#;

#[test]
fn a_spelled_decoder_decodes_when_a_local_elsewhere_shares_the_arrays_name() {
    check(&Row {
        kept: &[],
        ..row(
            SPELLED_DECODER_BESIDE_A_LOCAL_OF_THE_ARRAYS_NAME,
            "s0 b 1\ns1\n",
        )
    });
}

const BAILED_UNIT_BESIDE_LOCALS_OF_ITS_NAMES: &str = r#"var G = 'g';
function f(){ var a = 1; var b = 2; return a + b; }
function a(){ var h=['p','q','r','s']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + G; }
console.log(b(0), f()); console.log(b(1));"#;

#[test]
fn a_bailed_unit_gets_its_names_back_when_locals_elsewhere_share_them() {
    check(&row(BAILED_UNIT_BESIDE_LOCALS_OF_ITS_NAMES, "pg 3\nqg\n"));
}

const TWO_BAILED_UNITS_OF_ONE_NAME_IN_SIBLING_SCOPES: &str = r#"var G = 'g';
(function(){
function a(){ var h=['p','q']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + G; }
console.log(b(0));
})();
(function(){
function a(){ var h=['r','s']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + G; }
console.log(b(1));
})();"#;

#[test]
fn two_bailed_units_of_one_name_in_sibling_scopes_both_get_their_names_back() {
    check(&row(
        TWO_BAILED_UNITS_OF_ONE_NAME_IN_SIBLING_SCOPES,
        "pg\nsg\n",
    ));
}

const SPELLED_DECODER_IN_A_SCOPE_WITH_A_WITH: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
var o = {};
with (o) { console.log(b(0), 'b'); }
console.log(b(1));"#;

#[test]
fn a_unit_whose_scope_holds_a_with_statement_is_never_stamped() {
    check(&row(SPELLED_DECODER_IN_A_SCOPE_WITH_A_WITH, "s0 b\ns1\n"));
}

const SPELLED_DECODER_IN_A_SCOPE_WITH_A_DIRECT_EVAL: &str = r#"function a(){ var h=['s0','s1','s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
function f(){ return eval('1'); }
console.log(b(0), 'b', f()); console.log(b(1));"#;

#[test]
fn a_unit_whose_scope_holds_a_direct_eval_is_never_stamped() {
    check(&row(
        SPELLED_DECODER_IN_A_SCOPE_WITH_A_DIRECT_EVAL,
        "s0 b 1\ns1\n",
    ));
}

fn memo_row(source: &'static str, prints: &'static str) -> Row {
    Row {
        kept: &["function a(", "function c("],
        ..row(source, prints)
    }
}

const EVAL_NAMES_THE_ARRAY_BY_A_COMPUTED_STRING: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
console.log(c(0)); console.log(eval('typeof ' + String.fromCharCode(97)));"#;

#[test]
fn a_direct_eval_that_computes_the_arrays_name_sees_the_input_program() {
    check(&memo_row(
        EVAL_NAMES_THE_ARRAY_BY_A_COMPUTED_STRING,
        "s0\nfunction\n",
    ));
}

const EVAL_IN_THE_UNITS_FUNCTION: &str = r#"function outer(){ function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
 return c(0) + eval('typeof ' + String.fromCharCode(97)); }
console.log(outer());"#;

#[test]
fn a_direct_eval_in_the_units_own_function_sees_the_input_program() {
    check(&memo_row(EVAL_IN_THE_UNITS_FUNCTION, "s0function\n"));
}

const WITH_OBJECT_KEYED_BY_A_COMPUTED_STRING: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
var o = {}; o[String.fromCharCode(97)] = function () { return ['w0']; };
with (o) { console.log(a()[0]); }
console.log(c(0));"#;

#[test]
fn a_with_whose_object_shadows_the_array_by_a_computed_key_sees_the_input_program() {
    check(&memo_row(
        WITH_OBJECT_KEYED_BY_A_COMPUTED_STRING,
        "w0\ns0\n",
    ));
}

const EVAL_IN_A_DESCENDANT_FUNCTION: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
function g(){ return eval('c.name') + ' ' + eval('typeof ' + String.fromCharCode(97)); }
console.log(c(1)); console.log(g());"#;

#[test]
fn a_direct_eval_in_a_descendant_function_sees_the_input_program() {
    check(&memo_row(EVAL_IN_A_DESCENDANT_FUNCTION, "s1\nc function\n"));
}

const EVAL_IN_THE_ENCLOSING_FUNCTION: &str = r#"function outer(){ eval('1'); function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
 return c(0) + typeof a; }
console.log(outer());"#;

#[test]
fn a_unit_beside_a_direct_eval_in_its_function_leaves_no_stamp() {
    check(&memo_row(EVAL_IN_THE_ENCLOSING_FUNCTION, "s0function\n"));
}

const EVAL_OF_A_PARAMETER_IN_THE_ENCLOSING_FUNCTION: &str = r#"function outer(s){ eval(s); function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
 return c(0) + typeof a; }
console.log(outer('var q = 1'));"#;

#[test]
fn a_unit_beside_an_eval_of_a_parameter_leaves_no_stamp() {
    check(&memo_row(
        EVAL_OF_A_PARAMETER_IN_THE_ENCLOSING_FUNCTION,
        "s0function\n",
    ));
}

const MEMO_FILLED_BY_A_DIRECT_EVAL: &str = r#"function a() { var h = ["s0", "s1", "s2"]; a = function () { return h; }; return a(); }
function b(f, g) {
  var e = a();
  return (b = function (f, g) {
    if (b.g === undefined) { b.c = {}; b.g = true; }
    if (b.c[f] === undefined) { b.c[f] = e[f] + g; }
    return b.c[f];
  }), b(f, g);
}
eval("b(0, \"y\")");
console.log(b(0, "x")); console.log(b(1, "x"));"#;

#[test]
fn a_memo_filled_through_a_direct_eval_leaves_no_stamp() {
    check(&row(MEMO_FILLED_BY_A_DIRECT_EVAL, "s0y\ns1x\n"));
}

const CATCH_PARAMETER_REDECLARED_BY_A_VAR: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
try { throw 1; } catch (a) { var a = 2; }
console.log(c(0), a.name);"#;

#[test]
fn a_catch_parameter_redeclared_by_a_var_does_not_keep_the_stamp() {
    check(&memo_row(CATCH_PARAMETER_REDECLARED_BY_A_VAR, "s0 a\n"));
}

const DEFAULT_PARAMETER_READS_THE_ARRAY: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
function f(x = a) { var a = 1; return x.name; }
console.log(c(0), f());"#;

#[test]
fn a_default_parameter_reading_the_array_does_not_keep_the_stamp() {
    check(&memo_row(DEFAULT_PARAMETER_READS_THE_ARRAY, "s0 a\n"));
}

const DEFAULT_PARAMETER_BESIDE_A_STATELESS_UNIT: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
function f(x = a) { var a = 1; return x.name; }
console.log(b(0), b(1), f());"#;

#[test]
fn a_default_parameter_beside_a_stateless_unit_does_not_keep_the_stamp() {
    check(&row(DEFAULT_PARAMETER_BESIDE_A_STATELESS_UNIT, "s0 s1 a\n"));
}

const CATCH_PARAMETER_REDECLARED_TYPEOF: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
try { throw 1; } catch (a) { var a = 2; }
console.log(c(0), typeof a, c.name);"#;

#[test]
fn a_catch_redeclaration_beside_a_refused_unit_leaves_no_stamp() {
    check(&memo_row(
        CATCH_PARAMETER_REDECLARED_TYPEOF,
        "s0 function c\n",
    ));
}

const DEFAULT_PARAMETER_TYPEOF: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function c(i){ var e=a(); if (c.c===undefined){ c.c={}; } if (c.c[i]===undefined){ c.c[i]=e[i]; } return c.c[i]; }
function f(x = a) { var a = 1; return typeof x + a; }
console.log(c(0), f(), c.name);"#;

#[test]
fn a_default_parameter_beside_a_refused_unit_leaves_no_stamp() {
    check(&memo_row(DEFAULT_PARAMETER_TYPEOF, "s0 function1 c\n"));
}

const DEFAULT_PARAMETER_TYPEOF_BESIDE_A_STATELESS_UNIT: &str = r#"function a(){ var h=["s0","s1","s2"]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }
function f(x = a) { var a = 1; return typeof x + a; }
console.log(b(0), b(1), f());"#;

#[test]
fn a_default_parameters_typeof_beside_a_stateless_unit_leaves_no_stamp() {
    check(&row(
        DEFAULT_PARAMETER_TYPEOF_BESIDE_A_STATELESS_UNIT,
        "s0 s1 function1\n",
    ));
}
