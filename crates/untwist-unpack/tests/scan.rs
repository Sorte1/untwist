mod common;

use common::parse;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, IdentifierReference, Program, Statement};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use oxc_str::Ident;
use untwist_core::semantic::build_scoping;
use untwist_unpack::scan::is_free;

fn last_reference<'k, 'a>(program: &'k Program<'a>) -> &'k IdentifierReference<'a> {
    let statement = match program.body.last() {
        Some(Statement::FunctionDeclaration(function)) => function
            .body
            .as_ref()
            .and_then(|body| body.statements.last()),
        other => other,
    };
    let Some(Statement::ExpressionStatement(statement)) = statement else {
        panic!("the last statement is not an expression statement");
    };
    let Expression::Identifier(id) = &statement.expression else {
        panic!("the last statement is not a bare reference");
    };
    id
}

fn free(source: &str) -> bool {
    let alloc = Allocator::default();
    let program = parse(&alloc, alloc.alloc_str(source));
    let scoping = build_scoping(&program).scoping;
    is_free(Some(&scoping), last_reference(&program))
}

#[test]
fn a_name_nothing_binds_is_free() {
    assert!(free("require;\n"));
    assert!(free("function f() {\n  require;\n}\n"));
}

#[test]
fn a_name_bound_by_a_local_or_a_parameter_is_not_free() {
    assert!(!free("var require = 1;\nrequire;\n"));
    assert!(!free("function f(require) {\n  require;\n}\n"));
    assert!(!free(
        "function f() {\n  function require() {}\n  require;\n}\n"
    ));
}

#[test]
fn without_a_scoping_every_reference_is_free() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "var require = 1;\nrequire;\n");
    assert!(is_free(None, last_reference(&program)));
}

#[test]
fn a_reference_made_after_the_scoping_was_built_is_free() {
    let alloc = Allocator::default();
    let program = parse(&alloc, "var require = 1;\nrequire;\n");
    let scoping = build_scoping(&program).scoping;
    let builder = AstBuilder::new(&alloc);
    let made = Expression::new_identifier(SPAN, Ident::from_str_in("require", &builder), &builder);
    let Expression::Identifier(made) = &made else {
        panic!("an identifier");
    };
    assert!(is_free(Some(&scoping), made));
}
