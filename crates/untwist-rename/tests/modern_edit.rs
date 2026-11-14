use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_span::{GetSpan, Span};
use untwist_core::parse::parse_program;
use untwist_rename::modern::edit::reachable;
use untwist_rename::modern::regions::Regions;

#[test]
fn an_empty_span_cannot_be_placed() {
    let mut regions = Regions::default();
    assert!(regions.insert(Span::new(0, 0)).is_none());
    assert!(regions.has_unplaced());
    assert!(regions.is_empty());
}

fn expression_span(statement: &Statement<'_>) -> Span {
    match statement {
        Statement::ExpressionStatement(statement) => statement.expression.span(),
        _ => panic!("not an expression statement"),
    }
}

#[test]
fn a_region_that_matches_two_statements_is_not_reachable() {
    let alloc = Allocator::default();
    let parsed = parse_program(&alloc, "a();\nb();\n");
    let mut program = parsed.program;
    let first = expression_span(&program.body[0]);
    let mut regions = Regions::default();
    regions.insert(first);
    assert!(reachable(&program, &regions));
    if let Statement::ExpressionStatement(second) = &mut program.body[1] {
        if let oxc_ast::ast::Expression::CallExpression(call) = &mut second.expression {
            call.span = first;
        }
    }
    assert!(!reachable(&program, &regions));
}
