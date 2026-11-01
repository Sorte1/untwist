mod common;

use oxc_ast::ast::{Expression, Statement};
use untwist_eval::source::{function_source, satisfies_self_defending_regex};
use untwist_eval::value::FnCode;

fn render(source: &str, source_text_for_lookup: bool) -> String {
    common::with_program(source, |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        let Expression::FunctionExpression(function) = &stmt.expression else {
            panic!("expected a function expression");
        };
        let text = if source_text_for_lookup {
            program.source_text
        } else {
            ""
        };
        function_source(text, FnCode::Function(function.as_ref()))
    })
}

#[test]
fn minified_input_round_trips_to_the_exact_source_slice() {
    let rendered = render("(function(){return'newState';});", true);
    assert_eq!(rendered, "function(){return'newState';}");
    assert!(satisfies_self_defending_regex(&rendered));
    assert!(!rendered.contains('\n'));
}

#[test]
fn pretty_input_keeps_its_newlines_so_the_outer_guard_short_circuits() {
    let rendered = render("(function () {\n  return 'newState';\n});", true);
    assert!(rendered.contains('\n'));
}

#[test]
fn a_synthesized_node_falls_back_to_newline_bearing_codegen() {
    let rendered = render("(function(){return'newState';});", false);
    assert!(rendered.contains('\n'));
    assert!(rendered.starts_with("function"));
}

#[test]
fn the_guard_regex_rejects_template_literal_rendering() {
    assert!(satisfies_self_defending_regex(
        "function(){return'newState';}"
    ));
    assert!(!satisfies_self_defending_regex(
        "function(){return`newState`}"
    ));
    assert!(!satisfies_self_defending_regex(
        "function () {\n  return 'newState';\n}"
    ));
}

#[test]
fn empty_span_falls_back_to_codegen() {
    common::with_program("(function(){});", |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        let Expression::FunctionExpression(function) = &stmt.expression else {
            panic!("expected a function expression");
        };
        let rendered = function_source("", FnCode::Function(function.as_ref()));
        assert!(rendered.starts_with("function"));
    });
}

#[test]
fn span_beyond_the_text_falls_back_to_codegen() {
    common::with_program("(function(){return'newState';});", |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        let Expression::FunctionExpression(function) = &stmt.expression else {
            panic!("expected a function expression");
        };
        let short_text = "(func";
        let rendered = function_source(short_text, FnCode::Function(function.as_ref()));
        assert!(rendered.contains('\n'));
        assert!(rendered.starts_with("function"));
    });
}

#[test]
fn arrow_function_renders_via_gen_expr_when_span_is_out_of_range() {
    common::with_program("(() => 'newState');", |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        let Expression::ArrowFunctionExpression(arrow) = &stmt.expression else {
            panic!("expected an arrow function expression");
        };
        let rendered = function_source("", FnCode::Arrow(arrow.as_ref()));
        assert!(rendered.contains("=>"));
    });
}

#[test]
fn arrow_function_source_slice_round_trips_exactly() {
    let rendered = common::with_program("(()=>'newState');", |program| {
        let Some(Statement::ExpressionStatement(stmt)) = program.body.first() else {
            panic!("expected an expression statement");
        };
        let Expression::ArrowFunctionExpression(arrow) = &stmt.expression else {
            panic!("expected an arrow function expression");
        };
        function_source(program.source_text, FnCode::Arrow(arrow.as_ref()))
    });
    assert_eq!(rendered, "()=>'newState'");
}
