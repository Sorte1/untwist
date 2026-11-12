mod common;

use common::{eval, eval_with_notes};

#[test]
fn bind_to_string_bails_where_node_prints_native_code() {
    assert_eq!(
        eval("function t() { return 1; } t['bind'](null)['toString']()"),
        Err("unsupported construct: the source text of a bound function".to_string())
    );
}

#[test]
fn math_random_is_refused_rather_than_stubbed() {
    let (result, notes) = eval_with_notes("Math['random']()");
    assert!(result.is_err());
    assert!(notes.is_empty());
}

#[test]
fn a_regexp_over_surrogates_bails_rather_than_match_by_code_point() {
    assert!(eval("'\\u{1F600}'['replace'](/./g, 'x')").is_err());
    assert!(eval("'\\u{1F600}'['replace'](/[\\s\\S]/g, 'x')").is_err());
}

#[test]
fn a_for_let_head_gives_each_iteration_its_own_binding() {
    assert_eq!(
        eval("var fs = []; for (let i = 0; i < 3; i++) { fs['push'](function () { return i; }); } fs[0]() + '' + fs[1]()"),
        Ok("string:01".to_string())
    );
}

#[test]
fn there_is_no_temporal_dead_zone_for_let() {
    assert_eq!(
        eval("function f() { var r = typeof x; let x = 1; return r; } f()"),
        Ok("string:undefined".to_string())
    );
    assert_eq!(
        eval("var x = 'outer'; function f() { var r = x; let x = 1; return r; } f()"),
        Ok("string:outer".to_string())
    );
}

#[test]
fn math_round_is_exact_and_a_to_fixed_tie_is_refused() {
    assert_eq!(
        eval("Math['round'](0.49999999999999994)"),
        Ok("number:0".to_string())
    );
    assert!(eval("(2.5)['toFixed'](0)").is_err());
}

#[test]
fn an_error_objects_prototype_members_bail() {
    for member in ["toString", "name", "message"] {
        let source = format!("var r; try {{ null['x'] }} catch (e) {{ r = e['{member}']; }} r");
        assert_eq!(
            eval(&source),
            Err(
                "unsupported construct: a member of Error.prototype, which the evaluator does not model"
                    .to_string()
            ),
            "{member}"
        );
    }
    assert!(eval("var r; try { null['x'] } catch (e) { r = String(e); } r").is_err());
}

#[test]
fn the_object_constructor_bails_and_the_error_globals_are_absent() {
    assert!(eval("({})['constructor']")
        .is_err_and(|bail| bail.contains("a constructor the evaluator does not model")));
    assert_eq!(
        eval("typeof TypeError"),
        Err("unresolved global binding: TypeError".to_string())
    );
}

#[test]
fn new_on_a_bound_function_constructs_its_target() {
    assert_eq!(
        eval("function t() { this['a'] = 1; } var b = t['bind'](null); var o = new b(); o['a']"),
        Ok("number:1".to_string())
    );
}

#[test]
fn a_regexps_last_index_bails() {
    assert_eq!(
        eval("var re = /a/g; re['test']('aa'); re['lastIndex']"),
        Err(
            "unsupported construct: a property node has that the evaluator does not model"
                .to_string()
        )
    );
}

#[test]
fn typeof_a_hostile_global_still_works_although_every_access_bails() {
    assert_eq!(
        eval("typeof window"),
        Err("hostile global window reached".to_string())
    );
    assert_eq!(eval("typeof process"), Ok("string:object".to_string()));
    assert_eq!(eval("typeof eval"), Ok("string:function".to_string()));
    assert_eq!(
        eval("window['foo']"),
        Err("hostile global window reached".to_string())
    );
}

#[test]
fn splitting_a_subject_with_surrogates_on_a_regexp_bails() {
    assert!(eval("'\\u{1F600}'['split'](/(?:)/)['length']").is_err());
    assert!(eval("'a\\u{1F600}b'['split'](/(?:)/)['length']").is_err());
}

#[test]
fn a_function_declared_in_a_sloppy_block_bails() {
    assert_eq!(
        eval("var g = function () { return 1; }; { function g() { return 2; } } g()"),
        Err("unsupported construct: a function declaration outside a function body's top level, whose Annex B binding the evaluator does not model".to_string())
    );
}

#[test]
fn an_unresolved_assignment_in_strict_code_throws_a_reference_error() {
    let programs = [
        "'use strict'; function f() { x = 1; } var r; try { f(); r = typeof x; } catch (e) { r = 'threw'; } r",
        "function f() { 'use strict'; x = 1; } var r; try { f(); r = typeof x; } catch (e) { r = 'threw'; } r",
    ];
    for program in programs {
        assert_eq!(eval(program), Ok("string:threw".to_string()), "{program}");
    }
    assert_eq!(
        eval("function f() { x = 1; } f(); typeof x"),
        Ok("string:number".to_string())
    );
}

#[test]
fn a_destructured_for_let_head_is_refused() {
    assert!(eval("var t = 0; for (let k = 0, [j] = [5]; k < 2; k++) { t += j; } t").is_err());
}
