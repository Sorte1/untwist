mod common;

use common::eval;
use oxc_ast::ast::{Expression, Statement};
use untwist_eval::budget::Budget;
use untwist_eval::interp::Interp;
use untwist_eval::value::js_string_from_literal;

#[test]
fn literals_evaluate_to_themselves() {
    assert_eq!(eval("0x1cc"), Ok("number:460".to_string()));
    assert_eq!(
        eval("'Hello\\x20World!'"),
        Ok("string:Hello World!".to_string())
    );
    assert_eq!(eval("null"), Ok("null".to_string()));
    assert_eq!(eval("void 0"), Ok("undefined".to_string()));
}

#[test]
fn object_literals_are_truthy() {
    assert_eq!(eval("!![]"), Ok("boolean:true".to_string()));
}

#[test]
fn a_lone_surrogate_literal_survives() {
    assert_eq!(eval("'\\uD800'.length"), Ok("number:1".to_string()));
}

#[test]
fn var_declarations_bind_and_read_back() {
    assert_eq!(eval("var a = 5; a"), Ok("number:5".to_string()));
    assert_eq!(eval("var a; a"), Ok("undefined".to_string()));
}

#[test]
fn an_unresolved_global_bails_and_never_reads_undefined() {
    assert_eq!(
        eval("someUnknownGlobal"),
        Err("unresolved global binding: someUnknownGlobal".to_string())
    );
}

#[test]
fn typeof_an_unresolved_global_is_undefined_not_a_bail() {
    assert_eq!(
        eval("typeof someUnknownGlobal"),
        Ok("string:undefined".to_string())
    );
}

#[test]
fn negative_zero_survives_evaluation() {
    assert_eq!(eval("-0"), Ok("number:-0".to_string()));
}

#[test]
fn js_string_from_literal_round_trips_a_plain_string() {
    common::with_program("0; 'hello';", |program| {
        let Statement::ExpressionStatement(stmt) = &program.body[1] else {
            panic!("expected an expression statement");
        };
        let Expression::StringLiteral(literal) = &stmt.expression else {
            panic!("expected a string literal");
        };
        assert!(!literal.value.has_lone_surrogate());
        let js = js_string_from_literal(literal);
        assert_eq!(js.to_lossy_string(), "hello");
    });
}

#[test]
fn js_string_from_literal_decodes_lone_surrogates() {
    common::with_program("0; '\\uD800';", |program| {
        let Statement::ExpressionStatement(stmt) = &program.body[1] else {
            panic!("expected an expression statement");
        };
        let Expression::StringLiteral(literal) = &stmt.expression else {
            panic!("expected a string literal");
        };
        assert!(literal.value.has_lone_surrogate());
        let js = js_string_from_literal(literal);
        assert_eq!(js.len(), 1);
        assert_eq!(js.code_unit_at(0), Some(0xD800));
    });
}

#[test]
fn interp_new_stores_source_text_and_starts_with_no_notes() {
    let interp = Interp::new("var a = 1;", Budget::default());
    assert_eq!(interp.source_text, "var a = 1;");
    assert!(interp.notes.is_empty());
    assert_eq!(interp.budget.steps_used(), 0);
}
