use untwist_eval::coerce::{
    number_to_js_string, number_to_radix_string, number_to_string, parse_float, parse_int,
    to_int32, to_integer_or_infinity, to_number, to_number_units, to_uint16, to_uint32,
};

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn string_to_number_follows_spec() {
    assert_eq!(to_number(""), 0.0);
    assert_eq!(to_number("  12  "), 12.0);
    assert_eq!(to_number("0x1cc"), 460.0);
    assert_eq!(to_number("0b101"), 5.0);
    assert_eq!(to_number("0o17"), 15.0);
    assert_eq!(to_number("1e3"), 1000.0);
    assert_eq!(to_number("Infinity"), f64::INFINITY);
    assert_eq!(to_number("-Infinity"), f64::NEG_INFINITY);
    assert!(to_number("12px").is_nan());
    assert_eq!(to_number("12."), 12.0);
    assert_eq!(to_number(".5"), 0.5);
    assert!(to_number("0x").is_nan());
}

#[test]
fn and_uint32_wrap() {
    assert_eq!(to_int32(2.9), 2);
    assert_eq!(to_int32(-2.9), -2);
    assert_eq!(to_int32(1e10), 1410065408);
    assert_eq!(to_int32(f64::NAN), 0);
    assert_eq!(to_int32(f64::INFINITY), 0);
    assert_eq!(to_int32(0xfffffffff_u64 as f64), -1);
    assert_eq!(to_uint32(-1.0), 4294967295);
}

#[test]
fn to_number_rounds_large_radix_literals_exactly_once() {
    assert_eq!(to_number("0x43bceaba8a768af8"), 4881034183070616576.0);
    assert_eq!(to_number("0x00000000000000000000000000000000ff"), 255.0);
    let overflowing = to_number(&format!("0x{}", "f".repeat(40)));
    assert!(overflowing.is_finite());
    assert!(overflowing > 0.0);
    assert_eq!(
        to_number(&format!("0b{}", "1".repeat(64))),
        18446744073709551615.0
    );
}

#[test]
fn number_to_string_matches_js() {
    assert_eq!(number_to_string(0.0), "0");
    assert_eq!(number_to_string(-0.0), "0");
    assert_eq!(number_to_string(255.0), "255");
    assert_eq!(number_to_string(0.1 + 0.2), "0.30000000000000004");
    assert_eq!(number_to_string(1e21), "1e+21");
    assert_eq!(number_to_string(5e-7), "5e-7");
    assert_eq!(number_to_string(f64::NAN), "NaN");
    assert_eq!(number_to_string(f64::NEG_INFINITY), "-Infinity");
}

#[test]
fn parse_int_prefix_parses_like_the_rotator_needs() {
    assert_eq!(parse_int(&units("590708XPLBWj"), None), Some(590708.0));
    assert_eq!(parse_int(&units("0x10"), None), Some(16.0));
    assert_eq!(parse_int(&units("08"), None), Some(8.0));
    assert_eq!(parse_int(&units("  -12abc"), None), Some(-12.0));
    assert_eq!(parse_int(&units("ff"), Some(16.0)), Some(255.0));
    assert_eq!(parse_int(&units("0x10"), Some(16.0)), Some(16.0));
    assert!(parse_int(&units(""), None).is_some_and(f64::is_nan));
    assert!(parse_int(&units("XPL"), None).is_some_and(f64::is_nan));
    assert!(parse_int(&units("10"), Some(1.0)).is_some_and(f64::is_nan));
    assert_eq!(parse_int(&units("5e-7"), None), Some(5.0));
}

#[test]
#[allow(clippy::approx_constant)]
fn parse_float_takes_the_longest_decimal_prefix() {
    assert_eq!(parse_float(&units("3.14abc")), 3.14);
    assert_eq!(parse_float(&units("  -Infinity!")), f64::NEG_INFINITY);
    assert!(parse_float(&units("abc")).is_nan());
}

#[test]
fn to_number_over_code_units_rejects_lone_surrogates() {
    assert_eq!(to_number_units(&units("0x1cc")), 460.0);
    assert!(to_number_units(&[0xD800]).is_nan());
    assert_eq!(to_number_units(&[]), 0.0);
}

#[test]
fn radix_strings_match_js() {
    assert_eq!(number_to_radix_string(255.0, 16), "ff");
    assert_eq!(number_to_radix_string(10.0, 16), "a");
    assert_eq!(number_to_radix_string(-255.0, 16), "-ff");
    assert_eq!(number_to_radix_string(0.5, 2), "0.1");
    assert_eq!(number_to_radix_string(0.0, 16), "0");
    assert_eq!(number_to_radix_string(f64::NAN, 16), "NaN");
    assert_eq!(number_to_radix_string(1234.0, 10), "1234");
}

#[test]
fn uint16_and_integer_or_infinity_follow_the_spec() {
    assert_eq!(to_uint16(65601.0), 65);
    assert_eq!(to_uint16(-1.0), 65535);
    assert_eq!(to_uint16(f64::NAN), 0);
    assert_eq!(to_integer_or_infinity(-2.9), -2.0);
    assert_eq!(to_integer_or_infinity(f64::NAN), 0.0);
    assert_eq!(to_integer_or_infinity(f64::INFINITY), f64::INFINITY);
}

#[test]
fn number_to_js_string_is_code_units() {
    assert_eq!(number_to_js_string(1e21).to_lossy_string(), "1e+21");
    assert_eq!(number_to_js_string(-0.0).to_lossy_string(), "0");
}

#[test]
fn u0085_is_not_javascript_whitespace() {
    assert!(to_number("\u{85}1").is_nan());
    assert!(parse_int(&units("\u{85}1"), None).is_some_and(f64::is_nan));
    assert!(parse_float(&units("\u{85}1")).is_nan());
    assert_eq!(to_number("\u{2028}1"), 1.0);
    assert_eq!(to_number("\u{3000}1"), 1.0);
}
