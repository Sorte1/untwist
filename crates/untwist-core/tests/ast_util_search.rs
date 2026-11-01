use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_span::{GetSpan, Span};
use untwist_core::ast_util::search::{expression_at, statement_at};
use untwist_core::parse::parse_program;

#[test]
fn an_expression_is_found_at_any_depth_by_its_span() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f() { return g(41 + 1); }");
    let mut wanted = Span::empty(0);
    let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
        panic!("function declaration");
    };
    let Some(body) = function.body.as_ref() else {
        panic!("body");
    };
    let Some(Statement::ReturnStatement(statement)) = body.statements.first() else {
        panic!("return");
    };
    let Some(Expression::CallExpression(call)) = statement.argument.as_ref() else {
        panic!("call");
    };
    if let Some(argument) = call.arguments.first().and_then(|a| a.as_expression()) {
        wanted = argument.span();
    }

    let found = expression_at(&parsed.program, wanted).expect("found");
    assert_eq!(found.span(), wanted);
    assert!(matches!(found, Expression::BinaryExpression(_)));
    assert!(expression_at(&parsed.program, Span::new(0, 1)).is_none());
}

#[test]
fn a_statement_is_found_at_any_depth_by_its_span() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "function f() { var x = 1; }");
    let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
        panic!("function declaration");
    };
    let Some(body) = function.body.as_ref() else {
        panic!("body");
    };
    let wanted = body.statements.first().expect("statement").span();

    let found = statement_at(&parsed.program, wanted).expect("found");
    assert_eq!(found.span(), wanted);
    assert!(matches!(found, Statement::VariableDeclaration(_)));
    assert!(statement_at(&parsed.program, Span::new(0, 1)).is_none());
}
