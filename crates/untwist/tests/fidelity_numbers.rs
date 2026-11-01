mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused, refused};
use refused_rows::check;

const A_CAUGHT_RANGE_ERROR_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return (5).toString(1); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_range_error_is_refused() {
    check(&refused(A_CAUGHT_RANGE_ERROR_IS_REFUSED, "s0|E:RangeError:toString() radix argument must be between 2 and 36\ns1|E:RangeError:toString() radix argument must be between 2 and 36\ns2|E:RangeError:toString() radix argument must be between 2 and 36\n"));
}

const A_MISSING_NUMBER_METHOD_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [0.1.toFixed(20), (1e21).toLocaleString === undefined, 1e21.toString(10), (-1e-7).toString(), (1e-6).toString(), 0.000001 + '', 1.0000000000000002 + '', 4.35 * 100, 1.1 * 1.1].join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_missing_number_method_is_refused() {
    check(&refused(A_MISSING_NUMBER_METHOD_IS_REFUSED, "s0|0.10000000000000000555,false,1e+21,-1e-7,0.000001,0.000001,1.0000000000000002,434.99999999999994,1.2100000000000002\ns1|0.10000000000000000555,false,1e+21,-1e-7,0.000001,0.000001,1.0000000000000002,434.99999999999994,1.2100000000000002\ns2|0.10000000000000000555,false,1e+21,-1e-7,0.000001,0.000001,1.0000000000000002,434.99999999999994,1.2100000000000002\n"));
}

const A_RADIX_STRING_OF_A_FRACTION_OR_A_LARGE_INTEGER_IS_REFUSE: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return (255).toString(36) + ',' + (0.5).toString(2) + ',' + (-255).toString(16) + ',' + (1e21).toString(36) + ',' + (0.1).toString(3) + ',' + (123.456).toString(36) + ',' + (2**60).toString(2).length + ',' + Number.MAX_SAFE_INTEGER.toString(36) + ',' + (-0).toString(2); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_radix_string_of_a_fraction_or_a_large_integer_is_refused() {
    check(&refused(A_RADIX_STRING_OF_A_FRACTION_OR_A_LARGE_INTEGER_IS_REFUSE, "s0|73,0.1,-ff,5v1j4f4ds7c000,0.0022002200220022002200220022002201,3f.gez4w97ry,61,2gosa7pa2gv,0\ns1|73,0.1,-ff,5v1j4f4ds7c000,0.0022002200220022002200220022002201,3f.gez4w97ry,61,2gosa7pa2gv,0\ns2|73,0.1,-ff,5v1j4f4ds7c000,0.0022002200220022002200220022002201,3f.gez4w97ry,61,2gosa7pa2gv,0\n"));
}

const TRANSCENDENTAL_MATH_IS_REFUSED_AND_INEXACT_POWERS_STAY: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [Math.sin(1e10), Math.exp(1), Math.pow(2, 0.5), Math.tan(1e300), Math.atan2(1, 3), Math.log(10), Math.cos(1), Math.pow(10, -5), Math.pow(10, 308), 10 ** -5, Math.log10(1000), Math.log2(8), Math.expm1(1e-10), Math.sinh(1), Math.acos(0.5), Math.pow(1.1, 100), Math.exp(100), Math.log1p(1e-15), Math.atanh(0.5), Math.cosh(10)].join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn transcendental_math_is_refused_and_inexact_powers_stay() {
    check(&refused(TRANSCENDENTAL_MATH_IS_REFUSED_AND_INEXACT_POWERS_STAY, "s0|-0.4875060250875107,2.718281828459045,1.4142135623730951,1.4214488238747245,0.3217505543966422,2.302585092994046,0.5403023058681398,0.000009999999999999999,1e+308,0.000009999999999999999,3,3,1.00000000005e-10,1.1752011936438014,1.0471975511965979,13780.612339822383,2.6881171418161356e+43,9.999999999999995e-16,0.5493061443340548,11013.232920103324\ns1|-0.4875060250875107,2.718281828459045,1.4142135623730951,1.4214488238747245,0.3217505543966422,2.302585092994046,0.5403023058681398,0.000009999999999999999,1e+308,0.000009999999999999999,3,3,1.00000000005e-10,1.1752011936438014,1.0471975511965979,13780.612339822383,2.6881171418161356e+43,9.999999999999995e-16,0.5493061443340548,11013.232920103324\ns2|-0.4875060250875107,2.718281828459045,1.4142135623730951,1.4214488238747245,0.3217505543966422,2.302585092994046,0.5403023058681398,0.000009999999999999999,1e+308,0.000009999999999999999,3,3,1.00000000005e-10,1.1752011936438014,1.0471975511965979,13780.612339822383,2.6881171418161356e+43,9.999999999999995e-16,0.5493061443340548,11013.232920103324\n"));
}

const A_RADIX_STRING_OF_A_FRACTION_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [(0.1).toString(2), (1/3).toString(36), (Math.PI).toString(16), (1e-7).toString(2).length, (0.000001).toString(7), (255.5).toString(16), (-0.5).toString(36), (1e-300).toString(36).length, (2**-1074).toString(2).length, (123.456).toString(8)].join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_radix_string_of_a_fraction_is_refused() {
    check(&refused(A_RADIX_STRING_OF_A_FRACTION_IS_REFUSED, "s0|0.0001100110011001100110011001100110011001100110011001101,0.c,3.243f6a8885a3,75,0.00000005523220033662416625,ff.8,-0.i,205,1076,173.3513615237574734\ns1|0.0001100110011001100110011001100110011001100110011001101,0.c,3.243f6a8885a3,75,0.00000005523220033662416625,ff.8,-0.i,205,1076,173.3513615237574734\ns2|0.0001100110011001100110011001100110011001100110011001101,0.c,3.243f6a8885a3,75,0.00000005523220033662416625,ff.8,-0.i,205,1076,173.3513615237574734\n"));
}

const AN_INEXACT_POWER_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var x = -5, y = 23; return e[i] + '|' + Math.pow(10, x) + ',' + (10 ** x) + ',' + Math.pow(10, y) + ',' + Math.pow(7, -3) + ',' + Math.pow(2, 0.5); }"##
);

#[test]
fn an_inexact_power_is_refused() {
    check(&refused(AN_INEXACT_POWER_IS_REFUSED, "s0|0.000009999999999999999,0.000009999999999999999,1e+23,0.0029154518950437317,1.4142135623730951\ns1|0.000009999999999999999,0.000009999999999999999,1e+23,0.0029154518950437317,1.4142135623730951\ns2|0.000009999999999999999,0.000009999999999999999,1e+23,0.0029154518950437317,1.4142135623730951\n"));
}

const MATH_ROUND_EDGES: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + Math.round(0.49999999999999994) + ',' + Math.round(4503599627370497); }"
);

#[test]
fn math_round_is_exact_at_its_edges() {
    check(&agrees(
        MATH_ROUND_EDGES,
        "s0|0,4503599627370497\ns1|0,4503599627370497\ns2|0,4503599627370497\n",
    ));
}

const TO_FIXED_TIES: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + (2.5).toFixed(0) + ',' + (1.25).toFixed(1); }"
);

#[test]
fn a_to_fixed_tie_is_refused() {
    check(&refused(TO_FIXED_TIES, "s0|3,1.3\ns1|3,1.3\ns2|3,1.3\n"));
}

const TO_FIXED_EDGES: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + (-0).toFixed(1) + ',' + (1e21).toFixed(2) + ',' + (0.000001).toFixed(7); }"
);

#[test]
fn to_fixed_matches_node_on_negative_zero_and_large_values() {
    check(&agrees(
        TO_FIXED_EDGES,
        "s0|0.0,1e+21,0.0000010\ns1|0.0,1e+21,0.0000010\ns2|0.0,1e+21,0.0000010\n",
    ));
}

const A_LOGARITHM: &str = unit!("function b(i){ var e=a(); return e[i] + '|' + Math.log(10); }");

#[test]
fn an_inexact_logarithm_is_refused() {
    check(&refused(
        A_LOGARITHM,
        "s0|2.302585092994046\ns1|2.302585092994046\ns2|2.302585092994046\n",
    ));
}

const A_GLOBAL_REPLACE_KEEPS_THE_TEXT_AROUND_EMPTY_MATCHES: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return 'abc'.replace(/(?:)/g, '-') + ',' + '\ud83d\ude00'.replace(/(?:)/gu, '-').length + ',' + '\ud83d\ude00'.replace(/(?:)/g, '-').length; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_global_replace_keeps_the_text_around_empty_matches() {
    check(&agrees(
        A_GLOBAL_REPLACE_KEEPS_THE_TEXT_AROUND_EMPTY_MATCHES,
        "s0|-a-b-c-,4,5\ns1|-a-b-c-,4,5\ns2|-a-b-c-,4,5\n",
    ));
}

const MATH_MAX_ORDERS_NEGATIVE_ZERO_BELOW_ZERO: &str =
    unit!(r##"function b(i){ var e=a(); return e[i] + '|' + (1/Math.max(-0, 0)); }"##);

#[test]
fn math_max_orders_negative_zero_below_zero() {
    check(&agrees(
        MATH_MAX_ORDERS_NEGATIVE_ZERO_BELOW_ZERO,
        "s0|Infinity\ns1|Infinity\ns2|Infinity\n",
    ));
}

const MATH_MIN_ORDERS_NEGATIVE_ZERO_BELOW_ZERO: &str =
    unit!(r##"function b(i){ var e=a(); return e[i] + '|' + (1/Math.min(0, -0)); }"##);

#[test]
fn math_min_orders_negative_zero_below_zero() {
    check(&agrees(
        MATH_MIN_ORDERS_NEGATIVE_ZERO_BELOW_ZERO,
        "s0|-Infinity\ns1|-Infinity\ns2|-Infinity\n",
    ));
}

const MATH_MAX_COERCES_EVERY_ARGUMENT_AFTER_A_NAN: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ var n=0; var o={valueOf:function(){ n++; return 1; }}; Math.max(NaN, o); return n; })()); }"##
);

#[test]
fn math_max_coerces_every_argument_after_a_nan() {
    check(&agrees(
        MATH_MAX_COERCES_EVERY_ARGUMENT_AFTER_A_NAN,
        "s0|1\ns1|1\ns2|1\n",
    ));
}

const NUMBER_MIN_VALUE_IS_THE_SMALLEST_SUBNORMAL: &str =
    unit!(r##"function b(i){ var e=a(); return e[i] + '|' + (String(Number.MIN_VALUE)); }"##);

#[test]
fn number_min_value_is_the_smallest_subnormal() {
    check(&agrees(
        NUMBER_MIN_VALUE_IS_THE_SMALLEST_SUBNORMAL,
        "s0|5e-324\ns1|5e-324\ns2|5e-324\n",
    ));
}

const REPLACEMENT_TEMPLATES_SUBSTITUTE_THE_TEXT_AROUND_THE_MATCH: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (['abc'.replace(/b/, "[$`]"), 'abc'.replace(/b/, "[$']"), 'abc'.replace(/(b)/, '$10'), 'abc'.replace(/(b)|(z)/, '[$2]'), 'abc'.replace(/b/, '$<x>')].join('/')); }"##
);

#[test]
fn replacement_templates_substitute_the_text_around_the_match() {
    check(&decoded(REPLACEMENT_TEMPLATES_SUBSTITUTE_THE_TEXT_AROUND_THE_MATCH, "s0|a[a]c/a[c]c/ab0c/a[]c/a$<x>c\ns1|a[a]c/a[c]c/ab0c/a[]c/a$<x>c\ns2|a[a]c/a[c]c/ab0c/a[]c/a$<x>c\n"));
}

const REPLACEMENT_TEMPLATES_WITH_A_STRING_PATTERN_SUBSTITUTE_THE_TEXT_AROUND_THE_MATCH: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (['abc'.replace('b', '$&$&'), 'abc'.replace('b', "$`"), 'abc'.replace('b', "$'"), 'abc'.replace('b', '$$'), 'abc'.replace('b', '$1'), 'abc'.replace('b', '$0'), 'abc'.replace('b', '$<x>'), 'abc'.replace('b', '$'), 'abc'.replace('b', '$9$')].join('/')); }"##
);

#[test]
fn replacement_templates_with_a_string_pattern_substitute_the_text_around_the_match() {
    check(&decoded(REPLACEMENT_TEMPLATES_WITH_A_STRING_PATTERN_SUBSTITUTE_THE_TEXT_AROUND_THE_MATCH, "s0|abbc/aac/acc/a$c/a$1c/a$0c/a$<x>c/a$c/a$9$c\ns1|abbc/aac/acc/a$c/a$1c/a$0c/a$<x>c/a$c/a$9$c\ns2|abbc/aac/acc/a$c/a$1c/a$0c/a$<x>c/a$c/a$9$c\n"));
}

const TWO_DIGIT_REPLACEMENT_REFERENCES_FOLLOW_THE_CAPTURE_COUNT: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ('abcdefghijk'.replace(/(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)/, '$11-$10-$01-$0-$00-$12') + ',' + 'ab'.replace(/(a)/, '$01|$10|$2')); }"##
);

#[test]
fn two_digit_replacement_references_follow_the_capture_count() {
    check(&decoded(
        TWO_DIGIT_REPLACEMENT_REFERENCES_FOLLOW_THE_CAPTURE_COUNT,
        "s0|k-j-a-$0-$00-a2,a|a0|$2b\ns1|k-j-a-$0-$00-a2,a|a0|$2b\ns2|k-j-a-$0-$00-a2,a|a0|$2b\n",
    ));
}

const STRING_TO_STRING_ON_A_NUMBER_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return String.prototype.toString.call(5); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn string_to_string_on_a_number_is_refused() {
    check(&q_refused(
        STRING_TO_STRING_ON_A_NUMBER_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const STRING_VALUE_OF_ON_A_NUMBER_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return String.prototype.valueOf.call(5); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn string_value_of_on_a_number_is_refused() {
    check(&q_refused(
        STRING_VALUE_OF_ON_A_NUMBER_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_LONG_DIGIT_STRING_PARSES_EXACTLY: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([parseInt('123456789012345678901234567890123456789012345'), parseInt('ffffffffffffffffffffffffffffffffffffffffff', 16), Number('0x' + 'f'.repeat(41)), parseInt('1'.repeat(70), 2)].join()); }"##
);

#[test]
fn a_long_digit_string_parses_exactly() {
    check(&decoded(A_LONG_DIGIT_STRING_PARSES_EXACTLY, "s0|1.2345678901234567e+44,3.7414441915671115e+50,2.3384026197294447e+49,1.1805916207174113e+21\ns1|1.2345678901234567e+44,3.7414441915671115e+50,2.3384026197294447e+49,1.1805916207174113e+21\ns2|1.2345678901234567e+44,3.7414441915671115e+50,2.3384026197294447e+49,1.1805916207174113e+21\n"));
}

const A_LONG_DIGIT_STRING_IN_AN_ODD_RADIX_IS_REFUSED: &str =
    qunit!(r##"function qb(i){ var e=qa(); return e[i] + '|' + (parseInt('2'.repeat(90), 3)); }"##);

#[test]
fn a_long_digit_string_in_an_odd_radix_is_refused() {
    check(&q_refused(
        A_LONG_DIGIT_STRING_IN_AN_ODD_RADIX_IS_REFUSED,
        "s0|8.727963568087713e+42\ns1|8.727963568087713e+42\ns2|8.727963568087713e+42\n",
    ));
}
