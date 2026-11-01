mod refused_rows;
#[macro_use]
mod fidelity;

use fidelity::{agrees, decoded, q_refused, refused};
use refused_rows::check;

const A_GENERIC_ARRAY_METHOD_RECEIVER_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [1].concat([2, [3]], 4, 'ab').length + ',' + [].concat.call(1, 2).length; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_generic_array_method_receiver_is_refused() {
    check(&refused(
        A_GENERIC_ARRAY_METHOD_RECEIVER_IS_REFUSED,
        "s0|5,2\ns1|5,2\ns2|5,2\n",
    ));
}

const A_CAUGHT_REDUCE_OF_AN_EMPTY_ARRAY_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [].reduce(function(p, q){ return p + q; }); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_caught_reduce_of_an_empty_array_is_refused() {
    check(&refused(A_CAUGHT_REDUCE_OF_AN_EMPTY_ARRAY_IS_REFUSED, "s0|E:TypeError:Reduce of empty array with no initial value\ns1|E:TypeError:Reduce of empty array with no initial value\ns2|E:TypeError:Reduce of empty array with no initial value\n"));
}

const A_MISSING_ARRAY_METHOD_CALLED_IN_A_TRY_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return [1, [2]].flatMap(function(v){ return v; }).length; } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn a_missing_array_method_called_in_a_try_is_refused() {
    check(&refused(
        A_MISSING_ARRAY_METHOD_CALLED_IN_A_TRY_IS_REFUSED,
        "s0|2\ns1|2\ns2|2\n",
    ));
}

const A_FEATURE_TEST_OF_A_MISSING_BUILT_IN_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((typeof Object.getOwnPropertyDescriptor === 'function') ? 'native' : 'poly'); }"##
);

#[test]
fn a_feature_test_of_a_missing_built_in_is_refused() {
    check(&refused(
        A_FEATURE_TEST_OF_A_MISSING_BUILT_IN_IS_REFUSED,
        "s0|native\ns1|native\ns2|native\n",
    ));
}

const PROTO_TESTED_WITH_IN_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return ('__proto__' in {}) + ',' + ({}).hasOwnProperty('__proto__'); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn proto_tested_with_in_is_refused() {
    check(&refused(
        PROTO_TESTED_WITH_IN_IS_REFUSED,
        "s0|true,false\ns1|true,false\ns2|true,false\n",
    ));
}

const A_MISSING_OBJECT_STATIC_CALLED_IN_A_TRY_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return Object.getOwnPropertyDescriptor({x: 'V'}, 'x').value; } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn a_missing_object_static_called_in_a_try_is_refused() {
    check(&refused(
        A_MISSING_OBJECT_STATIC_CALLED_IN_A_TRY_IS_REFUSED,
        "s0|V\ns1|V\ns2|V\n",
    ));
}

const AN_ERROR_STACK_READ_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { null.x; } catch (z) { return typeof z.stack; } })()); }"##
);

#[test]
fn an_error_stack_read_is_refused() {
    check(&refused(
        AN_ERROR_STACK_READ_IS_REFUSED,
        "s0|string\ns1|string\ns2|string\n",
    ));
}

const AN_ARROW_IS_NOT_A_CONSTRUCTOR: &str = unit!(
    "function b(i){ var e=a(); var r; try { var f = () => 1; r = typeof new f(); } catch (z) { r = 'caught'; } return e[i] + '|' + r; }"
);

#[test]
fn constructing_an_arrow_throws() {
    check(&agrees(
        AN_ARROW_IS_NOT_A_CONSTRUCTOR,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const A_GETTER_IN_AN_OBJECT_LITERAL_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return ({get x(){ return 'G'; }}).x; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_getter_in_an_object_literal_is_refused() {
    check(&refused(
        A_GETTER_IN_AN_OBJECT_LITERAL_IS_REFUSED,
        "s0|G\ns1|G\ns2|G\n",
    ));
}

const A_SETTER_IN_AN_OBJECT_LITERAL_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var o = {set x(v){ this.y = v + '!'; }}; o.x = 'S'; return e[i] + '|' + o.y; }"##
);

#[test]
fn a_setter_in_an_object_literal_is_refused() {
    check(&refused(
        A_SETTER_IN_AN_OBJECT_LITERAL_IS_REFUSED,
        "s0|S!\ns1|S!\ns2|S!\n",
    ));
}

const A_SETTER_ONLY_PROPERTY_READ_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return ({set x(v){}}).x; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_setter_only_property_read_is_refused() {
    check(&refused(
        A_SETTER_ONLY_PROPERTY_READ_IS_REFUSED,
        "s0|undefined\ns1|undefined\ns2|undefined\n",
    ));
}

const A_PROTO_SHORTHAND_IS_AN_OWN_DATA_PROPERTY: &str = unit!(
    r##"function b(i){ var e=a(); var __proto__ = {x: 'P'}; var o = {__proto__}; return e[i] + '|' + o.x + Object.keys(o).join(); }"##
);

#[test]
fn a_proto_shorthand_is_an_own_data_property() {
    check(&agrees(
        A_PROTO_SHORTHAND_IS_AN_OWN_DATA_PROPERTY,
        "s0|undefined__proto__\ns1|undefined__proto__\ns2|undefined__proto__\n",
    ));
}

const ITERATING_AN_ARRAY_WITH_HOLES_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var n = 0; [1, , 3].forEach(function(){ n++; }); var m = [1, , 3].map(function(v){ return v * 2; }); return e[i] + '|' + n + JSON.stringify(m) + m.length + (1 in m); }"##
);

#[test]
fn iterating_an_array_with_holes_is_refused() {
    check(&refused(
        ITERATING_AN_ARRAY_WITH_HOLES_IS_REFUSED,
        "s0|2[2,null,6]3false\ns1|2[2,null,6]3false\ns2|2[2,null,6]3false\n",
    ));
}

const THE_KEYS_OF_AN_ARRAY_WITH_HOLES_ARE_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return Object.keys([5,,6]).join(); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn the_keys_of_an_array_with_holes_are_refused() {
    check(&refused(
        THE_KEYS_OF_AN_ARRAY_WITH_HOLES_ARE_REFUSED,
        "s0|0,2\ns1|0,2\ns2|0,2\n",
    ));
}

const IN_ON_AN_ARRAY_HOLE_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return (1 in [5,,6]) + ',' + [5,,6].length + ',' + [,].length; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn in_on_an_array_hole_is_refused() {
    check(&refused(
        IN_ON_AN_ARRAY_HOLE_IS_REFUSED,
        "s0|false,3,1\ns1|false,3,1\ns2|false,3,1\n",
    ));
}

const A_WRITE_PAST_THE_END: &str =
    unit!("function b(i){ var e=a(); var p = []; p[2] = 'x'; return e[i] + '|' + (1 in p); }");

#[test]
fn a_write_past_the_end_of_an_array_is_refused() {
    check(&refused(
        A_WRITE_PAST_THE_END,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const A_DELETED_ELEMENT: &str =
    unit!("function b(i){ var e=a(); var p = [1, 2]; delete p[0]; return e[i] + '|' + (0 in p); }");

#[test]
fn deleting_an_array_element_is_refused() {
    check(&refused(
        A_DELETED_ELEMENT,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const A_LENGTH_THAT_GROWS: &str =
    unit!("function b(i){ var e=a(); var p = []; p.length = 2; return e[i] + '|' + (0 in p); }");

#[test]
fn growing_an_array_through_its_length_is_refused() {
    check(&refused(
        A_LENGTH_THAT_GROWS,
        "s0|false\ns1|false\ns2|false\n",
    ));
}

const OBJECT_CREATE_WITH_DESCRIPTORS_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return Object.create({}, {x: {value: 'V', enumerable: true}}).x; } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn object_create_with_descriptors_is_refused() {
    check(&refused(
        OBJECT_CREATE_WITH_DESCRIPTORS_IS_REFUSED,
        "s0|V\ns1|V\ns2|V\n",
    ));
}

const OBJECT_ASSIGN_OF_AN_OWN_PROTO_KEY_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var s = {['__proto__']: {x: 'P'}}; var t = Object.assign({}, s); return e[i] + '|' + t.x + Object.keys(t).length; }"##
);

#[test]
fn object_assign_of_an_own_proto_key_is_refused() {
    check(&refused(
        OBJECT_ASSIGN_OF_AN_OWN_PROTO_KEY_IS_REFUSED,
        "s0|P0\ns1|P0\ns2|P0\n",
    ));
}

const A_NULL_PROTOTYPE_OBJECT_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); return e[i] + '|' + ((function(){ try { return ('toString' in Object.create(null)) + ',' + ('toString' in {}); } catch (z) { return 'E:' + z.name + ':' + z.message; } })()); }"##
);

#[test]
fn a_null_prototype_object_is_refused() {
    check(&refused(
        A_NULL_PROTOTYPE_OBJECT_IS_REFUSED,
        "s0|false,true\ns1|false,true\ns2|false,true\n",
    ));
}

const KEYS_OF_NULL: &str = unit!(
    "function b(i){ var e=a(); var r; try { r = Object.keys(null).length; } catch (z) { r = 'caught'; } return e[i] + '|' + r; }"
);

#[test]
fn object_keys_of_null_throws() {
    check(&agrees(KEYS_OF_NULL, "s0|caught\ns1|caught\ns2|caught\n"));
}

const A_JSON_PROTO_KEY_REACHING_OBJECT_ASSIGN_IS_REFUSED: &str = unit!(
    r##"function b(i){ var e=a(); var t = Object.assign({}, JSON.parse('{"__proto__": {"x": "P"}}')); return e[i] + '|' + t.x; }"##
);

#[test]
fn a_json_proto_key_reaching_object_assign_is_refused() {
    check(&refused(
        A_JSON_PROTO_KEY_REACHING_OBJECT_ASSIGN_IS_REFUSED,
        "s0|P\ns1|P\ns2|P\n",
    ));
}

const OBJECT_KEYS_LISTS_INTEGER_KEYS_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (Object.keys({ b: 1, 2: 1, a: 1, 1: 1 }).join()); }"##
);

#[test]
fn object_keys_lists_integer_keys_first() {
    check(&decoded(
        OBJECT_KEYS_LISTS_INTEGER_KEYS_FIRST,
        "s0|1,2,b,a\ns1|1,2,b,a\ns2|1,2,b,a\n",
    ));
}

const JSON_PARSE_KEYS_LIST_INTEGER_KEYS_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (Object.keys(JSON.parse('{"b":1,"1":2}')).join()); }"##
);

#[test]
fn json_parse_keys_list_integer_keys_first() {
    check(&decoded(
        JSON_PARSE_KEYS_LIST_INTEGER_KEYS_FIRST,
        "s0|1,b\ns1|1,b\ns2|1,b\n",
    ));
}

const COMPUTED_INTEGER_KEYS_ARE_LISTED_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var k = 1; var o = { b: 1, [k + 1]: 1, [k]: 1 }; return Object.keys(o).join(); })()); }"##
);

#[test]
fn computed_integer_keys_are_listed_first() {
    check(&decoded(
        COMPUTED_INTEGER_KEYS_ARE_LISTED_FIRST,
        "s0|1,2,b\ns1|1,2,b\ns2|1,2,b\n",
    ));
}

const ONLY_CANONICAL_INTEGER_KEYS_ARE_LISTED_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (Object.keys({ b: 1, 1.5: 1, 1e3: 1, 0x10: 1 }).join()); }"##
);

#[test]
fn only_canonical_integer_keys_are_listed_first() {
    check(&decoded(
        ONLY_CANONICAL_INTEGER_KEYS_ARE_LISTED_FIRST,
        "s0|16,1000,b,1.5\ns1|16,1000,b,1.5\ns2|16,1000,b,1.5\n",
    ));
}

const OBJECT_ASSIGN_COPIES_INTEGER_KEYS_FIRST: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (Object.keys(Object.assign({ b: 1 }, { 1: 1 })).join()); }"##
);

#[test]
fn object_assign_copies_integer_keys_first() {
    check(&decoded(
        OBJECT_ASSIGN_COPIES_INTEGER_KEYS_FIRST,
        "s0|1,b\ns1|1,b\ns2|1,b\n",
    ));
}

const THE_LARGEST_UINT32_KEY_IS_NOT_AN_ARRAY_INDEX: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (Object.keys({ b: 1, 4294967295: 1, 4294967294: 1, a: 1, '01': 1, 0: 1 }).join()); }"##
);

#[test]
fn the_largest_uint32_key_is_not_an_array_index() {
    check(&decoded(THE_LARGEST_UINT32_KEY_IS_NOT_AN_ARRAY_INDEX, "s0|0,4294967294,b,4294967295,a,01\ns1|0,4294967294,b,4294967295,a,01\ns2|0,4294967294,b,4294967295,a,01\n"));
}

const CONSTRUCTING_A_METHOD_THROWS: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var o = { m(){ this.v = 1; } }; try { new o.m(); return 'constructed'; } catch (z) { return 'caught'; } })()); }"##
);

#[test]
fn constructing_a_method_throws() {
    check(&decoded(
        CONSTRUCTING_A_METHOD_THROWS,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const DEFINE_PROPERTY_ON_AN_ARRAY_LENGTH_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; try { Object.defineProperty(a, 'length', { value: 1, writable: true, enumerable: true, configurable: true }); } catch (z) { return 'caught'; } return a.join(); })()); }"##
);

#[test]
fn define_property_on_an_array_length_is_refused() {
    check(&q_refused(
        DEFINE_PROPERTY_ON_AN_ARRAY_LENGTH_IS_REFUSED,
        "s0|caught\ns1|caught\ns2|caught\n",
    ));
}

const ARRAY_INDEX_OF_HONOURS_A_FRACTIONAL_FROM_INDEX: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (String([1,2].indexOf(1, 1.5))); }"##
);

#[test]
fn array_index_of_honours_a_fractional_from_index() {
    check(&decoded(
        ARRAY_INDEX_OF_HONOURS_A_FRACTIONAL_FROM_INDEX,
        "s0|-1\ns1|-1\ns2|-1\n",
    ));
}

const SPLICE_WITHOUT_ARGUMENTS_REMOVES_NOTHING: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1,2,3]; var r = a.splice(); return r.length + ':' + a.join(); })()); }"##
);

#[test]
fn splice_without_arguments_removes_nothing() {
    check(&decoded(
        SPLICE_WITHOUT_ARGUMENTS_REMOVES_NOTHING,
        "s0|0:1,2,3\ns1|0:1,2,3\ns2|0:1,2,3\n",
    ));
}

const JSON_PARSE_WITH_A_REVIVER_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify(JSON.parse('[1,2]', function(k, v){ return v === 1 ? undefined : v; }))); }"##
);

#[test]
fn json_parse_with_a_reviver_is_refused() {
    check(&q_refused(
        JSON_PARSE_WITH_A_REVIVER_IS_REFUSED,
        "s0|[null,2]\ns1|[null,2]\ns2|[null,2]\n",
    ));
}

const JSON_PARSE_WITH_A_REVIVER_FUNCTION_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + (JSON.stringify(JSON.parse('{"a":1,"b":[1,2]}', function(k, v){ return typeof v === 'number' ? v * 2 : v; }))); }"##
);

#[test]
fn json_parse_with_a_reviver_function_is_refused() {
    check(&q_refused(
        JSON_PARSE_WITH_A_REVIVER_FUNCTION_IS_REFUSED,
        "s0|{\"a\":2,\"b\":[2,4]}\ns1|{\"a\":2,\"b\":[2,4]}\ns2|{\"a\":2,\"b\":[2,4]}\n",
    ));
}

const ARRAY_INDEX_OF_AND_INCLUDES_FOLLOW_FROM_INDEX: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ([[1,2,3].indexOf(2,-1), [1,2,3].indexOf(3,-1), [1,2,1].indexOf(1,1), [1,2].indexOf(1,-Infinity), [1,2].indexOf(1,Infinity), [1,2,3].includes(1,1), [NaN].includes(NaN,-5), [].indexOf(1, {valueOf: function(){ throw 1; }})].join()); }"##
);

#[test]
fn array_index_of_and_includes_follow_from_index() {
    check(&decoded(ARRAY_INDEX_OF_AND_INCLUDES_FOLLOW_FROM_INDEX, "s0|-1,2,2,0,-1,false,true,-1\ns1|-1,2,2,0,-1,false,true,-1\ns2|-1,2,2,0,-1,false,true,-1\n"));
}

const ARRAY_CALLBACKS_READ_THE_LIVE_ARRAY: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; var seen = []; a.forEach(function(v, k){ if (k === 0) a[2] = 9; seen.push(v); }); var b = [1, 2, 3]; var f = b.filter(function(v, k){ if (k === 0) b.pop(); return true; }); var c = [1, 2, 3]; var r = c.reduce(function(p, v){ c[2] = 5; return p + v; }); return seen.join('') + ',' + f.join('') + ',' + r; })()); }"##
);

#[test]
fn array_callbacks_read_the_live_array() {
    check(&decoded(
        ARRAY_CALLBACKS_READ_THE_LIVE_ARRAY,
        "s0|129,12,8\ns1|129,12,8\ns2|129,12,8\n",
    ));
}

const A_MAP_THAT_SHRINKS_ITS_ARRAY_IS_REFUSED: &str = qunit!(
    r##"function qb(i){ var e=qa(); return e[i] + '|' + ((function(){ var a = [1, 2, 3]; return a.map(function(v){ a.pop(); return v; }).length; })()); }"##
);

#[test]
fn a_map_that_shrinks_its_array_is_refused() {
    check(&q_refused(
        A_MAP_THAT_SHRINKS_ITS_ARRAY_IS_REFUSED,
        "s0|3\ns1|3\ns2|3\n",
    ));
}
