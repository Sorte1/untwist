mod common;

use common::eval;
use untwist_eval::decode::{Evaluator, InterpEvaluator};

fn ok(value: &str) -> Result<String, String> {
    Ok(format!("string:{value}"))
}

fn decoded(body: &str) -> Result<String, String> {
    let source = format!(
        "function arr() {{ var h = ['hello']; arr = function () {{ return h; }}; return arr(); }}\n\
         function dec(c, d) {{\n  var e = arr();\n  \
         dec = function (f, g) {{ f = f - 0; var h = e[f]; {body} return h; }};\n  \
         return dec(c, d);\n}}\ndec(0);\n"
    );
    common::with_program(&source, |program| {
        let decoder_program = common::decoder_program(program, "arr", &["dec"], None);
        let calls = common::calls_for(program, 0);
        let mut evaluator = InterpEvaluator::default();
        evaluator
            .decode(&decoder_program, &calls)
            .map(|values| common::render_decoded(&values[0]))
            .map_err(|bail| bail.to_string())
    })
}

#[test]
fn a_var_for_in_binding_keeps_the_last_key_after_the_loop() {
    let source = "function f() { var o = {a: 1, b: 2}; for (var k in o) {} return k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("b"));
}

#[test]
fn a_var_for_of_binding_keeps_the_last_item_after_the_loop() {
    let source = "function f() { for (var v of ['p', 'q']) {} return v; } f();";

    let value = eval(source);

    assert_eq!(value, ok("q"));
}

#[test]
fn a_var_for_in_binding_overwrites_the_earlier_var_of_the_same_name() {
    let source = "function f() { var k = 'kept'; for (var k in {a: 1}) {} return k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("a"));
}

#[test]
fn a_var_for_in_binding_over_nothing_keeps_the_earlier_value() {
    let source = "function f() { var k = 'kept'; for (var k in {}) {} return k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("kept"));
}

#[test]
fn a_var_for_in_binding_at_the_top_level_keeps_the_last_key() {
    let source = "for (var k in {a: 1, b: 2}) {}\nk;";

    let value = eval(source);

    assert_eq!(value, ok("b"));
}

#[test]
fn closures_made_in_a_var_for_of_loop_share_the_one_binding() {
    let source = "function f() { var made = []; for (var v of ['p', 'q']) { made.push(function () { return v; }); } return made[0](); } f();";

    let value = eval(source);

    assert_eq!(value, ok("q"));
}

#[test]
fn closures_made_in_a_let_for_of_loop_keep_their_own_binding() {
    let source = "function f() { var made = []; for (let v of ['p', 'q']) { made.push(function () { return v; }); } return made[0](); } f();";

    let value = eval(source);

    assert_eq!(value, ok("p"));
}

#[test]
fn a_decoder_reads_the_last_key_of_a_var_for_in_after_the_loop() {
    let body = "var o = {a: 1, b: 2}; for (var k in o) {} h = h + k;";

    let value = decoded(body);

    assert_eq!(value, ok("hellob"));
}

#[test]
fn a_decoder_reads_the_last_item_of_a_var_for_of_after_the_loop() {
    let body = "for (var v of ['p', 'q']) {} h = h + v;";

    let value = decoded(body);

    assert_eq!(value, ok("helloq"));
}

#[test]
fn a_caught_const_assignment_keeps_the_value() {
    let source = "function f() { const k = 'A'; try { k = 'B'; } catch (err) { return 'T' + k; } return k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("TA"));
}

#[test]
fn an_uncaught_const_assignment_ends_in_a_type_error() {
    let source = "function f() { const c = 1; c = 2; return c; } f();";

    let value = eval(source);

    assert_eq!(value, Err("throw error:TypeError".to_string()));
}

#[test]
fn updating_a_const_throws_and_keeps_the_value() {
    let source = "function f() { const k = 1; try { k++; } catch (err) { return 'T' + k; } return '' + k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn compound_assignment_to_a_const_throws_and_keeps_the_value() {
    let source = "function f() { const k = 1; try { k += 1; } catch (err) { return 'T' + k; } return '' + k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn a_logical_assignment_that_writes_a_const_throws() {
    let source = "function f() { const k = 1; try { k &&= 2; } catch (err) { return 'T' + k; } return '' + k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn a_logical_assignment_that_short_circuits_leaves_a_const_alone() {
    let source = "function f() { const k = 1; k ||= 2; return '' + k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("1"));
}

#[test]
fn a_closure_assigning_an_outer_const_throws() {
    let source = "function f() { const c = 1; function g() { c = 2; } try { g(); } catch (err) { return 'T' + c; } return '' + c; } f();";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn assigning_a_block_const_throws() {
    let source = "function f() { { const c = 1; try { c = 2; } catch (err) { return 'T' + c; } return '' + c; } } f();";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn assigning_a_top_level_const_throws() {
    let source = "const c = 1;\nvar r = 'no';\ntry { c = 2; } catch (err) { r = 'T' + c; }\nr;";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn a_function_assigning_a_top_level_const_throws() {
    let source = "const c = 1;\nfunction g() { c = 2; }\nvar r = 'no';\ntry { g(); } catch (err) { r = 'T' + c; }\nr;";

    let value = eval(source);

    assert_eq!(value, ok("T1"));
}

#[test]
fn destructuring_into_a_const_is_refused_and_never_assigns() {
    let source = "function f() { const c = 1; try { [c] = [2]; } catch (err) { return 'T' + c; } return '' + c; } f();";

    let value = eval(source);

    assert_eq!(
        value,
        Err("unsupported construct: destructuring assignment target".to_string())
    );
}

#[test]
fn assigning_a_const_for_of_binding_throws() {
    let source = "function f() { try { for (const v of [1]) { v = 2; } } catch (err) { return 'T'; } return 'no'; } f();";

    let value = eval(source);

    assert_eq!(value, ok("T"));
}

#[test]
fn a_const_for_of_binding_is_fresh_in_every_iteration() {
    let source =
        "function f() { var s = ''; for (const v of ['p', 'q']) { s += v; } return s; } f();";

    let value = eval(source);

    assert_eq!(value, ok("pq"));
}

#[test]
fn a_let_stays_assignable() {
    let source = "function f() { let k = 'A'; k = 'B'; return k; } f();";

    let value = eval(source);

    assert_eq!(value, ok("B"));
}

#[test]
fn a_decoder_assigning_a_const_takes_the_catch_path() {
    let body = "const k = 'A'; try { k = 'B'; } catch (err) { h = h + 'T'; } h = h + k;";

    let value = decoded(body);

    assert_eq!(value, ok("helloTA"));
}

#[test]
fn a_decoder_updating_a_const_takes_the_catch_path() {
    let body = "const k = 1; try { k++; } catch (err) { h = h + 'T'; } h = h + k;";

    let value = decoded(body);

    assert_eq!(value, ok("helloT1"));
}

#[test]
fn a_global_replace_resets_last_index() {
    let source = "var re = /o/g; re.test('o'); var s = 'hello'.replace(re, '0'); s + re.test('o');";

    let value = eval(source);

    assert_eq!(value, ok("hell0true"));
}

#[test]
fn a_global_replace_all_resets_last_index() {
    let source =
        "var re = /o/g; re.test('o'); var s = 'hello'.replaceAll(re, '0'); s + re.test('o');";

    let value = eval(source);

    assert_eq!(value, ok("hell0true"));
}

#[test]
fn a_global_match_resets_last_index() {
    let source = "var re = /o/g; re.test('o'); '' + 'o'.match(re).length + re.test('o');";

    let value = eval(source);

    assert_eq!(value, ok("1true"));
}

#[test]
fn a_global_match_without_a_hit_resets_last_index() {
    let source = "var re = /o/g; re.test('o'); '' + 'x'.match(re) + re.test('o');";

    let value = eval(source);

    assert_eq!(value, ok("nulltrue"));
}

#[test]
fn a_global_replace_starts_its_callbacks_at_last_index_zero() {
    let source = "var re = /o/g; re.test('o'); var seen = ''; 'oo'.replace(re, function () { seen += re.test('o'); return 'x'; }); seen;";

    let value = eval(source);

    assert_eq!(value, ok("truefalse"));
}

#[test]
fn a_sticky_replace_starts_at_last_index_and_moves_it() {
    let source =
        "var re = /l/y; re.test('l'); var s = 'hlllo'.replace(re, '0'); s + re.test('__l');";

    let value = eval(source);

    assert_eq!(value, ok("h0llotrue"));
}

#[test]
fn a_sticky_replace_that_misses_resets_last_index() {
    let source = "var re = /l/y; re.test('l'); var s = 'hxllo'.replace(re, '0'); s + re.test('l');";

    let value = eval(source);

    assert_eq!(value, ok("hxllotrue"));
}

#[test]
fn a_sticky_match_starts_at_last_index_and_moves_it() {
    let source =
        "var re = /l/y; re.test('l'); var m = 'hlllo'.match(re); '' + m.index + re.test('__l');";

    let value = eval(source);

    assert_eq!(value, ok("1true"));
}

#[test]
fn a_sticky_match_that_misses_resets_last_index() {
    let source = "var re = /l/y; re.test('l'); var m = 'hxllo'.match(re); '' + m + re.test('l');";

    let value = eval(source);

    assert_eq!(value, ok("nulltrue"));
}

#[test]
fn a_sticky_search_starts_at_zero_and_leaves_last_index_alone() {
    let source = "var re = /l/y; re.test('l'); var at = 'lxl'.search(re); '' + at + re.test('_l');";

    let value = eval(source);

    assert_eq!(value, ok("0true"));
}

#[test]
fn a_decoder_global_replace_resets_last_index() {
    let body = "var re = /o/g; re.test('o'); h = h.replace(re, '0'); h = h + re.test('o');";

    let value = decoded(body);

    assert_eq!(value, ok("hell0true"));
}

#[test]
fn a_decoder_global_match_resets_last_index() {
    let body = "var re = /o/g; re.test('o'); h = h + 'o'.match(re).length + re.test('o');";

    let value = decoded(body);

    assert_eq!(value, ok("hello1true"));
}

const UNSOUND_DEFAULT: &str = "unsupported construct: a parameter default that reads its own parameter, a later one or a binding of the function body";

fn refused_default() -> Result<String, String> {
    Err(UNSOUND_DEFAULT.to_string())
}

#[test]
fn a_default_closure_over_a_body_var_is_refused() {
    let source = "var x = 'o';\nfunction q(a = function () { return x; }) { var x = 'i'; return a(); }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_closure_over_a_body_function_is_refused() {
    let source = "var x = 'o';\nfunction q(a = function () { return typeof x; }) { function x() {} return a(); }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_arrow_over_a_body_let_is_refused() {
    let source = "var x = 'o';\nfunction q(a = () => x) { let x = 'i'; return a(); }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_closure_over_a_var_of_a_nested_block_is_refused() {
    let source =
        "var x = 'o';\nfunction q(a = () => x) { if (a) { var x = 'i'; } return a(); }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_reading_a_later_parameter_is_refused() {
    let source = "var b = 'o';\nfunction q(a = b, b) { return a; }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_reading_its_own_parameter_is_refused() {
    let source = "var a = 'o';\nfunction q(a = a) { return a; }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn a_default_closure_over_a_later_parameter_is_refused() {
    let source = "var b = 'o';\nfunction q(a = (() => b)(), b) { return a; }\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn an_arrow_default_reading_a_later_parameter_is_refused() {
    let source = "var b = 'o';\nvar q = (a = b, b) => a;\nq();";

    let value = eval(source);

    assert_eq!(value, refused_default());
}

#[test]
fn an_unsound_default_that_is_not_evaluated_is_not_refused() {
    let source = "var b = 'o';\nfunction q(a = b, b) { return a; }\nq('given');";

    let value = eval(source);

    assert_eq!(value, ok("given"));
}

#[test]
fn a_default_reading_an_earlier_parameter_reads_it() {
    let source = "function q(a, b = a + '!') { return b; }\nq('e');";

    let value = eval(source);

    assert_eq!(value, ok("e!"));
}

#[test]
fn a_default_closure_over_an_earlier_parameter_reads_it() {
    let source = "function q(a, b = function () { return a; }) { return b(); }\nq('e');";

    let value = eval(source);

    assert_eq!(value, ok("e"));
}

#[test]
fn a_default_naming_a_body_var_directly_reads_the_outer_binding() {
    let source = "var x = 'outer';\nfunction f(a = x) { var x = 1; return a; }\nf();";

    let value = eval(source);

    assert_eq!(value, ok("outer"));
}

#[test]
fn a_default_closure_that_shadows_the_body_name_is_not_refused() {
    let source =
        "function q(a = function (x) { return x; }) { var x = 'i'; return a('own'); }\nq();";

    let value = eval(source);

    assert_eq!(value, ok("own"));
}

#[test]
fn a_decoder_with_a_default_closure_over_a_body_var_is_refused() {
    let body =
        "var x = 'o'; function q(a = function () { return x; }) { var x = 'i'; return a(); } h = h + q();";

    let value = decoded(body);

    assert_eq!(value, refused_default());
}

#[test]
fn a_decoder_with_a_default_reading_a_later_parameter_is_refused() {
    let body = "var b = 'o'; function q(a = b, b) { return a; } try { h = h + q(); } catch (err) { h = h + 'T'; }";

    let value = decoded(body);

    assert_eq!(value, refused_default());
}

#[test]
fn a_string_key_spelled_like_a_symbol_marker_is_not_to_primitive() {
    let body = "var o = {'@@toPrimitive': function () { return 'X'; }, toString: function () { return 'Y'; }}; h = h + o;";

    let value = decoded(body);

    assert_eq!(value, ok("helloY"));
}

#[test]
fn a_string_key_spelled_like_a_symbol_marker_is_not_to_primitive_in_a_script() {
    let source = "var o = {'@@toPrimitive': function () { return 'X'; }, toString: function () { return 'Y'; }};\n'' + o;";

    let value = eval(source);

    assert_eq!(value, ok("Y"));
}

#[test]
fn stringify_does_not_drop_a_function_that_has_to_json() {
    let source =
        "var f = function () {}; f.toJSON = function () { return 1; }; JSON.stringify({a: f});";

    let value = eval(source);

    assert_eq!(value, Err("unsupported construct: toJSON".to_string()));
}

#[test]
fn stringify_of_a_function_that_has_to_json_is_not_undefined() {
    let source =
        "var f = function () {}; f.toJSON = function () { return 1; }; '' + JSON.stringify(f);";

    let value = eval(source);

    assert_eq!(value, Err("unsupported construct: toJSON".to_string()));
}

#[test]
fn stringify_still_drops_a_plain_function() {
    let source = "var f = function () {}; JSON.stringify({a: f, b: 1}) + JSON.stringify(f);";

    let value = eval(source);

    assert_eq!(value, ok("{\"b\":1}undefined"));
}
