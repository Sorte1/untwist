mod common;

use common::{eval, eval_with};
use untwist_eval::bail::Bail;
use untwist_eval::budget::Budget;

#[test]
fn regexp_literals_and_constructor_both_work() {
    assert_eq!(
        eval("/ab+/['test']('xabbz')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval(r#"new RegExp('function *\\( *\\)')['test']('function ()')"#),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("new RegExp('[abc]', 'g')['test']('zzz')"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn the_high_preset_self_defence_guard_matches_compact_source() {
    let source = r#"
var head = '\\w+ *\\(\\) *{\\w+ *';
var tail = "['|\"].+['|\"];? *}";
new RegExp(head + tail)['test']((function(){return 'newState';})['toString']())"#;
    assert_eq!(eval(source), Ok("boolean:true".to_string()));
}

#[test]
fn a_lone_surrogate_subject_bails_instead_of_matching() {
    assert_eq!(
        eval("/x/['test'](String.fromCharCode(0xD800))"),
        Err(Bail::Unsupported(
            &"a regular expression over surrogates, which the evaluator matches by code point"
        )
        .to_string())
    );
}

#[test]
fn a_regexp_over_a_surrogate_pair_bails_without_panicking() {
    let source = "
var re = /x*/g;
re['test']('😀');
re['test']('😀')";
    assert!(eval(source).is_err());
}

#[test]
fn replace_supports_strings_regex_globals_and_callbacks() {
    assert_eq!(
        eval("'a-b'['replace']('-', '+')"),
        Ok("string:a+b".to_string())
    );
    assert_eq!(
        eval("'RKDt-x'['replace'](/[RKDtRMZPzEEAyuE=]/g, '')"),
        Ok("string:-x".to_string())
    );
    assert_eq!(
        eval("'%41'['replace'](/%([0-9A-F]{2})/g, function (m, p1) { return p1; })"),
        Ok("string:41".to_string())
    );
    assert_eq!(
        eval("'ab'['replace'](/(a)(b)/, '$2$1')"),
        Ok("string:ba".to_string())
    );
}

#[test]
fn match_and_search_are_available() {
    assert_eq!(eval("'xaby'['search'](/ab/)"), Ok("number:1".to_string()));
    assert_eq!(
        eval("'xaby'['match'](/ab/)[0]"),
        Ok("string:ab".to_string())
    );
    assert_eq!(eval("'xy'['match'](/ab/)"), Ok("null".to_string()));
}

#[test]
fn replace_and_search_over_astral_characters_bail() {
    assert!(eval("'a😀b'['replace'](/😀/, 'x')").is_err());
    assert!(eval("'😀'['search'](/./)").is_err());
}

#[test]
fn a_long_global_replace_trips_the_fuel_cap_under_a_small_budget() {
    let source = format!("'{}'['replace'](/a/g, 'b')", "a".repeat(5000));
    assert_eq!(
        eval_with(&source, Budget::new(200)),
        Err(Bail::Fuel.to_string())
    );
}

#[test]
fn a_global_replace_over_a_long_string_is_linear_not_quadratic() {
    let source = format!("'{}'['replace'](/a/g, 'b')", "a".repeat(5000));
    assert_eq!(
        eval_with(&source, Budget::new(20_000)),
        Ok(format!("string:{}", "b".repeat(5000)))
    );
}

#[test]
fn replace_all_requires_a_global_regexp_but_allows_string_patterns() {
    assert_eq!(
        eval("'aaa'['replaceAll'](/a/, 'b')"),
        Err("throw error:TypeError".to_string())
    );
    assert_eq!(
        eval("'aaa'['replaceAll'](/a/g, 'b')"),
        Ok("string:bbb".to_string())
    );
    assert_eq!(
        eval("'aaa'['replaceAll']('a', 'b')"),
        Ok("string:bbb".to_string())
    );
}

#[test]
fn match_with_global_flag_returns_every_matched_substring() {
    assert_eq!(
        eval("'a1b22c333'['match'](/\\d+/g)['length']"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        eval("'a1b22c333'['match'](/\\d+/g)[0]"),
        Ok("string:1".to_string())
    );
    assert_eq!(
        eval("'a1b22c333'['match'](/\\d+/g)[1]"),
        Ok("string:22".to_string())
    );
    assert_eq!(
        eval("'a1b22c333'['match'](/\\d+/g)[2]"),
        Ok("string:333".to_string())
    );
    assert_eq!(eval("'abc'['match'](/x/g)"), Ok("null".to_string()));
}

#[test]
fn match_without_global_flag_returns_the_exec_shaped_array() {
    assert_eq!(
        eval("'xay'['match'](/a/)['index']"),
        Ok("number:1".to_string())
    );
}

#[test]
fn sticky_flag_requires_the_match_to_start_at_lastindex() {
    assert_eq!(
        eval("var r=/a/y; r['test']('ba')"),
        Ok("boolean:false".to_string())
    );
    assert_eq!(
        eval("var r=/a/y; r['test']('ab')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("var r=/a/y; r['test']('ab'); r['test']('ab')"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn replacement_group_references_beyond_the_pattern_stay_literal() {
    assert_eq!(
        eval("'ab'['replace'](/(a)/, '$2')"),
        Ok("string:$2b".to_string())
    );
}

#[test]
fn constructing_from_an_existing_regexp_is_outside_its_shape() {
    assert!(eval("new RegExp(/ab/gi)['toString']()").is_err());
}

#[test]
fn constructing_with_no_pattern_compiles_the_empty_pattern() {
    assert_eq!(
        eval("new RegExp(undefined)['toString']()"),
        Ok("string:/(?:)/".to_string())
    );
    assert_eq!(
        eval("new RegExp()['toString']()"),
        Ok("string:/(?:)/".to_string())
    );
}

#[test]
fn an_unknown_or_repeated_regexp_flag_is_a_syntax_error() {
    assert_eq!(
        eval("var r; try { new RegExp('a', 'q'); r = 'ok'; } catch (e) { r = 'threw'; } r"),
        Ok("string:threw".to_string())
    );
    assert_eq!(
        eval("var r; try { new RegExp('a', 'gg'); r = 'ok'; } catch (e) { r = 'threw'; } r"),
        Ok("string:threw".to_string())
    );
    assert_eq!(
        eval("var r; try { new RegExp('a', 'gimsuy'); r = 'ok'; } catch (e) { r = 'threw'; } r"),
        Ok("string:ok".to_string())
    );
}

#[test]
fn the_u_and_v_flags_together_throw_a_syntax_error() {
    assert_eq!(
        eval("new RegExp('a', 'uv')"),
        Err("throw error:SyntaxError".to_string())
    );
    assert_eq!(
        eval("RegExp('a', 'vu')"),
        Err("throw error:SyntaxError".to_string())
    );
    assert_eq!(
        eval("new RegExp('a', 'guv')"),
        Err("throw error:SyntaxError".to_string())
    );
}

#[test]
fn the_u_or_v_flag_alone_still_compiles() {
    assert_eq!(
        eval("new RegExp('a', 'u')['test']('a')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("new RegExp('a', 'v')['test']('a')"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn catastrophic_backtracking_runs_out_of_fuel_instead_of_hanging() {
    let source = format!("/^(a+)+b$/['test']('{}c')", "a".repeat(43));
    assert_eq!(eval(&source), Err(Bail::Fuel.to_string()));
}

#[test]
fn catastrophic_backtracking_in_a_global_replace_runs_out_of_fuel() {
    let source = format!("'{}c'['replace'](/(a|aa)+b/g, 'x')", "a".repeat(60));
    assert_eq!(eval(&source), Err(Bail::Fuel.to_string()));
}

#[test]
fn a_bounded_backtracking_search_still_answers() {
    let source = format!("/^(a+)+b$/['test']('{}c')", "a".repeat(12));
    assert_eq!(eval(&source), Ok("boolean:false".to_string()));
}
