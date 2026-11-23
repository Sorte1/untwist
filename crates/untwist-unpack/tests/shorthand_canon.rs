mod common;

use common::parse;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use untwist_unpack::webpack::canon::canonical_statement;
use untwist_unpack::webpack::machinery::form_text;

fn canonical(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(source);
    let program = parse(&alloc, source);
    canonical_statement(&program.body[0], "r")
}

fn form(source: &str) -> Option<String> {
    let alloc = Allocator::default();
    let source = alloc.alloc_str(source);
    let program = parse(&alloc, source);
    let Statement::ExpressionStatement(statement) = &program.body[0] else {
        return None;
    };
    form_text(&statement.expression, "r")
}

#[test]
fn canonical_statements_that_destructure_different_keys_differ() {
    let a = canonical("function f(o) { var a; ({a} = o); return a; }");
    let b = canonical("function f(o) { var b; ({b} = o); return b; }");
    assert!(a.is_some());
    assert_ne!(a, b);
}

#[test]
fn machinery_forms_that_destructure_different_keys_differ() {
    let a = form("(function (o) { var a; ({a} = o); return a; })");
    let b = form("(function (o) { var b; ({b} = o); return b; })");
    assert!(a.is_some());
    assert_ne!(a, b);
}
