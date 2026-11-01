mod common;

use common::eval;

#[test]
fn from_char_code_is_to_uint16_and_makes_code_units() {
    assert_eq!(eval("String.fromCharCode(65)"), Ok("string:A".to_string()));
    assert_eq!(
        eval("String.fromCharCode(65601)"),
        Ok("string:A".to_string())
    );
    assert_eq!(
        eval("String.fromCharCode(0xD800).length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("String['fromCharCode'](72, 105)"),
        Ok("string:Hi".to_string())
    );
}

#[test]
fn char_at_out_of_range_is_the_empty_string_and_char_code_at_is_nan() {
    assert_eq!(eval("'ab'['charAt'](5)"), Ok("string:".to_string()));
    assert_eq!(eval("'ab'['charAt'](1)"), Ok("string:b".to_string()));
    assert_eq!(eval("'ab'['charCodeAt'](5)"), Ok("number:NaN".to_string()));
    assert_eq!(eval("'ab'['charCodeAt'](0)"), Ok("number:97".to_string()));
}

#[test]
fn index_of_returns_minus_one_which_the_atob_loop_relies_on() {
    let alphabet = "'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/='";
    assert_eq!(
        eval(&format!("{alphabet}['indexOf']('a')")),
        Ok("number:0".to_string())
    );
    assert_eq!(
        eval(&format!("{alphabet}['indexOf']('!')")),
        Ok("number:-1".to_string())
    );
    assert_eq!(
        eval(&format!("~{alphabet}['indexOf']('!')")),
        Ok("number:0".to_string())
    );
}

#[test]
fn the_atob_two_digit_hex_idiom_works() {
    assert_eq!(
        eval("('00' + (10)['toString'](0x10))['slice'](-0x2)"),
        Ok("string:0a".to_string())
    );
    assert_eq!(
        eval("('00' + (255)['toString'](0x10))['slice'](-0x2)"),
        Ok("string:ff".to_string())
    );
}

#[test]
fn slice_substring_substr_and_split() {
    assert_eq!(
        eval("'abcdef'['slice'](1, -1)"),
        Ok("string:bcde".to_string())
    );
    assert_eq!(
        eval("'abcdef'['substring'](4, 1)"),
        Ok("string:bcd".to_string())
    );
    assert_eq!(eval("'abcdef'['substr'](-2)"), Ok("string:ef".to_string()));
    assert_eq!(
        eval("'4|3|0'['split']('|').length"),
        Ok("number:3".to_string())
    );
    assert_eq!(eval("'ab'['split']('').length"), Ok("number:2".to_string()));
}

#[test]
fn length_and_concat_and_repeat() {
    assert_eq!(eval("'abc'.length"), Ok("number:3".to_string()));
    assert_eq!(
        eval("'a'['concat']('b', 'c')"),
        Ok("string:abc".to_string())
    );
    assert_eq!(eval("'ab'['repeat'](2)"), Ok("string:abab".to_string()));
}

#[test]
fn code_point_at_combines_surrogate_pairs() {
    assert_eq!(
        eval("'\u{1F600}'.codePointAt(0)"),
        Ok("number:128512".to_string())
    );
    assert_eq!(eval("'abc'.codePointAt(5)"), Ok("undefined".to_string()));
    assert_eq!(
        eval("'\\uD83Da'.codePointAt(0)"),
        Ok("number:55357".to_string())
    );
}

#[test]
fn starts_ends_and_includes() {
    assert_eq!(
        eval("'hello'.startsWith('he')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("'hello'.startsWith('lo', 3)"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("'hello'.endsWith('lo')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("'hello'.endsWith('hel', 3)"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("'hello'.endsWith('hello!')"),
        Ok("boolean:false".to_string())
    );
    assert_eq!(
        eval("'hello'.includes('ell')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("'hello'.includes('z')"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn pad_start_and_pad_end() {
    assert_eq!(eval("'5'.padStart(3, '0')"), Ok("string:005".to_string()));
    assert_eq!(
        eval("'abc'.padStart(6, '12')"),
        Ok("string:121abc".to_string())
    );
    assert_eq!(eval("'abc'.padStart(2)"), Ok("string:abc".to_string()));
    assert_eq!(eval("'abc'.padStart(5, '')"), Ok("string:abc".to_string()));
    assert_eq!(eval("'5'.padEnd(3, '0')"), Ok("string:500".to_string()));
    assert_eq!(eval("'abc'.padEnd(5)"), Ok("string:abc  ".to_string()));
}

#[test]
fn pad_start_bails_before_allocating_a_huge_string() {
    assert_eq!(
        eval("'x'.padStart(1e9)"),
        Err("string length cap exceeded".to_string())
    );
}

#[test]
fn value_of_matches_to_string() {
    assert_eq!(eval("'abc'.valueOf()"), Ok("string:abc".to_string()));
}

#[test]
fn repeat_bounds_the_count_even_for_an_empty_receiver() {
    assert_eq!(
        eval("''.repeat(1e12)"),
        Err("string length cap exceeded".to_string())
    );
    assert_eq!(eval("'ab'.repeat(3)"), Ok("string:ababab".to_string()));
    assert_eq!(eval("'x'.repeat(0)"), Ok("string:".to_string()));
}

#[test]
fn split_treats_undefined_separator_as_omitted_and_respects_limit() {
    assert_eq!(
        eval("'a undefined b'.split(undefined).length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'a,b,c'.split(',', 1).length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'a,b,c'.split(',', 0).length"),
        Ok("number:0".to_string())
    );
    assert_eq!(
        eval("'a,b,c'.split(',', undefined).length"),
        Ok("number:3".to_string())
    );
}

#[test]
fn last_index_of_honors_from_index() {
    assert_eq!(
        eval("'aaa'.lastIndexOf('a', 1)"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'canal'.lastIndexOf('a', 0)"),
        Ok("number:-1".to_string())
    );
    assert_eq!(
        eval("'canal'.lastIndexOf('a', NaN)"),
        Ok("number:3".to_string())
    );
}

#[test]
fn number_to_string_radix_range_is_enforced() {
    assert_eq!(eval("(255)['toString'](16)"), Ok("string:ff".to_string()));
    assert_eq!(
        eval("(5)['toString'](1)"),
        Err("throw error:RangeError".to_string())
    );
    assert_eq!(
        eval("(5)['toString'](37)"),
        Err("throw error:RangeError".to_string())
    );
}

#[test]
fn split_with_a_regexp_separator_splits_on_matches_not_on_the_source_text() {
    assert_eq!(
        eval("'a1b2c'.split(/\\d/).join('|')"),
        Ok("string:a|b|c".to_string())
    );
    assert_eq!(
        eval("'abc'.split(/\\d/).length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'a,b,,c'.split(/,/).join('|')"),
        Ok("string:a|b||c".to_string())
    );
}

#[test]
fn split_with_a_regexp_separator_keeps_captures_and_honours_the_limit() {
    assert_eq!(
        eval("'a1b2c'.split(/(\\d)/).join('|')"),
        Ok("string:a|1|b|2|c".to_string())
    );
    assert_eq!(
        eval("'x1y'.split(/(\\d)(z)?/)[2]"),
        Ok("undefined".to_string())
    );
    assert_eq!(
        eval("'a1b2c3d'.split(/\\d/, 2).join('|')"),
        Ok("string:a|b".to_string())
    );
}

#[test]
fn split_with_an_empty_regexp_match_advances_and_handles_the_empty_subject() {
    assert_eq!(
        eval("'ab'.split(/(?:)/).join('|')"),
        Ok("string:a|b".to_string())
    );
    assert_eq!(eval("''.split(/a/).length"), Ok("number:1".to_string()));
    assert_eq!(eval("''.split(/(?:)/).length"), Ok("number:0".to_string()));
}

#[test]
fn trim_uses_the_full_js_whitespace_set() {
    assert_eq!(
        eval("'\\u20281\\u2029'['trim']()['length']"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'\\u30001\\u1680'['trim']()['length']"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'\\u205f1\\u202f'['trim']()['length']"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("'\\u00851'['trim']()['length']"),
        Ok("number:2".to_string())
    );
}
