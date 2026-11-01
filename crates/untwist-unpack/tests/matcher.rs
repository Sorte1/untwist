use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use untwist_unpack::matcher::{
    anonymous_factory, call_parts, const_member, literal_id, named_const_member, param_names,
    single_param_name, unwrap_statement_expression,
};

fn first_expression<'a>(alloc: &'a Allocator, source: &'a str) -> Expression<'a> {
    let parsed = untwist_core::parse::parse_program(alloc, source);
    match parsed.program.body.into_iter().next() {
        Some(Statement::ExpressionStatement(stmt)) => stmt.unbox().expression,
        _ => {
            let directive = parsed
                .program
                .directives
                .into_iter()
                .next()
                .expect("expression statement");
            Expression::StringLiteral(oxc_allocator::Box::new_in(directive.expression, &alloc))
        }
    }
}

#[test]
fn const_member_accepts_both_forms() {
    let alloc = Allocator::default();
    let dot = first_expression(&alloc, "require.d;");
    let computed = first_expression(&alloc, "require['d'];");
    assert!(const_member(&dot, "d").is_some());
    assert!(const_member(&computed, "d").is_some());
    assert!(const_member(&dot, "r").is_none());
    assert!(named_const_member(&dot, "require", "d"));
    assert!(!named_const_member(&dot, "other", "d"));
}

#[test]
fn literal_ids_render_like_javascript() {
    let alloc = Allocator::default();
    assert_eq!(
        literal_id(&first_expression(&alloc, "386;")),
        Some("386".to_string())
    );
    assert_eq!(
        literal_id(&first_expression(&alloc, "'2wwy';")),
        Some("2wwy".to_string())
    );
    assert_eq!(literal_id(&first_expression(&alloc, "x;")), None);
}

#[test]
fn anonymous_factory_accepts_functions_and_block_arrows_only() {
    let alloc = Allocator::default();
    let function = first_expression(&alloc, "(function (a, b) { x(); });");
    let arrow = first_expression(&alloc, "((a, b) => { x(); });");
    let expression_arrow = first_expression(&alloc, "((a) => a);");
    let named = first_expression(&alloc, "(function f() {});");
    let generator = first_expression(&alloc, "(function* () {});");
    assert!(anonymous_factory(&function).is_some());
    assert!(anonymous_factory(&arrow).is_some());
    assert!(anonymous_factory(&expression_arrow).is_none());
    assert!(anonymous_factory(&named).is_none());
    assert!(anonymous_factory(&generator).is_none());
    let factory = anonymous_factory(&function).expect("factory");
    assert_eq!(param_names(factory.params), vec!["a", "b"]);
    assert_eq!(factory.body.statements.len(), 1);
}

#[test]
fn unwrap_statement_expression_peels_bang_but_keeps_a_sequence() {
    let alloc = Allocator::default();
    let bang = first_expression(&alloc, "!(function (m) {})([]);");
    let plain = first_expression(&alloc, "(function (m) {})([]);");
    let sequence = first_expression(&alloc, "f(), (function (m) {})([]);");
    let inner = first_expression(&alloc, "!(f(), (function (m) {})([]));");
    assert!(call_parts(unwrap_statement_expression(&bang)).is_some());
    assert!(call_parts(unwrap_statement_expression(&plain)).is_some());
    assert!(call_parts(unwrap_statement_expression(&sequence)).is_none());
    assert!(call_parts(unwrap_statement_expression(&inner)).is_none());
}

#[test]
fn single_param_name_requires_exactly_one_identifier() {
    let alloc = Allocator::default();
    let one = first_expression(&alloc, "(function (moduleId) {});");
    let two = first_expression(&alloc, "(function (a, b) {});");
    let one = anonymous_factory(&one).expect("factory");
    let two = anonymous_factory(&two).expect("factory");
    assert_eq!(single_param_name(one.params), Some("moduleId"));
    assert_eq!(single_param_name(two.params), None);
}
