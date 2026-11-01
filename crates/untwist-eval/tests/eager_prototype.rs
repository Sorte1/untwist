mod common;

use common::eval;

#[test]
fn reading_prototype_on_a_function_yields_an_object_that_new_uses() {
    assert_eq!(
        eval("function F() {} F.prototype.x = 1; new F().x"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("var f = function () {}; typeof f.prototype"),
        Ok("string:object".to_string())
    );
    assert_eq!(
        eval("function F() {} F.prototype === F.prototype"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn prototype_is_present_for_in_and_absent_for_arrows() {
    assert_eq!(
        eval("function F() {} 'prototype' in F"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("var a = () => 1; typeof a.prototype"),
        Ok("string:undefined".to_string())
    );
}

#[test]
fn the_own_property_paths_see_a_prototype_that_does_not_enumerate() {
    assert_eq!(
        eval("function F() {} F.hasOwnProperty('prototype')"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; Object.keys(F).length"),
        Ok("number:1".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; Object.keys(F)[0]"),
        Ok("string:a".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; var n = 0; for (var k in F) { n = n + 1; } n"),
        Ok("number:1".to_string())
    );
}

#[test]
fn prototype_never_enumerates_however_late_it_is_first_observed() {
    assert_eq!(
        eval("function F() {} F.a = 1; var s = ''; for (var k in F) { s = s + k + '|'; } s"),
        Ok("string:a|".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; Object.entries(F)[0][0]"),
        Ok("string:a".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; Object.values(F)[0] === F.prototype"),
        Ok("boolean:false".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; Object.keys(Object.assign({}, F))[0]"),
        Ok("string:a".to_string())
    );
    assert_eq!(
        eval("function F() {} F.a = 1; F.prototype = 7; Object.keys(F)[0]"),
        Ok("string:a".to_string())
    );
}

#[test]
fn deleting_prototype_is_refused_so_it_stays_after_a_later_observation() {
    assert_eq!(
        eval("function F() {} delete F.prototype; 'prototype' in F"),
        Ok("boolean:true".to_string())
    );
    assert_eq!(
        eval("function F() {} F.prototype; delete F.prototype; typeof F.prototype"),
        Ok("string:object".to_string())
    );
    assert_eq!(
        eval("function F() {} F.prototype; delete F.prototype; Object.keys(F).length"),
        Ok("number:0".to_string())
    );
    assert_eq!(
        eval("function F() {} F.prototype; delete F.prototype; F.hasOwnProperty('prototype')"),
        Ok("boolean:true".to_string())
    );
}

#[test]
fn new_on_a_function_whose_prototype_is_a_primitive_leaves_it_as_node_does() {
    assert_eq!(
        eval("function F() {} F.prototype = 5; new F(); typeof F.prototype"),
        Ok("string:number".to_string())
    );
}
