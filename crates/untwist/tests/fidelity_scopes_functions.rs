mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused, refused};
use refused_rows::check;

const THE_SOURCE_OF_A_BUILT_IN: &str =
    unit!("function b(i){ var e=a(); return e[i] + '|' + Math.max.toString(); }");

#[test]
fn the_source_text_of_a_built_in_is_refused() {
    check(&refused(
        THE_SOURCE_OF_A_BUILT_IN,
        "s0|function max() { [native code] }\ns1|function max() { [native code] }\ns2|function max() { [native code] }\n",
    ));
}

const A_SLOPPY_WRITE_TO_A_PRIMITIVE: &str = unit!(
    "function b(i){ var e=a(); var r; try { var s = 'abc'; s.x = 1; r = 'ok'; } catch (z) { r = 'caught'; } return e[i] + '|' + r; }"
);

#[test]
fn a_sloppy_property_write_on_a_primitive_is_refused() {
    check(&refused(
        A_SLOPPY_WRITE_TO_A_PRIMITIVE,
        "s0|ok\ns1|ok\ns2|ok\n",
    ));
}

const THE_PROTOTYPE_OF_A_FUNCTION: &str = unit!(
    "function b(i){ var e=a(); return e[i] + '|' + (Object.getPrototypeOf(function(){}) === null); }"
);

#[test]
fn the_prototype_of_a_function_is_decoded_as_function_prototype() {
    check(&decoded(
        THE_PROTOTYPE_OF_A_FUNCTION,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const AN_ARRAY_INSTANCE_OF_OBJECT: &str =
    unit!("function b(i){ var e=a(); return e[i] + '|' + ([] instanceof Object); }");

#[test]
fn instanceof_through_the_array_prototype_is_decoded() {
    check(&decoded(
        AN_ARRAY_INSTANCE_OF_OBJECT,
        "s0|true\ns1|true\ns2|true\n",
    ));
}

const TYPEOF_IN_THE_TEMPORAL_DEAD_ZONE_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var r; try { r = typeof x; } catch (z) { r = z.name; } let x = 1; return e[i] + '|' + r; }"##
);

#[test]
fn typeof_in_the_temporal_dead_zone_is_refused() {
    check(&refused(
        TYPEOF_IN_THE_TEMPORAL_DEAD_ZONE_IS_REFUSED,
        "s0|ReferenceError\ns1|ReferenceError\ns2|ReferenceError\n",
    ));
}

const A_FOR_LET_LOOP_GIVES_EACH_ITERATION_ITS_OWN_BINDING: &str = unit!(
    r##"function b(i){ var e=a(); var fs = []; for (let k = 0; k < 3; k++) fs.push(function(){ return k; }); var gs = []; for (var j = 0; j < 3; j++) gs.push(function(){ return j; }); return e[i] + '|' + fs[0]() + fs[2]() + gs[0](); }"##
);

#[test]
fn a_for_let_loop_gives_each_iteration_its_own_binding() {
    check(&agrees(
        A_FOR_LET_LOOP_GIVES_EACH_ITERATION_ITS_OWN_BINDING,
        "s0|023\ns1|023\ns2|023\n",
    ));
}

const A_READ_IN_THE_DEAD_ZONE: &str = unit!(
    "function b(i){ var e=a(); var q = 'o'; var r = (function(){ try { return q; } catch (z) { return 'tdz'; } let q = 1; })(); return e[i] + '|' + r; }"
);

#[test]
fn a_read_in_the_temporal_dead_zone_is_refused() {
    check(&refused(
        A_READ_IN_THE_DEAD_ZONE,
        "s0|tdz\ns1|tdz\ns2|tdz\n",
    ));
}

const A_FOR_IN_LET_CLOSURE: &str = unit!(
    "function b(i){ var e=a(); var fs = []; for (let k in {x: 1, y: 2}) fs.push(function(){ return k; }); return e[i] + '|' + fs[0]() + fs[1](); }"
);

#[test]
fn a_for_in_let_loop_gives_each_iteration_its_own_binding() {
    check(&agrees(A_FOR_IN_LET_CLOSURE, "s0|xy\ns1|xy\ns2|xy\n"));
}

const A_FOR_OF_HEAD_READING_ITS_OWN_BINDING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ var x = ['outer']; var r = []; try { for (let x of x) r.push(x); } catch (z) { r.push('caught'); } return r.join(); })()); }"##
);

#[test]
fn a_for_of_head_reading_its_own_binding_is_refused() {
    check(&refused(
        A_FOR_OF_HEAD_READING_ITS_OWN_BINDING_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_TYPEOF_IN_A_FOR_OF_HEAD_OF_ITS_OWN_BINDING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ var x = 'outer'; var r = []; try { for (let x of [typeof x]) r.push(x); } catch (z) { r.push('caught'); } return r.join(); })()); }"##
);

#[test]
fn a_typeof_in_a_for_of_head_of_its_own_binding_is_refused() {
    check(&refused(
        A_TYPEOF_IN_A_FOR_OF_HEAD_OF_ITS_OWN_BINDING_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_FOR_IN_HEAD_READING_ITS_OWN_BINDING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ var x = { a: 1 }; var r = []; try { for (let x in x) r.push(x); } catch (z) { r.push('caught'); } return r.join(); })()); }"##
);

#[test]
fn a_for_in_head_reading_its_own_binding_is_refused() {
    check(&refused(
        A_FOR_IN_HEAD_READING_ITS_OWN_BINDING_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_CLOSURE_IN_A_FOR_OF_HEAD_OVER_ITS_OWN_BINDING_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ var r = []; try { for (let y of (function(){ return [typeof y]; })()) r.push(y); } catch (z) { r.push('caught'); } return r.join(); })()); }"##
);

#[test]
fn a_closure_in_a_for_of_head_over_its_own_binding_is_refused() {
    check(&refused(
        A_CLOSURE_IN_A_FOR_OF_HEAD_OVER_ITS_OWN_BINDING_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_FUNCTION_EXPRESSION_NAME_IGNORES_A_SLOPPY_WRITE: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function f(){ f = 1; return typeof f; })()); }"##
);

#[test]
fn a_function_expression_name_ignores_a_sloppy_write() {
    check(&decoded(
        A_FUNCTION_EXPRESSION_NAME_IGNORES_A_SLOPPY_WRITE,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const A_FUNCTION_EXPRESSION_NAME_THROWS_ON_A_STRICT_WRITE: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ try { return (function f(){ 'use strict'; f = 1; return typeof f; })(); } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn a_function_expression_name_throws_on_a_strict_write() {
    check(&decoded(
        A_FUNCTION_EXPRESSION_NAME_THROWS_ON_A_STRICT_WRITE,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_METHOD_HAS_NO_PROTOTYPE: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (typeof ({ m(){} }).m.prototype); }"##
);

#[test]
fn a_method_has_no_prototype() {
    check(&q_refused(
        A_METHOD_HAS_NO_PROTOTYPE,
        "s0|undefined\ns1|undefined\ns2|undefined\n",
    ));
}

const AN_ASYNC_FUNCTION_CALL_RETURNS_A_PROMISE: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ('' + (async function(){ return 1; })()); }"##
);

#[test]
fn an_async_function_call_returns_a_promise() {
    check(&q_refused(
        AN_ASYNC_FUNCTION_CALL_RETURNS_A_PROMISE,
        "s0|[object Promise]\ns1|[object Promise]\ns2|[object Promise]\n",
    ));
}

const A_GENERATOR_CALL_RETURNS_AN_ITERATOR: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (typeof (function*(){ return 1; })() + ',' + String((function*(){ return 1; })())); }"##
);

#[test]
fn a_generator_call_returns_an_iterator() {
    check(&q_refused(A_GENERATOR_CALL_RETURNS_AN_ITERATOR, "s0|object,[object Generator]\ns1|object,[object Generator]\ns2|object,[object Generator]\n"));
}

const CONSTRUCTING_AN_ASYNC_FUNCTION_OR_AN_ARROW_THROWS: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var r = []; try { new (async function(){})(); r.push('made'); } catch (z) { r.push('caught'); } try { new (() => 1)(); r.push('made'); } catch (z) { r.push('caught'); } return r.join(); })()); }"##
);

#[test]
fn constructing_an_async_function_or_an_arrow_throws() {
    check(&decoded(
        CONSTRUCTING_AN_ASYNC_FUNCTION_OR_AN_ARROW_THROWS,
        "s0|caught,caught\ns1|caught,caught\ns2|caught,caught\n",
    ));
}

const A_FUNCTION_EXPRESSION_NAME_SURVIVES_AN_UPDATE: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function f(){ f++; f += 1; return typeof f; })()); }"##
);

#[test]
fn a_function_expression_name_survives_an_update() {
    check(&decoded(
        A_FUNCTION_EXPRESSION_NAME_SURVIVES_AN_UPDATE,
        "s0|function\ns1|function\ns2|function\n",
    ));
}

const A_VAR_OF_THE_SAME_NAME_SHADOWS_A_FUNCTION_EXPRESSION_NAME: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function f(){ var f = 1; f = 2; return typeof f + f; })()); }"##
);

#[test]
fn a_var_of_the_same_name_shadows_a_function_expression_name() {
    check(&decoded(
        A_VAR_OF_THE_SAME_NAME_SHADOWS_A_FUNCTION_EXPRESSION_NAME,
        "s0|number2\ns1|number2\ns2|number2\n",
    ));
}
