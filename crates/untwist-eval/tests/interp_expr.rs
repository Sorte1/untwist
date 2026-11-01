mod common;

use common::eval;

#[test]
fn numbers_to_expressions_folding_works_end_to_end() {
    assert_eq!(
        eval("-0x1021e + -0x7eac8 + 0x17 * 0xac9c"),
        Ok("number:431390".to_string())
    );
    assert_eq!(
        eval("-0x1 * -0x133e + -0x2b * 0x13 + -0x100c"),
        Ok("number:1".to_string())
    );
}

#[test]
fn split_strings_concatenation_works_end_to_end() {
    assert_eq!(
        eval("'He' + 'll' + 'o ' + 'Wo' + 'rl' + 'd!'"),
        Ok("string:Hello World!".to_string())
    );
}

#[test]
fn logical_operators_short_circuit() {
    assert_eq!(eval("0 || 'fallback'"), Ok("string:fallback".to_string()));
    assert_eq!(eval("'' && missingGlobal"), Ok("string:".to_string()));
    assert_eq!(eval("null ?? 3"), Ok("number:3".to_string()));
    assert_eq!(
        eval("false ? missingGlobal : 7"),
        Ok("number:7".to_string())
    );
}

#[test]
fn sequence_expressions_yield_their_last_operand() {
    assert_eq!(
        eval("var a = 1; (a = 2, a + 1)"),
        Ok("number:3".to_string())
    );
}

#[test]
fn template_literals_stringify_their_parts() {
    assert_eq!(
        eval("var n = 255; `x${n}y`"),
        Ok("string:x255y".to_string())
    );
}

#[test]
fn array_and_object_literals_build_objects() {
    assert_eq!(eval("['log', void 0]"), Ok("array:2".to_string()));
    assert_eq!(eval("({ c: 0x2f2, d: '0x396' })"), Ok("object".to_string()));
    assert_eq!(eval("[1, , 3]"), Ok("array:3".to_string()));
}

#[test]
fn function_expressions_become_closures() {
    assert_eq!(
        eval("(function (a, b) { return a; })"),
        Ok("function".to_string())
    );
    assert_eq!(eval("(() => 1)"), Ok("function".to_string()));
}

#[test]
fn spread_in_a_literal_bails_rather_than_guessing() {
    assert_eq!(
        eval("[...[1, 2]]"),
        Err("unsupported construct: spread element".to_string())
    );
}

#[test]
fn this_outside_a_function_bails() {
    assert_eq!(
        eval("this"),
        Err("unsupported construct: sloppy-mode this bound to a primitive or nothing".to_string())
    );
}

#[test]
fn object_literal_shorthand_and_computed_keys_use_declared_bindings() {
    assert_eq!(
        eval("var a = 5; var o = { a }; o.a"),
        Ok("number:5".to_string())
    );
    assert_eq!(
        eval("var k = 'x'; var o = { [k]: 1 }; o.x"),
        Ok("number:1".to_string())
    );
}
