mod common;

use common::{eval, eval_with};
use untwist_eval::budget::{Budget, ARRAY_LEN_CAP, STRING_LEN_CAP};

const SHAPE: &str =
    "unsupported construct: a native call outside the argument shape the evaluator models exactly";

#[test]
fn push_shift_rotation_returns_the_moved_element() {
    assert_eq!(
        eval("var e = ['a','b','c']; e['push'](e['shift']()); e[0] + e[2]"),
        Ok("string:ba".to_string())
    );
    assert_eq!(
        eval("var e = []; var got = e['shift'](); typeof got"),
        Ok("string:undefined".to_string())
    );
}

#[test]
fn join_slice_concat_index_of_and_reverse() {
    assert_eq!(eval("['a','b']['join']('|')"), Ok("string:a|b".to_string()));
    assert_eq!(
        eval("[1,2,3]['slice'](1).length"),
        Ok("number:2".to_string())
    );
    assert_eq!(
        eval("[1]['concat']([2,3]).length"),
        Ok("number:3".to_string())
    );
    assert_eq!(
        eval("['a','b']['indexOf']('b')"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("['a','b']['indexOf']('z')"),
        Ok("number:-1".to_string())
    );
    assert_eq!(eval("[1,2]['reverse']()[0]"), Ok("number:2".to_string()));
}

#[test]
fn callback_methods_run_user_functions() {
    assert_eq!(
        eval("[1,2,3]['map'](function (v) { return v * 2; })[2]"),
        Ok("number:6".to_string())
    );
    assert_eq!(
        eval("var s = 0; [1,2,3]['forEach'](function (v) { s += v; }); s"),
        Ok("number:6".to_string())
    );
    assert_eq!(
        eval("[1,2,3]['reduce'](function (a, v) { return a + v; }, 0)"),
        Ok("number:6".to_string())
    );
}

#[test]
fn length_write_truncates_and_splice_removes() {
    assert_eq!(
        eval("var a = [1,2,3]; a.length = 1; a.length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("var a = [1,2,3]; a['splice'](1, 1); a['join']('')"),
        Ok("string:13".to_string())
    );
}

#[test]
fn a_prototype_method_on_a_non_array_is_outside_its_shape() {
    assert_eq!(
        eval("var f = ['a']['push']; f.call(1, 2)"),
        Err(SHAPE.to_string())
    );
}

#[test]
fn an_array_method_without_an_array_receiver_is_outside_its_shape() {
    assert_eq!(eval("var f = ['a']['push']; f(1)"), Err(SHAPE.to_string()));
}

#[test]
fn splice_reentrant_shrink_during_coercion_does_not_panic() {
    assert_eq!(
        eval(
            "var a=[1,2,3,4,5,6,7,8,9,10]; \
             var evil={valueOf:function(){a.length=0;return 3;}}; \
             a['splice'](5, evil).length"
        ),
        Ok("number:0".to_string())
    );
}

#[test]
fn map_refuses_a_callback_that_shrinks_the_receiver() {
    assert_eq!(
        eval("var a=[1,2,3]; a['map'](function (x) { a.length = 0; return x; }).length"),
        Err("unsupported construct: an array hole, which the evaluator does not model".to_string())
    );
}

#[test]
fn includes_uses_same_value_zero_but_index_of_stays_strict() {
    assert_eq!(
        eval("[NaN]['includes'](NaN)"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(eval("[NaN]['indexOf'](NaN)"), Ok("number:-1".to_string()));
    assert_eq!(
        eval("[1,2]['includes']('2')"),
        Ok("boolean:false".to_string())
    );
}

#[test]
fn filter_keeps_matching_elements_in_order() {
    assert_eq!(
        eval("[1,2,3,4]['filter'](function (v) { return v % 2 === 0; })['join'](',')"),
        Ok("string:2,4".to_string())
    );
}

#[test]
fn unshift_several_arguments_preserves_order_and_returns_new_length() {
    assert_eq!(
        eval("var a = [3]; var len = a['unshift'](1, 2); len + ':' + a['join'](',')"),
        Ok("string:3:1,2,3".to_string())
    );
}

#[test]
fn splice_with_negative_start_returns_removed_and_mutates_in_place() {
    assert_eq!(
        eval(
            "var a=[1,2,3,4]; \
             var removed = a['splice'](-3, 2, 'a'); \
             removed['join'](',') + '|' + a['join'](',')"
        ),
        Ok("string:2,3|1,a,4".to_string())
    );
}

#[test]
fn push_past_the_cap_bails_without_growing() {
    assert_eq!(
        eval(&format!(
            "var a = []; a.length = {ARRAY_LEN_CAP}; a['push'](1)"
        )),
        Err("array length cap exceeded".to_string())
    );
}

#[test]
fn unshift_past_the_cap_bails_without_growing() {
    assert_eq!(
        eval(&format!(
            "var a = []; a.length = {ARRAY_LEN_CAP}; a['unshift'](1)"
        )),
        Err("array length cap exceeded".to_string())
    );
}

#[test]
fn splice_inserting_past_the_cap_bails_without_growing() {
    assert_eq!(
        eval(&format!(
            "var a = []; a.length = {ARRAY_LEN_CAP}; a['splice'](0, 0, 'x')"
        )),
        Err("array length cap exceeded".to_string())
    );
}

#[test]
fn concat_past_the_cap_bails_without_growing() {
    assert_eq!(
        eval(&format!(
            "var a = []; a.length = {ARRAY_LEN_CAP}; a['concat']([1])"
        )),
        Err("array length cap exceeded".to_string())
    );
}

#[test]
fn join_past_the_string_cap_bails() {
    let half = STRING_LEN_CAP / 2 + 100_000;
    let source = format!("var s = 'x'['repeat']({half}); var a = [s, s]; a['join']('')");
    assert_eq!(eval(&source), Err("string length cap exceeded".to_string()));
}

#[test]
fn a_long_shift_loop_bails_on_a_small_budget_instead_of_running_it_all() {
    assert_eq!(
        eval_with(
            "var a = []; a.length = 4000; while (a.length) a.shift();",
            Budget::new(100_000)
        ),
        Err("step budget exhausted".to_string())
    );
}

#[test]
fn a_long_shift_loop_completes_with_a_generous_budget() {
    assert_eq!(
        eval_with(
            "var a = []; a.length = 4000; while (a.length) a.shift(); a.length",
            Budget::default()
        ),
        Ok("number:0".to_string())
    );
}

#[test]
fn reverse_slice_and_index_of_are_unaffected_by_the_length_charge() {
    assert_eq!(
        eval("[1,2,3]['reverse']()['join'](',')"),
        Ok("string:3,2,1".to_string())
    );
    assert_eq!(
        eval("[1,2,3,4,5]['slice'](1, 3)['join'](',')"),
        Ok("string:2,3".to_string())
    );
    assert_eq!(eval("[1,2,3]['indexOf'](2)"), Ok("number:1".to_string()));
}
