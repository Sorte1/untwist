mod common;

use common::eval_with;
use untwist_eval::budget::Budget;

const BUDGET: u64 = 60_000;

fn array_of(items: usize) -> String {
    format!("var a = []; for (var i = 0; i < {items}; i++) a.push(i);")
}

fn refused_for_fuel(source: &str, budget: u64) -> bool {
    match eval_with(source, Budget::new(budget)) {
        Ok(_) => false,
        Err(why) => {
            assert!(why.contains("budget") || why.contains("fuel"), "{why}");
            true
        }
    }
}

#[test]
fn entering_a_for_in_loop_pays_for_every_key_it_enumerates() {
    let source = format!(
        "{} var n = 0; for (var j = 0; j < 200; j++) {{ for (var k in a) {{ n++; break; }} }} n;",
        array_of(1000)
    );
    assert!(refused_for_fuel(&source, BUDGET), "{source}");
    assert_eq!(
        eval_with(&source, Budget::new(20 * BUDGET)),
        Ok("number:200".to_string())
    );
}

#[test]
fn a_for_in_loop_entered_a_few_times_still_runs() {
    let source = format!(
        "{} var n = 0; for (var j = 0; j < 4; j++) {{ for (var k in a) {{ n++; break; }} }} n;",
        array_of(1000)
    );
    assert_eq!(
        eval_with(&source, Budget::new(BUDGET)),
        Ok("number:4".to_string())
    );
}

#[test]
fn a_for_in_over_a_string_pays_for_every_index() {
    let source = "var s = 'x'; for (var i = 0; i < 12; i++) s += s; var n = 0; \
                  for (var j = 0; j < 100; j++) { for (var k in s) { n++; break; } } n;";
    assert!(refused_for_fuel(source, BUDGET), "{source}");
}

#[test]
fn object_keys_pays_for_every_key() {
    let source = format!(
        "{} var n = 0; for (var j = 0; j < 200; j++) n += Object.keys(a).length; n;",
        array_of(1000)
    );
    assert!(refused_for_fuel(&source, BUDGET), "{source}");
}

fn long_strings(compare: &str) -> String {
    format!(
        "var s = 'x'; for (var i = 0; i < 16; i++) s += s; var a = s + 'y', b = s + 'y', n = 0; \
         for (var i = 0; i < 100; i++) {{ {compare} }} n;"
    )
}

#[test]
fn a_switch_pays_for_a_long_string_comparison_like_strict_equality() {
    let switched = long_strings("switch (a) { case b: n++; }");
    let compared = long_strings("if (a === b) n++;");
    assert!(refused_for_fuel(&compared, BUDGET), "{compared}");
    assert!(refused_for_fuel(&switched, BUDGET), "{switched}");
    let roomy = Budget::new(10 * BUDGET);
    assert_eq!(eval_with(&switched, roomy), Ok("number:100".to_string()));
}
