mod common;

use common::{eval, eval_with};
use untwist_eval::budget::Budget;

#[test]
fn the_self_defending_state_machine_terminates() {
    let source = "\
var C = function (fn) {
  this['hjROCd'] = fn;
  this['EwIbIj'] = [1, 0, 1];
  this['XvCZzf'] = function(){return 'newState';};
  this['yBIPgR'] = '\\\\w+ *\\\\(\\\\) *{\\\\w+ *';
  this['RuITBF'] = \"['|\\\"].+['|\\\"];? *}\";
};
C['prototype']['tlFPzk'] = function () {
  var re = new RegExp(this['yBIPgR'] + this['RuITBF']);
  var v = re['test'](this['XvCZzf']['toString']()) ? --this['EwIbIj'][1] : --this['EwIbIj'][0];
  return this['TfsNRQ'](v);
};
C['prototype']['TfsNRQ'] = function (v) {
  if (!Boolean(~v)) return v;
  return this['naXjRo'](this['hjROCd']);
};
C['prototype']['naXjRo'] = function (fn) {
  for (var i = 0, len = this['EwIbIj']['length']; i < len; i++) {
    this['EwIbIj']['push'](Math['round'](Math['random']()));
    len = this['EwIbIj']['length'];
  }
  return fn(this['EwIbIj'][0]);
};
new C(function (x) { return x; })['tlFPzk']()";
    assert_eq!(eval(source), Ok("number:-1".to_string()));
}

#[test]
fn the_debug_protection_helper_bails_instead_of_evaluating_a_string() {
    let source = "(function () { return true; })['constructor']('debu' + 'gger')['call']('action')";
    assert_eq!(
        eval(source),
        Err("hostile global Function reached".to_string())
    );
}

#[test]
fn the_global_variable_helper_bails_rather_than_reaching_a_host_object() {
    let source = "\
var getGlobal = function () {
  var g;
  try { g = Function('return (function() ' + '{}.constructor(\"return this\")( )' + ')')(); }
  catch (e) { g = window; }
  return g;
};
getGlobal()";
    assert_eq!(
        eval(source),
        Err("hostile global Function reached".to_string())
    );
}

#[test]
fn deep_recursion_hits_the_depth_cap_not_the_rust_stack() {
    assert_eq!(
        eval("function d(c) { return d(c + 1); } d(0)"),
        Err("call depth cap exceeded".to_string())
    );
}

const DEEP_NESTING_STACK_BYTES: usize = 64 << 20;

#[test]
fn a_deeply_nested_expression_evaluates_without_overflowing_the_stack() {
    std::thread::Builder::new()
        .stack_size(DEEP_NESTING_STACK_BYTES)
        .spawn(|| {
            let depth = 20_000;
            let source = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
            assert_eq!(eval(&source), Ok("number:1".to_string()));
            let unary = format!("{}1", "- ".repeat(20_001));
            assert_eq!(eval(&unary), Ok("number:-1".to_string()));
        })
        .expect("the deep-nesting stack thread spawns")
        .join()
        .expect("the deep-nesting stack thread completed");
}

#[test]
fn a_runaway_string_build_hits_the_length_cap() {
    assert_eq!(
        eval_with(
            "var s = 'x'; while (!![]) { s += s; } s",
            Budget::new(10_000_000)
        ),
        Err("string length cap exceeded".to_string())
    );
}

#[test]
fn an_infinite_loop_hits_the_loop_cap() {
    assert_eq!(
        eval("while (!![]) {}"),
        Err("loop iteration cap exceeded".to_string())
    );
}

#[test]
fn unsupported_syntax_bails_by_name_rather_than_guessing() {
    assert_eq!(
        eval("class C {}; 1"),
        Err("unsupported construct: statement kind".to_string())
    );
    assert_eq!(
        eval("var [a] = [1]; a"),
        Err("unsupported construct: destructuring declaration".to_string())
    );
    assert_eq!(
        eval("(function* () { yield 1; })()"),
        Err("unsupported construct: a call of a generator or an async function".to_string())
    );
    assert_eq!(
        eval("1n + 1n"),
        Err("unsupported construct: BigInt literal".to_string())
    );
}

#[test]
fn a_long_bound_function_chain_bails_instead_of_overflowing_the_stack() {
    let source = "\
var f = function () { return 1; };
for (var i = 0; i < 90000; i++) { f = f['bind'](null); }
for (var j = 0; j < 90000; j++) { f = f['bind'](null); }
f['toString']()";
    assert_eq!(
        eval(source),
        Err("bound function chain too deep".to_string())
    );
}

fn timed_fuel_bail(source: &str, steps: u64) {
    let started = std::time::Instant::now();
    let result = eval_with(source, Budget::new(steps));
    let elapsed = started.elapsed();
    assert_eq!(
        result,
        Err("step budget exhausted".to_string()),
        "expected a fuel bail for: {source}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(1),
        "took {elapsed:?} for: {source}"
    );
}

#[test]
fn string_addition_is_charged_by_the_operand_lengths() {
    timed_fuel_bail(
        "var s = 'x'; for (var i = 0; i < 20; i++) { s = s + s; } s['length']",
        1_000,
    );
}

#[test]
fn a_template_literal_is_charged_by_the_pieces_it_joins() {
    timed_fuel_bail(
        "var s = 'x'; for (var i = 0; i < 20; i++) { s = `${s}${s}`; } s['length']",
        1_000,
    );
}

#[test]
fn a_huge_property_key_is_charged_by_its_length() {
    timed_fuel_bail(
        "var k = 'x'['repeat'](100000); var o = {}; for (var i = 0; i < 60; i++) { o[k] = i; } 1",
        150_000,
    );
}

#[test]
fn coercing_and_comparing_huge_strings_is_charged_by_their_lengths() {
    timed_fuel_bail(
        "var s = '9'['repeat'](100000); var n = 0; for (var i = 0; i < 60; i++) { n = n - s; } n",
        150_000,
    );
    timed_fuel_bail(
        "var s = 'x'['repeat'](100000); var c = 0; for (var i = 0; i < 60; i++) { if (s < s) { c++; } } c",
        150_000,
    );
}

#[test]
fn scanning_a_long_string_for_a_substring_is_charged_by_the_scan() {
    timed_fuel_bail(
        "var s = 'x'['repeat'](100000); var n = 0; for (var i = 0; i < 60; i++) { n = n + s['indexOf']('y'); } n",
        150_000,
    );
}

#[test]
fn slicing_and_trimming_a_long_string_is_charged_by_its_length() {
    timed_fuel_bail(
        "var s = 'x'['repeat'](100000); var n = 0; for (var i = 0; i < 60; i++) { n = n + s['slice'](0)['length']; } n",
        150_000,
    );
    timed_fuel_bail(
        "var s = 'x'['repeat'](100000); var n = 0; for (var i = 0; i < 60; i++) { n = n + s['trim']()['length']; } n",
        150_000,
    );
}

#[test]
fn a_global_regexp_replace_is_charged_by_the_subject_it_scans() {
    timed_fuel_bail(
        "var s = 'x'['repeat'](10000); s['replace'](/x/g, 'y')['length']",
        25_000,
    );
}

#[test]
fn joining_a_large_array_is_charged_by_the_output_it_builds() {
    timed_fuel_bail("var a = []; a['length'] = 40000; a['join']('y')", 1_200);
}

#[test]
fn stringifying_a_large_array_is_charged_by_the_output_it_builds() {
    timed_fuel_bail(
        "var a = []; a['length'] = 10000; JSON['stringify'](a)['length']",
        500,
    );
}

#[test]
fn popping_a_large_array_does_not_rescan_it_each_time() {
    timed_fuel_bail(
        "var a = []; a['length'] = 100000; while (a['length']) { a['pop'](); } 1",
        100_000,
    );
}

#[test]
fn unshifting_a_huge_argument_list_is_charged_before_it_moves_the_tail() {
    timed_fuel_bail(
        "var big = []; big['length'] = 500000; var a = [1]; a['unshift']['apply'](a, big); 1",
        20_000,
    );
}

#[test]
fn copying_a_huge_argument_list_into_a_call_is_charged() {
    timed_fuel_bail(
        "var big = []; big['length'] = 500000; var f = function () { return 1; }; var n = 0; for (var i = 0; i < 50; i++) { n = n + f['apply'](null, big); } n",
        20_000,
    );
}

#[test]
fn growing_a_dense_array_is_charged_by_the_growth() {
    timed_fuel_bail(
        "var a = []; for (var i = 0; i < 50; i++) { a['length'] = 1000000; a['length'] = 0; } 1",
        10_000,
    );
}

#[test]
fn an_index_beyond_the_array_length_cap_bails_instead_of_allocating() {
    assert_eq!(
        eval("var a = []; a[2000000] = 1; a['length']"),
        Err("array length cap exceeded".to_string())
    );
}

#[test]
fn an_oversized_regexp_pattern_bails_and_matching_is_charged_by_the_pattern() {
    assert_eq!(
        eval("new RegExp('a'['repeat'](5000))"),
        Err("unsupported construct: regular expression pattern too long".to_string())
    );
    timed_fuel_bail(
        "var s = 'a'['repeat'](5000); var re = new RegExp('a'['repeat'](1000)); re['test'](s)",
        20_000,
    );
}

#[test]
fn define_property_growing_a_dense_array_is_charged() {
    timed_fuel_bail(
        "for (var i = 0; i < 3; i++) { var a = []; Object['defineProperty'](a, '1048575', { value: 1, writable: true, enumerable: true, configurable: true }); } 1",
        2_000,
    );
}

#[test]
fn object_assign_growing_a_dense_array_is_charged() {
    timed_fuel_bail(
        "for (var i = 0; i < 3; i++) { var a = []; Object['assign'](a, { '1048575': 1 }); } 1",
        2_000,
    );
}

#[test]
fn stringifying_a_long_string_is_charged_by_its_length() {
    timed_fuel_bail(
        "var s = 'x'['repeat'](100000); for (var i = 0; i < 60; i++) { JSON['stringify'](s); } 1",
        150_000,
    );
}

#[test]
fn rendering_a_long_regexp_source_is_charged_by_its_length() {
    timed_fuel_bail(
        "var re = new RegExp('a'['repeat'](4000)); for (var i = 0; i < 200; i++) { re['toString'](); } 1",
        10_000,
    );
}

#[test]
fn a_prototype_chain_deeper_than_the_cap_bails_instead_of_reading_undefined() {
    assert_eq!(
        eval("var o = {}; for (var i = 0; i < 100; i++) { o = Object['create'](o); } o['missing']"),
        Err("prototype chain too deep".to_string())
    );
}
