mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused, refused};
use refused_rows::check;

const A_CAUGHT_INVALID_PATTERN_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return new RegExp('('); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_invalid_pattern_is_refused() {
    check(&refused(A_CAUGHT_INVALID_PATTERN_IS_REFUSED, "s0|E:SyntaxError:Invalid regular expression: /(/: Unterminated group\ns1|E:SyntaxError:Invalid regular expression: /(/: Unterminated group\ns2|E:SyntaxError:Invalid regular expression: /(/: Unterminated group\n"));
}

const A_REGEXP_LAST_INDEX_READ_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var r = /\d/g, m, s = ''; while ((m = r.exec('a1b2c3')) !== null) s += m[0] + m.index + r.lastIndex; return e[i] + '|' + s; }"##
);

#[test]
fn a_regexp_last_index_read_is_refused() {
    check(&refused(
        A_REGEXP_LAST_INDEX_READ_IS_REFUSED,
        "s0|112234356\ns1|112234356\ns2|112234356\n",
    ));
}

const A_REGEXP_OVER_SURROGATES_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return '\ud83d\ude00'.match(/./g).length + ',' + '\ud83d\ude00'.match(/./gu).length + ',' + /^.$/u.test('\ud83d\ude00') + ',' + /^.$/.test('\ud83d\ude00'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_regexp_over_surrogates_is_refused() {
    check(&refused(
        A_REGEXP_OVER_SURROGATES_IS_REFUSED,
        "s0|2,1,true,false\ns1|2,1,true,false\ns2|2,1,true,false\n",
    ));
}

const EXEC_AFTER_AN_EMPTY_MATCH: &str = unit!(
    "function b(i){ var e=a(); var r = /q*/g; r.exec('z'); var m = r.exec('z'); return e[i] + '|' + m.index; }"
);

#[test]
fn exec_leaves_last_index_at_the_end_of_an_empty_match() {
    check(&agrees(EXEC_AFTER_AN_EMPTY_MATCH, "s0|0\ns1|0\ns2|0\n"));
}

const AN_EXEC_RESULT: &str = unit!(
    "function b(i){ var e=a(); var m = /y/.exec('xyz'); return e[i] + '|' + m.input + m.index + typeof m.groups; }"
);

#[test]
fn an_exec_result_carries_its_input_and_groups() {
    check(&agrees(
        AN_EXEC_RESULT,
        "s0|xyz1undefined\ns1|xyz1undefined\ns2|xyz1undefined\n",
    ));
}

const A_NAMED_GROUP: &str =
    unit!("function b(i){ var e=a(); return e[i] + '|' + /(?<k>x)/.exec('x').groups.k; }");

#[test]
fn a_named_group_is_refused() {
    check(&refused(A_NAMED_GROUP, "s0|x\ns1|x\ns2|x\n"));
}

const A_LAST_INDEX_WRITE: &str = unit!(
    "function b(i){ var e=a(); var r = /x/g; r.lastIndex = 1; return e[i] + '|' + r.test('x'); }"
);

#[test]
fn a_last_index_write_is_refused() {
    check(&refused(
        A_LAST_INDEX_WRITE,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const CASE_INSENSITIVE_MATCHING_WITHOUT_U_OVER_NON_ASCII_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (String([/\u017f/i.test('s'), /\u212a/i.test('k'), /[a-z]/i.test('\u212a'), /\u00df/i.test('SS'), /\u0130/i.test('i'), /\w/i.test('\u017f')])); }"##
);

#[test]
fn case_insensitive_matching_without_u_over_non_ascii_is_refused() {
    check(&q_refused(CASE_INSENSITIVE_MATCHING_WITHOUT_U_OVER_NON_ASCII_IS_REFUSED, "s0|false,false,false,false,false,false\ns1|false,false,false,false,false,false\ns2|false,false,false,false,false,false\n"));
}

const A_BRACED_UNICODE_ESCAPE_WITHOUT_U_IS_REFUSED: &str =
    qunit!(r##"function qb(i){ var e=qa(); return e[i] + '|' + (String(/\u{2}/.test('uu'))); }"##);

#[test]
fn a_braced_unicode_escape_without_u_is_refused() {
    check(&q_refused(
        A_BRACED_UNICODE_ESCAPE_WITHOUT_U_IS_REFUSED,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const A_BRACED_UNICODE_ESCAPE_WITHOUT_U_NEVER_MATCHES_ITS_CODE_POINT: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (String([/\u{61}/u.test('a'), /\u{61}/.test('a')])); }"##
);

#[test]
fn a_braced_unicode_escape_without_u_never_matches_its_code_point() {
    check(&q_refused(
        A_BRACED_UNICODE_ESCAPE_WITHOUT_U_NEVER_MATCHES_ITS_CODE_POINT,
        "s0|true,false\ns1|true,false\ns2|true,false\n",
    ));
}

const REGEXP_TO_STRING_ESCAPES_ITS_SOURCE_AND_ORDERS_ITS_FLAGS: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([String(new RegExp('a/b')), String(new RegExp('\n')), String(new RegExp('')), String(new RegExp('a', 'ig'))].join(' ')); }"##
);

#[test]
fn regexp_to_string_escapes_its_source_and_orders_its_flags() {
    check(&decoded(REGEXP_TO_STRING_ESCAPES_ITS_SOURCE_AND_ORDERS_ITS_FLAGS, "s0|/a\\/b/ /\\n/ /(?:)/ /a/gi\ns1|/a\\/b/ /\\n/ /(?:)/ /a/gi\ns2|/a\\/b/ /\\n/ /(?:)/ /a/gi\n"));
}

const REGEXP_TO_STRING_ESCAPES_LINE_TERMINATORS_AND_SLASHES_OUTSIDE_CLASSES: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([String(new RegExp('[/]')), String(new RegExp('a\\\n')), String(new RegExp('\u2028')), String(new RegExp('[\\]/]/')), String(new RegExp('a', 'smig')), String(/x/)].join(' ')); }"##
);

#[test]
fn regexp_to_string_escapes_line_terminators_and_slashes_outside_classes() {
    check(&decoded(REGEXP_TO_STRING_ESCAPES_LINE_TERMINATORS_AND_SLASHES_OUTSIDE_CLASSES, "s0|/[/]/ /a\\n/ /\\u2028/ /[\\]/]\\// /a/gims /x/\ns1|/[/]/ /a\\n/ /\\u2028/ /[\\]/]\\// /a/gims /x/\ns2|/[/]/ /a\\n/ /\\u2028/ /[\\]/]\\// /a/gims /x/\n"));
}

const CASE_INSENSITIVE_ASCII_MATCHING_WITHOUT_U_STILL_DECODES: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([/ABC/i.test('xabcx'), /q.E/i.exec('12QwE3')[0], 'aBc'.replace(/b/gi, '-')].join()); }"##
);

#[test]
fn case_insensitive_ascii_matching_without_u_still_decodes() {
    check(&decoded(
        CASE_INSENSITIVE_ASCII_MATCHING_WITHOUT_U_STILL_DECODES,
        "s0|true,QwE,a-c\ns1|true,QwE,a-c\ns2|true,QwE,a-c\n",
    ));
}

const A_CASE_INSENSITIVE_CLASS_OF_LETTERS_IS_REFUSED: &str =
    qunit!(r##"function qb(i){ var e=qa(); return e[i] + '|' + (/[a-z]+/i.exec('12QwE3')[0]); }"##);

#[test]
fn a_case_insensitive_class_of_letters_is_refused() {
    check(&q_refused(
        A_CASE_INSENSITIVE_CLASS_OF_LETTERS_IS_REFUSED,
        "s0|QwE\ns1|QwE\ns2|QwE\n",
    ));
}

const SEARCH_COMPILES_A_STRING_PATTERN: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (String(['a.c'.search('.'), 'abc'.search(), 'xa*'.search('a*')])); }"##
);

#[test]
fn search_compiles_a_string_pattern() {
    check(&decoded(
        SEARCH_COMPILES_A_STRING_PATTERN,
        "s0|0,0,0\ns1|0,0,0\ns2|0,0,0\n",
    ));
}

const STARTS_WITH_A_REGEXP_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return 'abc'.startsWith(/a/); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn starts_with_a_regexp_is_refused() {
    check(&q_refused(
        STARTS_WITH_A_REGEXP_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const INCLUDES_WITH_A_REGEXP_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return 'abc'.includes(/a/); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn includes_with_a_regexp_is_refused() {
    check(&q_refused(
        INCLUDES_WITH_A_REGEXP_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const ENDS_WITH_A_REGEXP_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return 'a/a/'.endsWith(/a/); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn ends_with_a_regexp_is_refused() {
    check(&q_refused(
        ENDS_WITH_A_REGEXP_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const MATCH_COMPILES_A_STRING_PATTERN: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (['a.c'.match('.')[0], 'abc'.match()[0].length, 'xa*'.match('a*').index, 'abc'.match('z')].join()); }"##
);

#[test]
fn match_compiles_a_string_pattern() {
    check(&decoded(
        MATCH_COMPILES_A_STRING_PATTERN,
        "s0|a,0,0,\ns1|a,0,0,\ns2|a,0,0,\n",
    ));
}
