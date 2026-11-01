mod common;

use common::{eval, eval_with_notes};

#[test]
fn number_to_string_with_a_radix() {
    assert_eq!(eval("(255)['toString'](16)"), Ok("string:ff".to_string()));
    assert_eq!(eval("(255)['toString']()"), Ok("string:255".to_string()));
    assert_eq!(eval("(1e21)['toString']()"), Ok("string:1e+21".to_string()));
}

#[test]
fn math_functions_used_by_obfuscator_output() {
    assert_eq!(eval("Math['round'](0.5)"), Ok("number:1".to_string()));
    assert_eq!(eval("Math['floor'](-2.5)"), Ok("number:-3".to_string()));
    assert_eq!(eval("Math['max'](1, 7, 3)"), Ok("number:7".to_string()));
    assert_eq!(eval("Math['abs'](-4)"), Ok("number:4".to_string()));
}

#[test]
fn math_random_is_refused_rather_than_stubbed() {
    assert!(eval("Math['random']()").is_err());
    assert!(eval("Math['floor'](Math['random']() * 0)").is_err());
    let (result, notes) = eval_with_notes("Math['random']()");
    assert!(result.is_err());
    assert!(notes.is_empty());
}

#[test]
fn number_statics() {
    assert_eq!(
        eval("Number['isInteger'](2)"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("Number['MAX_SAFE_INTEGER']"),
        Ok("number:9007199254740991".to_string())
    );
}

#[test]
fn math_round_negative_halves_round_toward_negative_zero() {
    assert_eq!(eval("Math['round'](-0.5)"), Ok("number:-0".to_string()));
    assert_eq!(eval("Math['round'](-0.2)"), Ok("number:-0".to_string()));
    assert_eq!(eval("Math['round'](2.5)"), Ok("number:3".to_string()));
    assert_eq!(eval("Math['round'](-2.5)"), Ok("number:-2".to_string()));
}

#[test]
fn number_stays_callable_and_reuses_the_global_parsers() {
    assert_eq!(eval("Number('0x1cc')"), Ok("number:460".to_string()));
    assert_eq!(eval("Number()"), Ok("number:0".to_string()));
    assert_eq!(
        eval("Number['parseInt']('42px')"),
        Ok("number:42".to_string())
    );
    assert_eq!(
        eval("Number['parseFloat']('3.14x')"),
        Ok("number:3.14".to_string())
    );
}

#[test]
fn math_pow_follows_the_exponentiation_operator_on_the_one_infinity_cases() {
    assert_eq!(
        eval("Math['pow'](1, Infinity)"),
        Ok("number:NaN".to_string())
    );
    assert_eq!(
        eval("Math['pow'](-1, -Infinity)"),
        Ok("number:NaN".to_string())
    );
    assert_eq!(eval("Math['pow'](2, 10)"), Ok("number:1024".to_string()));
}
