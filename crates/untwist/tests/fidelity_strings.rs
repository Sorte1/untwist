mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused, refused};
use refused_rows::check;

const A_CAUGHT_ATOB_ERROR_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return atob('cz*A='); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_atob_error_is_refused() {
    check(&refused(A_CAUGHT_ATOB_ERROR_IS_REFUSED, "s0|E:InvalidCharacterError:Invalid character\ns1|E:InvalidCharacterError:Invalid character\ns2|E:InvalidCharacterError:Invalid character\n"));
}

const A_CAUGHT_MALFORMED_URI_ERROR_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return decodeURIComponent('%E0%A4%A'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_malformed_uri_error_is_refused() {
    check(&refused(
        A_CAUGHT_MALFORMED_URI_ERROR_IS_REFUSED,
        "s0|E:URIError:URI malformed\ns1|E:URIError:URI malformed\ns2|E:URIError:URI malformed\n",
    ));
}

const A_CAUGHT_URI_ERROR_ON_A_SURROGATE_ESCAPE_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return decodeURIComponent('%ED%A0%80'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_uri_error_on_a_surrogate_escape_is_refused() {
    check(&refused(
        A_CAUGHT_URI_ERROR_ON_A_SURROGATE_ESCAPE_IS_REFUSED,
        "s0|E:URIError:URI malformed\ns1|E:URIError:URI malformed\ns2|E:URIError:URI malformed\n",
    ));
}

const A_CAUGHT_URI_ERROR_ON_A_SHORT_ESCAPE_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return decodeURIComponent('%'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_uri_error_on_a_short_escape_is_refused() {
    check(&refused(
        A_CAUGHT_URI_ERROR_ON_A_SHORT_ESCAPE_IS_REFUSED,
        "s0|E:URIError:URI malformed\ns1|E:URIError:URI malformed\ns2|E:URIError:URI malformed\n",
    ));
}

const A_CAUGHT_URI_ERROR_ON_AN_OVERLONG_ESCAPE_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return decodeURIComponent('%C0%80'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_uri_error_on_an_overlong_escape_is_refused() {
    check(&refused(
        A_CAUGHT_URI_ERROR_ON_AN_OVERLONG_ESCAPE_IS_REFUSED,
        "s0|E:URIError:URI malformed\ns1|E:URIError:URI malformed\ns2|E:URIError:URI malformed\n",
    ));
}

const A_CAUGHT_URI_ERROR_FROM_ENCODING_A_LONE_SURROGATE_IS_REFU: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return encodeURIComponent('\ud800'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_uri_error_from_encoding_a_lone_surrogate_is_refused() {
    check(&refused(
        A_CAUGHT_URI_ERROR_FROM_ENCODING_A_LONE_SURROGATE_IS_REFU,
        "s0|E:URIError:URI malformed\ns1|E:URIError:URI malformed\ns2|E:URIError:URI malformed\n",
    ));
}

const A_CAUGHT_REPEAT_ERROR_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return 'ab'.repeat(-1); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_repeat_error_is_refused() {
    check(&refused(A_CAUGHT_REPEAT_ERROR_IS_REFUSED, "s0|E:RangeError:Invalid count value: -1\ns1|E:RangeError:Invalid count value: -1\ns2|E:RangeError:Invalid count value: -1\n"));
}

const A_GENERIC_SLICE_OF_A_STRING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [].slice.call('abc').join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_generic_slice_of_a_string_is_refused() {
    check(&refused(
        A_GENERIC_SLICE_OF_A_STRING_IS_REFUSED,
        "s0|a,b,c\ns1|a,b,c\ns2|a,b,c\n",
    ));
}

const A_GETTER_READ_THROUGH_TO_STRING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return String({get x(){ return 1; }}.x).length; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_getter_read_through_to_string_is_refused() {
    check(&refused(
        A_GETTER_READ_THROUGH_TO_STRING_IS_REFUSED,
        "s0|1\ns1|1\ns2|1\n",
    ));
}

const A_GETTER_SEEN_BY_JSON_STRINGIFY_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return JSON.stringify({get x(){ return 'G'; }}); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_getter_seen_by_json_stringify_is_refused() {
    check(&refused(
        A_GETTER_SEEN_BY_JSON_STRINGIFY_IS_REFUSED,
        "s0|{\"x\":\"G\"}\ns1|{\"x\":\"G\"}\ns2|{\"x\":\"G\"}\n",
    ));
}

const THE_KEYS_OF_AN_ARRAY_WITH_HOLES_AND_A_STRING_ARE_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return Object.keys('ab').join() + Object.keys([5,,6]).join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn the_keys_of_an_array_with_holes_and_a_string_are_refused() {
    check(&refused(
        THE_KEYS_OF_AN_ARRAY_WITH_HOLES_AND_A_STRING_ARE_REFUSED,
        "s0|0,10,2\ns1|0,10,2\ns2|0,10,2\n",
    ));
}

const JSON_STRINGIFY_USES_THE_SHORT_ESCAPES: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + (JSON.stringify('\u2028\ud800"\b\u0001\u007f')); }"##
);

#[test]
fn json_stringify_uses_the_short_escapes() {
    check(&agrees(JSON_STRINGIFY_USES_THE_SHORT_ESCAPES, "s0|\" \\ud800\\\"\\b\\u0001\"\ns1|\" \\ud800\\\"\\b\\u0001\"\ns2|\" \\ud800\\\"\\b\\u0001\"\n"));
}

const FOR_IN_OVER_A_STRING_VISITS_ITS_INDICES: &str = unit!(
    r##"function b(i){ var e=a(); var s = ''; for (var k in 'ab') s += k; return e[i] + '|' + s; }"##
);

#[test]
fn for_in_over_a_string_visits_its_indices() {
    check(&agrees(
        FOR_IN_OVER_A_STRING_VISITS_ITS_INDICES,
        "s0|01\ns1|01\ns2|01\n",
    ));
}

const OBJECT_KEYS_OF_A_STRING_LISTS_ITS_INDICES: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return Object.keys('ab').join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn object_keys_of_a_string_lists_its_indices() {
    check(&agrees(
        OBJECT_KEYS_OF_A_STRING_LISTS_ITS_INDICES,
        "s0|0,1\ns1|0,1\ns2|0,1\n",
    ));
}

const JSON_STRINGIFY_WRITES_INTEGER_KEYS_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify({ b: 1, 1: 2 })); }"##
);

#[test]
fn json_stringify_writes_integer_keys_first() {
    check(&decoded(
        JSON_STRINGIFY_WRITES_INTEGER_KEYS_FIRST,
        "s0|{\"1\":2,\"b\":1}\ns1|{\"1\":2,\"b\":1}\ns2|{\"1\":2,\"b\":1}\n",
    ));
}

const JSON_STRINGIFY_WITH_AN_INDENT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify([1, [2]], null, 1)); }"##
);

#[test]
fn json_stringify_with_an_indent_is_refused() {
    check(&q_refused(
        JSON_STRINGIFY_WITH_AN_INDENT_IS_REFUSED,
        "s0|[\n 1,\n [\n  2\n ]\n]\ns1|[\n 1,\n [\n  2\n ]\n]\ns2|[\n 1,\n [\n  2\n ]\n]\n",
    ));
}

const JSON_STRINGIFY_WITH_A_REPLACER_ARRAY_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify({ a: 1, b: 2 }, ['a'])); }"##
);

#[test]
fn json_stringify_with_a_replacer_array_is_refused() {
    check(&q_refused(
        JSON_STRINGIFY_WITH_A_REPLACER_ARRAY_IS_REFUSED,
        "s0|{\"a\":1}\ns1|{\"a\":1}\ns2|{\"a\":1}\n",
    ));
}

const JSON_STRINGIFY_WITH_A_REPLACER_FUNCTION_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify({ a: 1, b: [2] }, function(k, v){ return typeof v === 'number' ? v * 10 : v; }) + JSON.stringify(5, function(k, v){ return k === '' ? 'root' : v; })); }"##
);

#[test]
fn json_stringify_with_a_replacer_function_is_refused() {
    check(&q_refused(JSON_STRINGIFY_WITH_A_REPLACER_FUNCTION_IS_REFUSED, "s0|{\"a\":10,\"b\":[20]}\"root\"\ns1|{\"a\":10,\"b\":[20]}\"root\"\ns2|{\"a\":10,\"b\":[20]}\"root\"\n"));
}

const OBJECT_PROTOTYPE_TO_STRING_NAMES_THE_CLASS: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([Object.prototype.toString.call([1]), Object.prototype.toString.call('s'), Object.prototype.toString.call(1), Object.prototype.toString.call(true), Object.prototype.toString.call(null), Object.prototype.toString.call(undefined), Object.prototype.toString.call(function(){}), Object.prototype.toString.call(/a/), String({})].join()); }"##
);

#[test]
fn object_prototype_to_string_names_the_class() {
    check(&decoded(OBJECT_PROTOTYPE_TO_STRING_NAMES_THE_CLASS, "s0|[object Array],[object String],[object Number],[object Boolean],[object Null],[object Undefined],[object Function],[object RegExp],[object Object]\ns1|[object Array],[object String],[object Number],[object Boolean],[object Null],[object Undefined],[object Function],[object RegExp],[object Object]\ns2|[object Array],[object String],[object Number],[object Boolean],[object Null],[object Undefined],[object Function],[object RegExp],[object Object]\n"));
}

const CASE_MAPPING_OF_NON_ASCII_TEXT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (encodeURIComponent(('a\ud800b').toUpperCase().replace('\ud800', '-')) + 'Σ'.toLowerCase() + 'ΑΣ\ud800'.toLowerCase().length); }"##
);

#[test]
fn case_mapping_of_non_ascii_text_is_refused() {
    check(&q_refused(
        CASE_MAPPING_OF_NON_ASCII_TEXT_IS_REFUSED,
        "s0|A-Bσ3\ns1|A-Bσ3\ns2|A-Bσ3\n",
    ));
}

const BTOA_WITHOUT_AN_ARGUMENT_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return btoa(); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn btoa_without_an_argument_is_refused() {
    check(&q_refused(
        BTOA_WITHOUT_AN_ARGUMENT_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const HAS_OWN_PROPERTY_ON_A_STRING_SEES_ITS_INDICES_AND_LENGTH: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([('abc').hasOwnProperty('1'), 'abc'.hasOwnProperty('length'), 'abc'.hasOwnProperty('3'), (5).hasOwnProperty('x')].join()); }"##
);

#[test]
fn has_own_property_on_a_string_sees_its_indices_and_length() {
    check(&decoded(
        HAS_OWN_PROPERTY_ON_A_STRING_SEES_ITS_INDICES_AND_LENGTH,
        "s0|true,true,false,false\ns1|true,true,false,false\ns2|true,true,false,false\n",
    ));
}

const SPLIT_CONVERTS_ITS_SEPARATOR_BEFORE_A_ZERO_LIMIT: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { 'abc'.split({ toString: function(){ return {}; } }, 0); return 'none'; } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn split_converts_its_separator_before_a_zero_limit() {
    check(&decoded(
        SPLIT_CONVERTS_ITS_SEPARATOR_BEFORE_A_ZERO_LIMIT,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}
