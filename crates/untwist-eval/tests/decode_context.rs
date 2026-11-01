mod common;

use oxc_allocator::Allocator;
use untwist_core::parse::parse_program;
use untwist_eval::budget::Budget;
use untwist_eval::decode::{Decoded, Evaluator, InterpEvaluator};
use untwist_eval::interp::eval_script_decoding;
use untwist_eval::value::AbruptKind;
use untwist_testing::node::printed_by_node;

fn without_kind(rendered: String) -> String {
    ["string:", "number:", "boolean:"]
        .iter()
        .find_map(|kind| rendered.strip_prefix(kind))
        .map_or_else(|| rendered.clone(), str::to_string)
}

fn plain_text(value: &Decoded) -> String {
    without_kind(common::render_decoded(value))
}

fn decoded(machinery: &str, argument: &str) -> Option<String> {
    let source = format!("{machinery}\nb({argument});");
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "a", &["b"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::new(Budget::default());
        let values = evaluator.decode(&decoder_program, &calls).ok()?;
        values.first().map(plain_text)
    })
}

fn node_on_call(machinery: &str, argument: &str) -> Option<String> {
    printed_by_node(&format!("{machinery}\nconsole.log(b({argument}));"))
        .map(|text| text.trim_end_matches('\n').to_string())
}

fn decodes_as_node_or_bails(machinery: &str, argument: &str, node_prints: &str) {
    if let Some(printed) = node_on_call(machinery, argument) {
        assert_eq!(printed, node_prints, "node on the input");
    }
    if let Some(value) = decoded(machinery, argument) {
        assert_eq!(value, node_prints, "the evaluator answered differently");
    }
}

fn bails(machinery: &str, argument: &str) {
    assert_eq!(
        decoded(machinery, argument),
        None,
        "the evaluator must bail"
    );
}

fn evaluated(source: &str) -> Option<String> {
    common::eval(source).ok().map(without_kind)
}

fn evaluated_decoding(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, source);
    eval_script_decoding(&parsed.program, Budget::default(), |result| match result {
        Ok(value) => Some(without_kind(common::render(&value))),
        Err(abrupt) => match abrupt.into_kind() {
            AbruptKind::Throw(value) => Some(format!("throw {}", common::render(&value))),
            AbruptKind::Bail(_) => None,
        },
    })
}

fn node_on_script(source: &str) -> Option<String> {
    let quoted = serde_json::to_string(source).unwrap();
    printed_by_node(&format!("console.log(String((0, eval)({quoted})));"))
        .map(|text| text.trim_end_matches('\n').to_string())
}

fn agrees_in_both_contexts(source: &str, node_prints: &str) {
    if let Some(printed) = node_on_script(source) {
        assert_eq!(printed, node_prints, "node on the input");
    }
    assert_eq!(
        evaluated(source).as_deref(),
        Some(node_prints),
        "outside decoding"
    );
    if let Some(value) = evaluated_decoding(source) {
        assert_eq!(value, node_prints, "while decoding");
    }
}

const HOLE_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=['s0','s1']; h[3]='s3'; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return (i in e) + '|' + e.length; }";

#[test]
fn a_string_array_function_that_leaves_a_hole_is_refused() {
    decodes_as_node_or_bails(HOLE_IN_THE_STRING_ARRAY_FUNCTION, "2", "false|4");
    bails(HOLE_IN_THE_STRING_ARRAY_FUNCTION, "2");
}

const LITERAL_HOLE_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=['s0',,'s2']; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return (i in e) + '|' + e.length; }";

#[test]
fn a_string_array_literal_with_a_hole_is_refused() {
    decodes_as_node_or_bails(LITERAL_HOLE_IN_THE_STRING_ARRAY_FUNCTION, "1", "false|3");
    bails(LITERAL_HOLE_IN_THE_STRING_ARRAY_FUNCTION, "1");
}

const SLOPPY_THIS_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=[typeof this]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }";

#[test]
fn a_string_array_function_that_reads_a_sloppy_this_is_refused() {
    decodes_as_node_or_bails(SLOPPY_THIS_IN_THE_STRING_ARRAY_FUNCTION, "0", "object");
    bails(SLOPPY_THIS_IN_THE_STRING_ARRAY_FUNCTION, "0");
}

const DELETE_OF_A_BINDING_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var z = 1; var h=[String(delete z)]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }";

#[test]
fn a_string_array_function_that_deletes_a_binding_is_refused() {
    decodes_as_node_or_bails(
        DELETE_OF_A_BINDING_IN_THE_STRING_ARRAY_FUNCTION,
        "0",
        "false",
    );
    bails(DELETE_OF_A_BINDING_IN_THE_STRING_ARRAY_FUNCTION, "0");
}

const BOOLEAN_MEMBER_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=[typeof true.toString]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }";

#[test]
fn a_string_array_function_that_reads_a_boolean_member_is_refused() {
    decodes_as_node_or_bails(BOOLEAN_MEMBER_IN_THE_STRING_ARRAY_FUNCTION, "0", "function");
    bails(BOOLEAN_MEMBER_IN_THE_STRING_ARRAY_FUNCTION, "0");
}

const ARGUMENTS_MISS_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=[typeof arguments.callee]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }";

#[test]
fn a_string_array_function_that_misses_on_its_arguments_is_refused() {
    decodes_as_node_or_bails(ARGUMENTS_MISS_IN_THE_STRING_ARRAY_FUNCTION, "0", "function");
    bails(ARGUMENTS_MISS_IN_THE_STRING_ARRAY_FUNCTION, "0");
}

const DEAD_ZONE_READ_IN_THE_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=[x]; let x = 's0'; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i]; }";

#[test]
fn a_string_array_function_that_reads_a_dead_zone_is_refused() {
    bails(DEAD_ZONE_READ_IN_THE_STRING_ARRAY_FUNCTION, "0");
}

const PLAIN_STRING_ARRAY_FUNCTION: &str = "\
function a(){ var h=['s0', undefined, `s2`, void 0]; a=function(){return h;}; return a(); }
function b(i){ var e=a(); return e[i] + '|' + e.indexOf(undefined) + (3 in e); }";

#[test]
fn a_plain_string_array_function_still_decodes_under_the_decode_checks() {
    decodes_as_node_or_bails(PLAIN_STRING_ARRAY_FUNCTION, "2", "s2|1true");
    assert_eq!(
        decoded(PLAIN_STRING_ARRAY_FUNCTION, "2").as_deref(),
        Some("s2|1true")
    );
}

#[test]
fn a_strict_script_throws_on_an_undeclared_assignment() {
    agrees_in_both_contexts(
        "'use strict'; var r; try { undeclaredZ = 1; r = 'none'; } catch (e) { r = 'caught'; } r",
        "caught",
    );
}

#[test]
fn a_strict_function_throws_on_an_undeclared_assignment() {
    agrees_in_both_contexts(
        "(function(){ 'use strict'; try { undeclaredZ = 1; return 'none'; } catch (e) { return 'caught'; } })()",
        "caught",
    );
}

#[test]
fn a_strict_assignment_evaluates_its_right_side_before_it_throws() {
    agrees_in_both_contexts(
        "'use strict'; var s = ''; try { undeclaredZ = (s += 'r'); } catch (e) { s += 'c'; } s",
        "rc",
    );
}

#[test]
fn a_strict_for_in_head_throws_on_an_undeclared_name() {
    agrees_in_both_contexts(
        "(function(){ 'use strict'; try { for (undeclaredK in {a: 1}) {} return 'none'; } catch (e) { return 'caught'; } })()",
        "caught",
    );
}

#[test]
fn an_arrow_in_strict_code_throws_on_an_undeclared_assignment() {
    agrees_in_both_contexts(
        "(function(){ 'use strict'; return (() => { try { undeclaredA = 1; return 'none'; } catch (e) { return 'caught'; } })(); })()",
        "caught",
    );
}

#[test]
fn an_arrow_with_its_own_directive_throws_on_an_undeclared_assignment() {
    agrees_in_both_contexts(
        "(() => { 'use strict'; try { undeclaredQ = 1; return 'none'; } catch (e) { return 'caught'; } })()",
        "caught",
    );
}

#[test]
fn a_sloppy_undeclared_assignment_still_creates_the_binding() {
    agrees_in_both_contexts("undeclaredY = 5; undeclaredY + 1", "6");
}

#[test]
fn a_strict_assignment_to_a_declared_or_built_in_global_is_written() {
    agrees_in_both_contexts(
        "'use strict'; var g; (function(){ g = 2; })(); var p = parseFloat; parseFloat = parseInt; var w = parseFloat === parseInt; parseFloat = p; g + ',' + w",
        "2,true",
    );
}

#[test]
fn for_in_skips_a_key_deleted_before_it_is_reached() {
    agrees_in_both_contexts(
        "var o = {a: 1, b: 2, c: 3}, s = ''; for (var k in o) { s += k; if (k === 'a') delete o.b; } s",
        "ac",
    );
}

#[test]
fn for_in_visits_a_key_deleted_and_added_again() {
    agrees_in_both_contexts(
        "var o = {a: 1, b: 2, c: 3}, s = ''; for (var k in o) { s += k; if (k === 'a') { delete o.b; o.b = 9; } } s + Object.keys(o).join('')",
        "abcacb",
    );
}

#[test]
fn for_in_skips_a_key_added_during_the_loop() {
    agrees_in_both_contexts(
        "var o = {a: 1, b: 2}, s = ''; for (var k in o) { s += k; if (k === 'a') o.d = 4; } s",
        "ab",
    );
}

#[test]
fn for_in_visits_a_deleted_key_the_prototype_still_has() {
    agrees_in_both_contexts(
        "var o = {a: 1, toString: 2}, s = ''; for (var k in o) { s += k; if (k === 'a') delete o.toString; } s",
        "atoString",
    );
}

#[test]
fn for_in_skips_a_deleted_key_the_prototype_does_not_have() {
    agrees_in_both_contexts(
        "var o = {a: 1, map: 2}, s = ''; for (var k in o) { s += k; if (k === 'a') delete o.map; } s",
        "a",
    );
}

#[test]
fn for_in_skips_the_indices_an_array_loses() {
    agrees_in_both_contexts(
        "var a = [1, 2, 3], s = ''; for (var k in a) { s += k; a.length = 1; } var b = [1, 2, 3]; for (var j in b) { s += j; b.pop(); } var c = ['x', 'y', 'z']; for (var i in c) { s += i; if (i === '0') c.splice(0, 2); } s",
        "0010",
    );
}

#[test]
fn for_in_over_a_string_visits_every_index() {
    agrees_in_both_contexts("var s = ''; for (var k in 'abc') { s += k; } s", "012");
}

#[test]
fn for_in_with_a_member_head_checks_the_key_before_it_assigns() {
    agrees_in_both_contexts(
        "var o = {a: 1, b: 2, c: 3}, t = {}, s = ''; for (t.k in o) { s += t.k; if (t.k === 'a') delete o.b; } s",
        "ac",
    );
}

#[test]
fn object_is_a_function_that_returns_its_object_argument() {
    agrees_in_both_contexts(
        "var o = {a: 1}; [typeof Object, Object(o) === o, new Object(o) === o, typeof Object(null), Object.keys(Object()).length].join()",
        "function,true,true,object,0",
    );
}

#[test]
fn typeof_a_host_global_object_is_refused_while_decoding() {
    for name in ["window", "self", "global", "frames"] {
        assert_eq!(
            evaluated_decoding(&format!("typeof {name}")),
            None,
            "{name}"
        );
    }
}

#[test]
fn a_native_whose_coercion_runs_user_code_is_refused_while_decoding() {
    let programs = [
        "'abc'.indexOf('b', { valueOf: function () { return 1; } })",
        "String({ toString: function () { return 'x'; } })",
        "[1, 2].join({ toString: function () { return '-'; } })",
        "'abc'.slice({ valueOf: function () { return 1; } })",
    ];
    for program in programs {
        assert_eq!(evaluated_decoding(program), None, "{program}");
        assert!(evaluated(program).is_some(), "{program}");
    }
}

#[test]
fn split_coerces_in_spec_order_and_still_answers_while_decoding() {
    agrees_in_both_contexts(
        "var log = ''; var r = 'a-b'.split({ toString: function () { log += 's'; return '-'; } }, { valueOf: function () { log += 'l'; return 5; } }); log + r.join('|')",
        "lsa|b",
    );
}

#[test]
fn the_source_text_of_a_method_is_refused() {
    let programs = [
        "({ m() { return 1; } }).m.toString()",
        "String(({ *g() {} }).g)",
        "Function.prototype.toString.call(({ async a() {} }).a)",
    ];
    for program in programs {
        assert_eq!(evaluated_decoding(program), None, "{program}");
        assert_eq!(evaluated(program), None, "{program}");
    }
    agrees_in_both_contexts(
        "String(function f(a) { return a; }) + String(async function () {})",
        "function f(a) { return a; }async function () {}",
    );
}
